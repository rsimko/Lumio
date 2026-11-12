use std::path::Path;

use image::DynamicImage;
use rawler::cfa::CFAColor;
use rawler::{RawImage, RawImageData};
use serde::{Deserialize, Serialize};

use crate::error::{LumioError, Result};

/// Standard 8-bit histogram resolution.
pub const HISTOGRAM_BINS: usize = 256;

/// Per-channel and luminance distribution of a preview image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Histogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub luma: Vec<u32>,
    /// Number of pixels counted, excluding any scaled-down preview edge.
    pub pixels: u64,
    /// True when the histogram was built from RAW sensor samples.
    pub raw: bool,
}

/// Counts every channel and a Rec. 709 luminance value for each pixel.
pub fn compute_histogram(image: &DynamicImage) -> Histogram {
    let rgb = image.to_rgb8();
    let mut red = vec![0u32; HISTOGRAM_BINS];
    let mut green = vec![0u32; HISTOGRAM_BINS];
    let mut blue = vec![0u32; HISTOGRAM_BINS];
    let mut luma = vec![0u32; HISTOGRAM_BINS];
    let mut pixels = 0u64;

    for pixel in rgb.pixels() {
        let [r, g, b] = pixel.0;
        red[r as usize] += 1;
        green[g as usize] += 1;
        blue[b as usize] += 1;
        // Integer Rec. 709 weights avoid floating point rounding drift while
        // staying within a u32 for every 8-bit channel value.
        let luminance = (2126 * r as u32 + 7152 * g as u32 + 722 * b as u32 + 5000) / 10_000;
        luma[luminance as usize] += 1;
        pixels += 1;
    }

    Histogram {
        red,
        green,
        blue,
        luma,
        pixels,
        raw: false,
    }
}

/// Builds a histogram from a fully decoded RAW image.
///
/// CFA samples are separated by colour and normalised against the camera
/// white level, so the result reflects sensor clipping rather than the
/// camera's rendered preview. Floating-point DNG samples are already
/// normalised to 0..1 by rawler and use a unit white level.
pub fn compute_raw_histogram(image: &RawImage) -> Histogram {
    let cfa = match &image.photometric {
        rawler::rawimage::RawPhotometricInterpretation::Cfa(config) => Some(&config.cfa),
        _ => Some(&image.camera.cfa),
    }
    .filter(|cfa| cfa.width > 0 && cfa.height > 0);

    // Skip masked sensor borders; they would otherwise create a large black
    // spike that is not part of the actual photo.
    let area = image.crop_area.or(image.active_area);
    let (x0, y0, width, height) = match area {
        Some(rect) if rect.d.w > 0 && rect.d.h > 0 => (rect.p.x, rect.p.y, rect.d.w, rect.d.h),
        _ => (0, 0, image.width, image.height),
    };

    match &image.data {
        RawImageData::Integer(data) => {
            let white = image
                .whitelevel
                .0
                .first()
                .copied()
                .filter(|level| *level > 0)
                .map(f64::from)
                .unwrap_or(f64::from(u16::MAX));
            histogram_from_samples(
                data.len(),
                image.width,
                x0,
                y0,
                width,
                height,
                image.cpp,
                white,
                cfa,
                |index| f64::from(data[index]),
            )
        }
        RawImageData::Float(data) => histogram_from_samples(
            data.len(),
            image.width,
            x0,
            y0,
            width,
            height,
            image.cpp,
            1.0,
            cfa,
            |index| f64::from(data[index]),
        ),
    }
}

/// Decodes a RAW file and builds a histogram from its sensor samples.
pub fn compute_raw_histogram_file(path: &Path) -> Result<Histogram> {
    let raw = rawler::decode_file(path)
        .map_err(|error| LumioError::Preview(format!("{}: {error}", path.display())))?;
    Ok(compute_raw_histogram(&raw))
}

#[allow(clippy::too_many_arguments)]
fn histogram_from_samples<F>(
    sample_count: usize,
    raw_width: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    cpp: usize,
    white: f64,
    cfa: Option<&rawler::CFA>,
    sample: F,
) -> Histogram
where
    F: Fn(usize) -> f64,
{
    let cpp = cpp.max(1);
    let raw_width = raw_width.max(1);
    let mut red = vec![0u32; HISTOGRAM_BINS];
    let mut green = vec![0u32; HISTOGRAM_BINS];
    let mut blue = vec![0u32; HISTOGRAM_BINS];
    let mut luma = vec![0u32; HISTOGRAM_BINS];
    let mut counted = 0u64;

    for row in y0..y0.saturating_add(height) {
        for col in x0..x0.saturating_add(width) {
            let base = row
                .saturating_mul(raw_width)
                .saturating_add(col)
                .saturating_mul(cpp);
            if cpp >= 3 {
                if base + 2 >= sample_count {
                    continue;
                }
                let r = normalise(sample(base), white);
                let g = normalise(sample(base + 1), white);
                let b = normalise(sample(base + 2), white);
                red[bin(r)] += 1;
                green[bin(g)] += 1;
                blue[bin(b)] += 1;
                luma[bin(0.2126 * r + 0.7152 * g + 0.0722 * b)] += 1;
            } else {
                if base >= sample_count {
                    continue;
                }
                let value = normalise(sample(base), white);
                let channel = bin(value);
                match cfa.map(|cfa| cfa.cfa_color_at(row, col)) {
                    Some(CFAColor::RED) => red[channel] += 1,
                    Some(CFAColor::GREEN) | Some(CFAColor::FUJI_GREEN) => green[channel] += 1,
                    Some(CFAColor::BLUE) => blue[channel] += 1,
                    Some(_) => {}
                    None => {
                        red[channel] += 1;
                        green[channel] += 1;
                        blue[channel] += 1;
                    }
                }
                luma[channel] += 1;
            }
            counted += 1;
        }
    }

    Histogram {
        red,
        green,
        blue,
        luma,
        pixels: counted,
        raw: true,
    }
}

