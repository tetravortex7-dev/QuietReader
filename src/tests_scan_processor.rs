use crate::app::scan_processor::{apply_smart_dark_mode, detect_content_bbox};
use crate::document::DecodedImage;
use eframe::egui::Color32;

#[test]
fn test_detect_content_bbox_with_white_margins() {
    let w = 200;
    let h = 300;
    let mut rgba = vec![255u8; w * h * 4];

    for y in 60..240 {
        for x in 40..160 {
            let idx = (y * w + x) * 4;
            rgba[idx] = 10;
            rgba[idx + 1] = 10;
            rgba[idx + 2] = 10;
        }
    }

    let img = DecodedImage {
        width: w as u32,
        height: h as u32,
        rgba,
    };

    let bbox = detect_content_bbox(&img, 24);

    assert!(bbox.min.x > 0.05);
    assert!(bbox.max.x < 0.95);
    assert!(bbox.min.y > 0.05);
    assert!(bbox.max.y < 0.95);
    assert!(bbox.width() < 0.85);
    assert!(bbox.height() < 0.85);
}

#[test]
fn test_detect_content_bbox_uniform_returns_full() {
    let w = 100;
    let h = 100;
    let rgba = vec![250u8; w * h * 4];

    let img = DecodedImage {
        width: w as u32,
        height: h as u32,
        rgba,
    };

    let bbox = detect_content_bbox(&img, 24);
    assert_eq!(bbox.min.x, 0.0);
    assert_eq!(bbox.min.y, 0.0);
    assert_eq!(bbox.max.x, 1.0);
    assert_eq!(bbox.max.y, 1.0);
}

#[test]
fn test_apply_smart_dark_mode_black_and_white() {
    let mut img = DecodedImage {
        width: 2,
        height: 2,
        rgba: vec![
            255, 255, 255, 255, 0, 0, 0, 255, 240, 240, 240, 255, 20, 20, 20, 255,
        ],
    };

    let bg = Color32::from_rgb(18, 18, 18);
    let text = Color32::from_rgb(220, 220, 220);

    apply_smart_dark_mode(&mut img, bg, text, true);

    let paper_r = img.rgba[0];
    let paper_g = img.rgba[1];
    let paper_b = img.rgba[2];
    assert!(paper_r < 40 && paper_g < 40 && paper_b < 40);

    let ink_r = img.rgba[4];
    let ink_g = img.rgba[5];
    let ink_b = img.rgba[6];
    assert!(ink_r > 190 && ink_g > 190 && ink_b > 190);
}

#[test]
fn test_apply_smart_dark_mode_preserves_color_art() {
    let mut img = DecodedImage {
        width: 1,
        height: 1,
        rgba: vec![230, 40, 40, 255],
    };

    let bg = Color32::from_rgb(20, 20, 20);
    let text = Color32::from_rgb(240, 240, 240);

    apply_smart_dark_mode(&mut img, bg, text, true);

    assert!(img.rgba[0] > 200);
    assert!(img.rgba[1] < 70);
    assert!(img.rgba[2] < 70);
}

#[test]
fn test_apply_smart_dark_mode_sepia_theme() {
    let mut img = DecodedImage {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };

    let bg = Color32::from_rgb(244, 236, 216);
    let text = Color32::from_rgb(60, 45, 30);

    apply_smart_dark_mode(&mut img, bg, text, false);

    assert!((img.rgba[0] as i32 - 244).abs() < 10);
    assert!((img.rgba[1] as i32 - 236).abs() < 10);
    assert!((img.rgba[2] as i32 - 216).abs() < 10);
}
