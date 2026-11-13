use crate::app::bars::render_bottom_bar;
use crate::app::icons::{
    STATUS_ARROW_SEP, dotted_responsive_row, dotted_strip, paint_arrow_pair, paint_dot_separator,
    paint_dotted_tokens,
};
use crate::app::library_view::{LibraryActions, LibraryViewState};
use crate::app::theme::ReaderTheme;
use crate::core::library::LibraryStore;
use crate::document::DocumentViewKind;
use crate::i18n::UiLanguage;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};

#[test]
fn test_vector_separators_and_arrow_render_within_bounds() {
    let ctx = egui::Context::default();
    for (width, lang) in [(180.0_f32, UiLanguage::En), (900.0_f32, UiLanguage::Ar)] {
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 400.0)));
        let is_rtl = lang.is_rtl();
        let t = lang.t();
        let mut min_rect = Rect::NOTHING;
        let mut out = ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.horizontal(|ui| {
                    paint_dot_separator(ui, 11.5, Color32::GRAY);
                    paint_arrow_pair(ui, "Exported: 2", "D:\\Books", Color32::WHITE, is_rtl);
                });
                dotted_strip(
                    ui,
                    &[t.fonts[0].badge, t.fonts[0].badge_tag],
                    11.5,
                    Color32::WHITE,
                    Color32::GRAY,
                );
                dotted_responsive_row(
                    ui,
                    &["240 WPM (actual)", "01:15 elapsed", "04:30 left"],
                    11.0,
                    Color32::WHITE,
                    Color32::GRAY,
                );
                let badge_rect =
                    Rect::from_min_size(ui.next_widget_position(), Vec2::new(width, 20.0));
                paint_dotted_tokens(
                    ui.painter(),
                    badge_rect,
                    &["EPUB", "FB2", "PDF", "CBZ"],
                    FontId::proportional(9.5),
                    Color32::WHITE,
                    Color32::GRAY,
                );
                min_rect = ui.min_rect();
            });
        });
        out.textures_delta.clear();
        assert!(min_rect.width() <= width + 2.0);
    }
}

#[test]
fn test_greeting_and_bottom_bar_render_without_unicode_arrows_or_bullets() {
    let ctx = egui::Context::default();
    for lang in [UiLanguage::En, UiLanguage::Ru, UiLanguage::Ar] {
        let mut store = LibraryStore::default();
        let mut view = LibraryViewState::default();
        let settings = AppSettings::default();
        let mut actions = LibraryActions::default();
        let status = format!("Done{STATUS_ARROW_SEP}C:\\Export");
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)));
        let mut out = ctx.run_ui(raw, |ui| {
            render_bottom_bar(
                ui,
                &None,
                &ReaderTheme::Light,
                Some(&status),
                1.0,
                DocumentViewKind::Text,
                (0, 0),
                (0, 0),
                (0, 1, 0.0, 0.0),
                None,
                lang,
            );
            egui::CentralPanel::default().show(ui, |ui| {
                view.render(
                    ui,
                    &settings,
                    &mut store,
                    &ReaderTheme::Light,
                    lang,
                    &mut actions,
                );
            });
        });
        out.textures_delta.clear();
    }
}
