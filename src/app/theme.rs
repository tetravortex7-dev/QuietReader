use eframe::egui::Color32;

/// Цветовые темы оформления читалки
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReaderTheme {
    Dark,
    Light,
    Sepia,
    Oled,
    Warm,
    Mint,
}

impl ReaderTheme {
    pub fn is_dark(&self) -> bool {
        matches!(self, ReaderTheme::Dark | ReaderTheme::Oled)
    }

    /// Цвет фона области чтения
    pub fn bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(26, 26, 28),
            ReaderTheme::Light => Color32::from_rgb(248, 249, 250),
            ReaderTheme::Sepia => Color32::from_rgb(244, 237, 222),
            ReaderTheme::Oled => Color32::from_rgb(0, 0, 0),
            ReaderTheme::Warm => Color32::from_rgb(250, 240, 220),
            ReaderTheme::Mint => Color32::from_rgb(238, 245, 238),
        }
    }

    /// Основной цвет текста
    pub fn text_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(220, 220, 220),
            ReaderTheme::Light => Color32::from_rgb(33, 37, 41),
            ReaderTheme::Sepia => Color32::from_rgb(60, 48, 36),
            ReaderTheme::Oled => Color32::from_rgb(205, 205, 205),
            ReaderTheme::Warm => Color32::from_rgb(70, 50, 30),
            ReaderTheme::Mint => Color32::from_rgb(38, 55, 42),
        }
    }

    /// Цвет фона панелей (верхней, нижней, оглавления)
    pub fn panel_bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(35, 35, 38),
            ReaderTheme::Light => Color32::from_rgb(238, 240, 244),
            ReaderTheme::Sepia => Color32::from_rgb(235, 227, 210),
            ReaderTheme::Oled => Color32::from_rgb(10, 10, 12),
            ReaderTheme::Warm => Color32::from_rgb(242, 230, 208),
            ReaderTheme::Mint => Color32::from_rgb(226, 236, 226),
        }
    }

    /// Акцентный цвет (кнопки, выделение глав)
    pub fn accent_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(100, 160, 255),
            ReaderTheme::Light => Color32::from_rgb(0, 102, 204),
            ReaderTheme::Sepia => Color32::from_rgb(140, 90, 40),
            ReaderTheme::Oled => Color32::from_rgb(85, 145, 255),
            ReaderTheme::Warm => Color32::from_rgb(180, 95, 25),
            ReaderTheme::Mint => Color32::from_rgb(45, 125, 80),
        }
    }

    /// Формирует стиль визуалов egui для данной темы
    pub fn create_visuals(&self) -> eframe::egui::Visuals {
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
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(20, 15, 10);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(0, 0, 0);
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(228, 220, 203);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(210, 200, 180));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(228, 220, 203);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(228, 220, 203);
            }
            ReaderTheme::Warm => {
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(30, 20, 10);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(10, 5, 0);
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(236, 224, 200);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(218, 204, 178));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(236, 224, 200);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(236, 224, 200);
            }
            ReaderTheme::Mint => {
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(15, 30, 20);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(0, 20, 10);
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(220, 230, 220);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(200, 215, 200));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(220, 230, 220);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(220, 230, 220);
            }
            ReaderTheme::Light => {
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(244, 245, 248);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgb(220, 224, 230));
                visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(244, 245, 248);
            }
            ReaderTheme::Dark => {}
        }
        visuals
    }
}
