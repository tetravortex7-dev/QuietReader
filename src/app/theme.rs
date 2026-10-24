use crate::core::BookmarkColor;
use crate::core::library::UiDesignMode;
use eframe::egui::Color32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReaderTheme {
    Dark,
    Light,
    Sepia,
    Oled,
    Warm,
    Mint,
    Twilight,
}

impl ReaderTheme {
    pub fn is_dark(&self) -> bool {
        matches!(
            self,
            ReaderTheme::Dark | ReaderTheme::Oled | ReaderTheme::Twilight
        )
    }

    pub fn bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(26, 26, 28),
            ReaderTheme::Light => Color32::from_rgb(248, 249, 250),
            ReaderTheme::Sepia => Color32::from_rgb(244, 237, 222),
            ReaderTheme::Oled => Color32::from_rgb(0, 0, 0),
            ReaderTheme::Warm => Color32::from_rgb(250, 240, 220),
            ReaderTheme::Mint => Color32::from_rgb(238, 245, 238),
            ReaderTheme::Twilight => Color32::from_rgb(54, 47, 43),
        }
    }

    pub fn text_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(220, 220, 220),
            ReaderTheme::Light => Color32::from_rgb(33, 37, 41),
            ReaderTheme::Sepia => Color32::from_rgb(60, 48, 36),
            ReaderTheme::Oled => Color32::from_rgb(205, 205, 205),
            ReaderTheme::Warm => Color32::from_rgb(70, 50, 30),
            ReaderTheme::Mint => Color32::from_rgb(38, 55, 42),
            ReaderTheme::Twilight => Color32::from_rgb(222, 210, 192),
        }
    }

    pub fn secondary_text_color(&self) -> Color32 {
        self.text_color().gamma_multiply(0.65)
    }

    pub fn panel_bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(35, 35, 38),
            ReaderTheme::Light => Color32::from_rgb(238, 240, 244),
            ReaderTheme::Sepia => Color32::from_rgb(235, 227, 210),
            ReaderTheme::Oled => Color32::from_rgb(10, 10, 12),
            ReaderTheme::Warm => Color32::from_rgb(242, 230, 208),
            ReaderTheme::Mint => Color32::from_rgb(226, 236, 226),
            ReaderTheme::Twilight => Color32::from_rgb(66, 58, 53),
        }
    }

    pub fn accent_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(100, 160, 255),
            ReaderTheme::Light => Color32::from_rgb(0, 102, 204),
            ReaderTheme::Sepia => Color32::from_rgb(140, 90, 40),
            ReaderTheme::Oled => Color32::from_rgb(85, 145, 255),
            ReaderTheme::Warm => Color32::from_rgb(180, 95, 25),
            ReaderTheme::Mint => Color32::from_rgb(45, 125, 80),
            ReaderTheme::Twilight => Color32::from_rgb(224, 150, 68),
        }
    }

    pub fn button_stroke_color(&self) -> Option<Color32> {
        None
    }

    pub fn create_visuals(&self) -> eframe::egui::Visuals {
        self.create_visuals_for(UiDesignMode::Cozy)
    }

    pub fn create_visuals_for(&self, design: UiDesignMode) -> eframe::egui::Visuals {
        use eframe::egui;
        let mut visuals = if self.is_dark() {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.panel_bg_color();
        visuals.window_fill = self.panel_bg_color();
        visuals.window_stroke = egui::Stroke::new(1.0, self.accent_color().gamma_multiply(0.4));
        visuals.override_text_color = Some(self.text_color());

        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
        visuals.selection.stroke = egui::Stroke::new(1.0, self.accent_color());

        match self {
            ReaderTheme::Oled => {
                visuals.extreme_bg_color = Color32::from_rgb(0, 0, 0);
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(10, 10, 12);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(30, 30, 35));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(14, 14, 18);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(14, 14, 18);
            }
            ReaderTheme::Sepia => {
                visuals.widgets.hovered.fg_stroke.color = self.text_color();
                visuals.widgets.active.fg_stroke.color = self.text_color();
                visuals.widgets.open.fg_stroke.color = self.text_color();
                visuals.widgets.inactive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(228, 220, 203);
                visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(228, 220, 203);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(228, 220, 203);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(220, 210, 192);
                visuals.widgets.active.bg_fill = Color32::from_rgb(212, 200, 180);
            }
            ReaderTheme::Warm => {
                visuals.widgets.hovered.fg_stroke.color = self.text_color();
                visuals.widgets.active.fg_stroke.color = self.text_color();
                visuals.widgets.open.fg_stroke.color = self.text_color();
                visuals.widgets.inactive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(236, 224, 200);
                visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(236, 224, 200);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(236, 224, 200);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(228, 214, 188);
                visuals.widgets.active.bg_fill = Color32::from_rgb(220, 204, 176);
            }
            ReaderTheme::Mint => {
                visuals.widgets.hovered.fg_stroke.color = self.text_color();
                visuals.widgets.active.fg_stroke.color = self.text_color();
                visuals.widgets.open.fg_stroke.color = self.text_color();
                visuals.widgets.inactive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(220, 230, 220);
                visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(220, 230, 220);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(220, 230, 220);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(210, 222, 210);
                visuals.widgets.active.bg_fill = Color32::from_rgb(200, 214, 200);
            }
            ReaderTheme::Twilight => {
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(250, 240, 225);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(255, 255, 255);
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(46, 40, 36);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(80, 70, 64));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(46, 40, 36);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(46, 40, 36);
            }
            ReaderTheme::Light => {
                visuals.widgets.inactive.fg_stroke.color = self.text_color();
                visuals.widgets.hovered.fg_stroke.color = self.text_color();
                visuals.widgets.active.fg_stroke.color = self.text_color();
                visuals.widgets.open.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.fg_stroke.color = self.text_color();
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(244, 245, 248);
                visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(238, 240, 244);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(238, 240, 244);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(226, 230, 236);
                visuals.widgets.active.bg_fill = Color32::from_rgb(216, 222, 230);
            }
            ReaderTheme::Dark => {}
        }

        let radius = egui::CornerRadius::same(design.corner_radius());
        visuals.widgets.noninteractive.corner_radius = radius;
        visuals.widgets.inactive.corner_radius = radius;
        visuals.widgets.hovered.corner_radius = radius;
        visuals.widgets.active.corner_radius = radius;
        visuals.widgets.open.corner_radius = radius;
        visuals.window_corner_radius = radius;
        visuals.menu_corner_radius = radius;
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, self.accent_color().gamma_multiply(0.55));
        visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, self.accent_color());
        visuals
    }
}

