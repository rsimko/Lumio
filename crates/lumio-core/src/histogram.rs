use image::DynamicImage;
use serde::{Deserialize, Serialize};

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
    }
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
}
