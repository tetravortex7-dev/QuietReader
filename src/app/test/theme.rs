#[test]
fn test_twilight_evening_theme() {
    use crate::app::theme::ReaderTheme;
    use eframe::egui::Color32;

    assert!(ReaderTheme::Twilight.is_dark());
    assert_eq!(
        ReaderTheme::Twilight.bg_color(),
        Color32::from_rgb(54, 47, 43)
    );
    assert_eq!(
        ReaderTheme::Twilight.text_color(),
        Color32::from_rgb(222, 210, 192)
    );
    assert_eq!(
        ReaderTheme::Twilight.panel_bg_color(),
        Color32::from_rgb(66, 58, 53)
    );
    assert_eq!(
        ReaderTheme::Twilight.accent_color(),
        Color32::from_rgb(224, 150, 68)
    );
}

#[test]
fn test_light_themes_button_borders() {
    use crate::app::theme::ReaderTheme;

    let light_themes = [
        ReaderTheme::Light,
        ReaderTheme::Sepia,
        ReaderTheme::Warm,
        ReaderTheme::Mint,
    ];

    for theme in light_themes {
        assert!(!theme.is_dark());
        assert!(theme.button_stroke_color().is_none());

        let visuals = theme.create_visuals();
        assert_eq!(visuals.widgets.inactive.bg_stroke.width, 0.0);
        assert_eq!(visuals.widgets.inactive.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.widgets.hovered.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.widgets.active.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.override_text_color, Some(theme.text_color()));
    }

    let dark_themes = [ReaderTheme::Dark, ReaderTheme::Oled, ReaderTheme::Twilight];

    for theme in dark_themes {
        assert!(theme.is_dark());
        assert!(theme.button_stroke_color().is_none());
    }
}

#[test]
fn test_new_themes_colors_and_oled() {
    use crate::app::theme::ReaderTheme;
    use eframe::egui::Color32;

    assert_eq!(ReaderTheme::Oled.bg_color(), Color32::from_rgb(0, 0, 0));
    assert!(ReaderTheme::Oled.is_dark());
    assert!(ReaderTheme::Dark.is_dark());
    assert!(!ReaderTheme::Warm.is_dark());
    assert!(!ReaderTheme::Mint.is_dark());
}