pub fn bookmark_color(color: BookmarkColor) -> Color32 {
    match color {
        BookmarkColor::Yellow => Color32::from_rgb(235, 195, 60),
        BookmarkColor::Green => Color32::from_rgb(72, 187, 120),
        BookmarkColor::Blue => Color32::from_rgb(66, 153, 225),
        BookmarkColor::Red => Color32::from_rgb(245, 101, 101),
    }
}

pub fn bookmark_highlight_color(color: BookmarkColor, is_dark: bool) -> Color32 {
    if is_dark {
        match color {
            BookmarkColor::Yellow => Color32::from_rgba_unmultiplied(235, 195, 60, 60),
            BookmarkColor::Green => Color32::from_rgba_unmultiplied(72, 187, 120, 60),
            BookmarkColor::Blue => Color32::from_rgba_unmultiplied(66, 153, 225, 60),
            BookmarkColor::Red => Color32::from_rgba_unmultiplied(245, 101, 101, 60),
        }
    } else {
        match color {
            BookmarkColor::Yellow => Color32::from_rgba_unmultiplied(255, 235, 120, 95),
            BookmarkColor::Green => Color32::from_rgba_unmultiplied(190, 245, 200, 95),
            BookmarkColor::Blue => Color32::from_rgba_unmultiplied(190, 225, 255, 95),
            BookmarkColor::Red => Color32::from_rgba_unmultiplied(255, 205, 205, 95),
        }
    }
}