fn normalise(value: f64, white: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    (value / white.max(f64::MIN_POSITIVE)).clamp(0.0, 1.0)
}

fn bin(value: f64) -> usize {
    ((value.clamp(0.0, 1.0) * (HISTOGRAM_BINS - 1) as f64).round() as usize).min(HISTOGRAM_BINS - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn counts_each_channel_and_luma_bin() {
        let mut image = RgbImage::new(3, 1);
        image.put_pixel(0, 0, Rgb([0, 0, 0]));
        image.put_pixel(1, 0, Rgb([255, 255, 255]));
        image.put_pixel(2, 0, Rgb([255, 0, 0]));

        let histogram = compute_histogram(&DynamicImage::ImageRgb8(image));
        assert_eq!(histogram.pixels, 3);
        assert!(!histogram.raw);
        assert_eq!(histogram.red[0], 1);
        assert_eq!(histogram.red[255], 2);
        assert_eq!(histogram.green[0], 2);
        assert_eq!(histogram.green[255], 1);
        assert_eq!(histogram.blue[0], 2);
        assert_eq!(histogram.blue[255], 1);
        assert_eq!(histogram.red.len(), HISTOGRAM_BINS);
        assert_eq!(histogram.luma.iter().sum::<u32>(), 3);
        // Rec. 709 red is ~21.26% of full scale.
        assert_eq!(histogram.luma[54], 1);
    }

    #[test]
    fn empty_images_produce_balanced_bins() {
        let histogram = compute_histogram(&DynamicImage::ImageRgb8(RgbImage::new(0, 0)));
        assert_eq!(histogram.pixels, 0);
        assert!(histogram.red.iter().all(|count| *count == 0));
        assert!(histogram.luma.iter().all(|count| *count == 0));
    }

    #[test]
    fn raw_samples_are_split_by_cfa_colour() {
        let cfa = rawler::CFA::new("RGGB");
        let data = [0u16, 100, 200, 255];
        let histogram = histogram_from_samples(4, 2, 0, 0, 2, 2, 1, 255.0, Some(&cfa), |index| {
            f64::from(data[index])
        });

        assert!(histogram.raw);
        assert_eq!(histogram.pixels, 4);
        assert_eq!(histogram.red[0], 1);
        assert_eq!(histogram.green[100], 1);
        assert_eq!(histogram.green[200], 1);
        assert_eq!(histogram.blue[255], 1);
        assert_eq!(histogram.luma.iter().sum::<u32>(), 4);
    }

    #[test]
    fn raw_samples_are_normalised_against_the_white_level() {
        let cfa = rawler::CFA::new("RGGB");
        let data = [0u16, 8192, 16_383];
        let histogram =
            histogram_from_samples(3, 3, 0, 0, 3, 1, 1, 16_383.0, Some(&cfa), |index| {
                f64::from(data[index])
            });

        assert_eq!(histogram.red[0], 1);
        assert_eq!(histogram.green[128], 1);
        assert_eq!(histogram.red[255], 1);
    }

    #[test]
    fn raw_samples_handle_interleaved_rgb() {
        let data = [255u16, 0, 0, 0, 255, 0];
        let histogram = histogram_from_samples(6, 2, 0, 0, 2, 1, 3, 255.0, None, |index| {
            f64::from(data[index])
        });

        assert!(histogram.raw);
        assert_eq!(histogram.pixels, 2);
        assert_eq!(histogram.red[255], 1);
        assert_eq!(histogram.green[255], 1);
        assert_eq!(histogram.blue[0], 2);
        assert_eq!(histogram.luma[54], 1);
        assert_eq!(histogram.luma[182], 1);
    }

    #[test]
    fn raw_samples_without_a_cfa_are_counted_as_greyscale() {
        let data = [64u16, 192];
        let histogram = histogram_from_samples(2, 2, 0, 0, 2, 1, 1, 255.0, None, |index| {
            f64::from(data[index])
        });

        assert_eq!(histogram.pixels, 2);
        assert_eq!(histogram.red[64], 1);
        assert_eq!(histogram.green[64], 1);
        assert_eq!(histogram.blue[64], 1);
        assert_eq!(histogram.red[192], 1);
        assert_eq!(histogram.green[192], 1);
        assert_eq!(histogram.blue[192], 1);
    }

    #[test]
    fn raw_samples_respect_the_crop_area() {
        let cfa = rawler::CFA::new("RGGB");
        let data = [0u16, 10, 20, 255];
        let histogram = histogram_from_samples(4, 2, 1, 1, 1, 1, 1, 255.0, Some(&cfa), |index| {
            f64::from(data[index])
        });

        assert_eq!(histogram.pixels, 1);
        assert_eq!(histogram.red.iter().sum::<u32>(), 0);
        assert_eq!(histogram.green.iter().sum::<u32>(), 0);
        assert_eq!(histogram.blue[255], 1);
    }
}
