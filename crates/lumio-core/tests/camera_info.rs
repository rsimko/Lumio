use std::path::{Path, PathBuf};

use lumio_core::camera::read_camera_info;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/camera.jpg")
}

#[test]
fn reads_camera_and_exposure_fields() {
    let info = read_camera_info(&fixture(), None);

    assert_eq!(info.make.as_deref(), Some("Lumio Camera Co."));
    assert_eq!(info.model.as_deref(), Some("Lumio Test X"));
    assert_eq!(info.lens.as_deref(), Some("Lumio 35-70mm f/2.8"));
    assert!((info.shutter_speed.unwrap() - 1.0 / 250.0).abs() < 1e-9);
    assert!((info.aperture.unwrap() - 2.8).abs() < 1e-6);
    assert_eq!(info.iso, Some(400));
    assert!((info.focal_length.unwrap() - 35.0).abs() < 1e-6);
}

#[test]
fn companion_jpeg_fills_missing_camera_fields() {
    let info = read_camera_info(Path::new("/nonexistent/photo.cr3"), Some(&fixture()));

    assert_eq!(info.model.as_deref(), Some("Lumio Test X"));
    assert_eq!(info.iso, Some(400));
    assert!(!info.is_empty());
}

#[test]
fn missing_files_return_empty_information() {
    assert!(read_camera_info(Path::new("/nonexistent/photo.cr3"), None).is_empty());
}
