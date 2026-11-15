mod appearance;
mod audio_dict;
mod chrome;
mod reading;
mod search_reset;
mod typography;
pub mod widgets;

#[cfg(all(test, feature = "enable_tests"))]
pub use appearance::compute_theme_swatch_cols;
#[cfg(all(test, feature = "enable_tests"))]
pub use search_reset::{SettingsSectionId, apply_default_app_preferences, matching_sections};
#[cfg(all(test, feature = "enable_tests"))]
pub use typography::compute_font_grid_cols;

use self::chrome::{
    render_compact_nav_bar, render_left_rail, render_modal_footer, render_modal_header,
    render_reset_confirm_banner,
};
use self::search_reset::apply_default_preferences;
#[cfg(not(all(test, feature = "enable_tests")))]
use self::search_reset::{SettingsSectionId, matching_sections};
use self::widgets::SettingsPalette;
use super::ambient::AmbientController;
use super::dialogs::SettingsTab;
use super::dictionary::DictionaryState;
use super::fonts::apply_font_family;
use super::icons::{Icon, IconLabel};
use super::license_view::ProUpgradeState;
use super::settings_about::{
    render_about_tab, render_pro_section, render_version_and_licenses_section,
};
use super::settings_system::{
    render_associations_section, render_encoding_section, render_library_pin_section,
    render_storage_section, render_system_tab, render_updates_section,
};
use super::theme::ReaderTheme;
use super::tts::TtsController;
use crate::core::library::LibraryStore;
use crate::i18n::Translations;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Vec2};

#[derive(Default)]
pub struct SettingsActions {
    pub close: bool,
    pub toggle_autoscroll: bool,
    pub save_settings: bool,
    pub register_association: bool,
    pub reload_text_encoding: bool,
    pub open_ambient: bool,
    pub open_tts: bool,
    pub open_dictionary: bool,
    pub open_vocabulary: bool,
}

pub struct SettingsView;

impl SettingsView {
    pub(super) fn render_overlay(
        app: &mut super::QuietReaderApp,
        ctx: &egui::Context,
        bounds: Rect,
        toggle_autoscroll: &mut bool,
        save_settings: &mut bool,
        register_assoc: &mut bool,
    ) {
        let mut is_autoscrolling_flag = app.autoscroll.is_active();
        let actions = Self::render(
            ctx,
            bounds,
            &mut app.settings_tab,
            &mut app.settings,
            &mut app.library,
            &mut app.library_dirty,
            &mut app.font_size,
            &mut app.theme,
            &mut app.keep_screen_on,
            &mut is_autoscrolling_flag,
            &mut app.ambient,
            &mut app.tts,
            &app.dictionary,
            &app.updater,
            &mut app.pro_upgrade,
        );
        if actions.close {
            app.show_settings_dialog = false;
        }
        if actions.toggle_autoscroll {
            *toggle_autoscroll = true;
        }
        if actions.save_settings {
            *save_settings = true;
        }
        if actions.register_association {
            *register_assoc = true;
        }
        if actions.reload_text_encoding {
            app.reload_current_text_encoding();
        }
        if actions.open_ambient {
            app.show_settings_dialog = false;
            app.ambient.is_open = true;
        }
        if actions.open_tts {
            app.show_settings_dialog = false;
            app.tts.show_panel = true;
        }
        if actions.open_dictionary {
            app.show_settings_dialog = false;
            app.dictionary.visible = true;
            app.dictionary.catalog_visible = false;
        }
        if actions.open_vocabulary {
            app.show_settings_dialog = false;
            app.dictionary.words_visible = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        ctx: &egui::Context,
        bounds: Rect,
        settings_tab: &mut SettingsTab,
        settings: &mut AppSettings,
        library: &mut LibraryStore,
        library_dirty: &mut bool,
        font_size: &mut f32,
        theme: &mut ReaderTheme,
        keep_screen_on: &mut bool,
        is_autoscrolling: &mut bool,
        ambient: &mut AmbientController,
        tts: &mut TtsController,
        dictionary: &DictionaryState,
        updater: &crate::updater::UpdateChecker,
        pro: &mut ProUpgradeState,
    ) -> SettingsActions {
        let mut actions = SettingsActions::default();
        let palette = SettingsPalette::from_theme(theme, library.design);
        let t = settings.ui_language.t();

        let search_id = egui::Id::unique("settings_search_query");
        let reset_confirm_id = egui::Id::unique("settings_reset_confirm");
        let reset_done_id = egui::Id::unique("settings_reset_done");
        let focus_search_flag_id = egui::Id::unique("settings_focus_search_requested");

        let mut search_query: String = ctx.data(|d| d.get_temp(search_id).unwrap_or_default());
        let mut reset_confirm: bool = ctx.data(|d| d.get_temp(reset_confirm_id).unwrap_or(false));
        let mut reset_done: bool = ctx.data(|d| d.get_temp(reset_done_id).unwrap_or(false));
        let focus_search: bool = ctx.data(|d| d.get_temp(focus_search_flag_id).unwrap_or(false));
        if focus_search {
            ctx.data_mut(|d| d.insert_temp(focus_search_flag_id, false));
        }

        let outer_pad_x = if bounds.width() >= 960.0 {
            28.0
        } else if bounds.width() >= 600.0 {
            16.0
        } else {
            8.0
        };
        let outer_pad_y = if bounds.height() >= 760.0 { 22.0 } else { 10.0 };
        let modal_w = (bounds.width() - outer_pad_x * 2.0).clamp(300.0, 1120.0);
        let modal_h = (bounds.height() - outer_pad_y * 2.0).clamp(320.0, 860.0);
        let modal_rect = Rect::from_center_size(bounds.center(), Vec2::new(modal_w, modal_h));

        if !pro.is_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if reset_confirm {
                reset_confirm = false;
            } else if !search_query.trim().is_empty() {
                search_query.clear();
            } else {
                actions.close = true;
            }
        }

        egui::Area::new(egui::Id::unique("settings_modal_overlay"))
            .order(egui::Order::Middle)
            .fixed_pos(bounds.min)
            .interactable(!pro.is_open)
            .show(ctx, |ui| {
                ui.set_clip_rect(bounds);
                ui.set_min_size(bounds.size());
                ui.set_max_size(bounds.size());
                ui.painter().rect_filled(bounds, 0.0, palette.scrim);

                let strips = [
                    Rect::from_min_max(bounds.min, Pos2::new(bounds.max.x, modal_rect.min.y)),
                    Rect::from_min_max(Pos2::new(bounds.min.x, modal_rect.max.y), bounds.max),
                    Rect::from_min_max(
                        Pos2::new(bounds.min.x, modal_rect.min.y),
                        Pos2::new(modal_rect.min.x, modal_rect.max.y),
                    ),
                    Rect::from_min_max(
                        Pos2::new(modal_rect.max.x, modal_rect.min.y),
                        Pos2::new(bounds.max.x, modal_rect.max.y),
                    ),
                ];
                for (idx, strip) in strips.iter().enumerate() {
                    if strip.width() > 0.0 && strip.height() > 0.0 {
                        let resp = ui.interact(
                            *strip,
                            ui.make_persistent_id(("settings_scrim_strip", idx)),
                            Sense::click(),
                        );
                        if resp.clicked() {
                            actions.close = true;
                        }
                    }
                }

                let mut modal_ui = ui.new_child(egui::UiBuilder::new().max_rect(modal_rect));
                modal_ui.set_clip_rect(modal_rect);
                let shadow = egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_black_alpha(if palette.is_dark { 150 } else { 44 }),
                };

                egui::Frame::new()
                    .fill(palette.modal_bg)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(palette.corner_lg)
                    .shadow(shadow)
                    .show(&mut modal_ui, |ui| {
                        ui.set_min_size(modal_rect.size());
                        ui.set_max_size(modal_rect.size());
                        ui.set_max_width(modal_w);
                        ui.spacing_mut().item_spacing = Vec2::ZERO;

                        render_modal_header(
                            ui,
                            &palette,
                            &t,
                            modal_w,
                            &mut search_query,
                            focus_search,
                            &mut reset_confirm,
                            &mut actions.close,
                        );

                        if reset_confirm {
                            render_reset_confirm_banner(
                                ui,
                                &palette,
                                &t,
                                &mut reset_confirm,
                                &mut reset_done,
                                || {
                                    apply_default_preferences(
                                        settings,
                                        library,
                                        library_dirty,
                                        theme,
                                        font_size,
                                        keep_screen_on,
                                        ambient,
                                        tts,
                                    );
                                    let effective = settings.effective_font_family();
                                    apply_font_family(ctx, &effective);
                                    actions.save_settings = true;
                                },
                            );
                        }

                        let footer_h = 46.0;
                        let body_h = (ui.available_height() - footer_h).max(120.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(modal_w, body_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_min_size(Vec2::new(modal_w, body_h));
                                ui.set_max_size(Vec2::new(modal_w, body_h));

                                let compact_nav = modal_w < 740.0;
                                if compact_nav {
                                    render_compact_nav_bar(
                                        ui,
                                        &palette,
                                        &t,
                                        settings_tab,
                                        &mut search_query,
                                        &mut reset_done,
                                    );
                                    render_scrollable_content(
                                        ui,
                                        ctx,
                                        &palette,
                                        &t,
                                        settings_tab,
                                        &search_query,
                                        reset_done,
                                        settings,
                                        library,
                                        library_dirty,
                                        font_size,
                                        theme,
                                        keep_screen_on,
                                        is_autoscrolling,
                                        ambient,
                                        tts,
                                        dictionary,
                                        updater,
                                        pro,
                                        &mut actions,
                                    );
                                } else {
                                    let rail_w = 224.0;
                                    let content_w = (modal_w - rail_w - 1.0).max(200.0);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(rail_w, body_h),
                                            egui::Layout::top_down(egui::Align::Min),
                                            |ui| {
                                                render_left_rail(
                                                    ui,
                                                    &palette,
                                                    &t,
                                                    settings_tab,
                                                    &mut search_query,
                                                    &mut reset_done,
                                                    settings,
                                                    body_h,
                                                );
                                            },
                                        );
                                        let sep_rect = ui.allocate_space(Vec2::new(1.0, body_h)).1;
                                        ui.painter().vline(
                                            sep_rect.center().x,
                                            sep_rect.min.y..=sep_rect.max.y,
                                            Stroke::new(1.0, palette.border_subtle),
                                        );
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(content_w, body_h),
                                            egui::Layout::top_down(egui::Align::Min),
                                            |ui| {
                                                ui.set_min_size(Vec2::new(content_w, body_h));
                                                render_scrollable_content(
                                                    ui,
                                                    ctx,
                                                    &palette,
                                                    &t,
                                                    settings_tab,
                                                    &search_query,
                                                    reset_done,
                                                    settings,
                                                    library,
                                                    library_dirty,
                                                    font_size,
                                                    theme,
                                                    keep_screen_on,
                                                    is_autoscrolling,
                                                    ambient,
                                                    tts,
                                                    dictionary,
                                                    updater,
                                                    pro,
                                                    &mut actions,
                                                );
                                            },
                                        );
                                    });
                                }
                            },
                        );

                        render_modal_footer(
                            ui,
                            &palette,
                            &t,
                            modal_w,
                            footer_h,
                            &mut actions.close,
                        );
                    });
            });

        ctx.data_mut(|d| {
            d.insert_temp(search_id, search_query);
            d.insert_temp(reset_confirm_id, reset_confirm);
            d.insert_temp(reset_done_id, reset_done);
        });

        actions
    }
}

#[allow(clippy::too_many_arguments)]
fn render_scrollable_content(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &SettingsPalette,
    t: &Translations,
    settings_tab: &SettingsTab,
    search_query: &str,
    reset_done: bool,
    settings: &mut AppSettings,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    font_size: &mut f32,
    theme: &mut ReaderTheme,
    keep_screen_on: &mut bool,
    is_autoscrolling: &mut bool,
    ambient: &mut AmbientController,
    tts: &mut TtsController,
    dictionary: &DictionaryState,
    updater: &crate::updater::UpdateChecker,
    pro: &mut ProUpgradeState,
    actions: &mut SettingsActions,
) {
    let pad = if ui.available_width() >= 560.0 {
        20
    } else {
        12
    };
    egui::Frame::new()
        .fill(p.modal_bg)
        .inner_margin(Margin::symmetric(pad, 16))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt((
                    "settings_content_scroll",
                    *settings_tab,
                    !search_query.trim().is_empty(),
                ))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 4.0);

                    if let Some(err) = AppSettings::last_save_error() {
                        ui.add(
                            IconLabel::new(
                                Icon::Warning,
                                &t.save_settings_error.replace("{}", &err),
                            )
                            .color(p.error)
                            .strong(),
                        );
                        ui.add_space(8.0);
                    } else if let Some(err) = AppSettings::last_secret_error() {
                        ui.add(
                            IconLabel::new(
                                Icon::Warning,
                                &t.secrets_storage_error.replace("{}", &err),
                            )
                            .color(p.error)
                            .strong(),
                        );
                        ui.add_space(8.0);
                    }

                    if reset_done {
                        ui.add(
                            IconLabel::new(Icon::Check, t.extra.settings_reset_done)
                                .size(12.5)
                                .color(p.tertiary)
                                .strong(),
                        );
                        ui.add_space(8.0);
                    }

                    if !search_query.trim().is_empty() {
                        let matches = matching_sections(search_query, t);
                        if matches.is_empty() {
                            ui.add_space(48.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(t.extra.settings_search_empty)
                                        .size(14.0)
                                        .color(p.text_variant),
                                );
                            });
                        } else {
                            for (idx, sec) in matches.into_iter().enumerate() {
                                if idx > 0 {
                                    ui.add_space(14.0);
                                }
                                render_section_by_id(
                                    sec,
                                    ui,
                                    ctx,
                                    p,
                                    t,
                                    settings,
                                    library,
                                    library_dirty,
                                    font_size,
                                    theme,
                                    keep_screen_on,
                                    is_autoscrolling,
                                    ambient,
                                    tts,
                                    dictionary,
                                    updater,
                                    pro,
                                    actions,
                                );
                            }
                        }
                    } else {
                        match settings_tab {
                            SettingsTab::Appearance => appearance::render_appearance_tab(
                                ui,
                                p,
                                t,
                                theme,
                                library,
                                library_dirty,
                                settings,
                                &mut actions.save_settings,
                            ),
                            SettingsTab::Fonts => typography::render_typography_tab(
                                ui,
                                ctx,
                                p,
                                t,
                                theme,
                                settings,
                                font_size,
                                &mut actions.save_settings,
                                pro,
                            ),
                            SettingsTab::Reading => reading::render_reading_tab(
                                ui,
                                p,
                                t,
                                settings,
                                keep_screen_on,
                                is_autoscrolling,
                                &mut actions.toggle_autoscroll,
                                &mut actions.save_settings,
                                pro,
                            ),
                            SettingsTab::AmbienceSpeech => audio_dict::render_ambience_tts_tab(
                                ui,
                                p,
                                t,
                                settings,
                                ambient,
                                tts,
                                &mut actions.save_settings,
                                &mut actions.open_ambient,
                                &mut actions.open_tts,
                                pro,
                            ),
                            SettingsTab::DictionariesTranslation => {
                                audio_dict::render_dict_translate_tab(
                                    ui,
                                    p,
                                    t,
                                    settings,
                                    dictionary,
                                    &mut actions.save_settings,
                                    &mut actions.open_dictionary,
                                    &mut actions.open_vocabulary,
                                    pro,
                                )
                            }
                            SettingsTab::System => render_system_tab(
                                ui,
                                p,
                                settings,
                                &mut actions.register_association,
                                &mut actions.save_settings,
                                &mut actions.reload_text_encoding,
                                updater,
                            ),
                            SettingsTab::About => {
                                render_about_tab(ui, p, settings, pro, &mut actions.save_settings)
                            }
                        }
                    }

                    ui.add_space(16.0);
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn render_section_by_id(
    section: SettingsSectionId,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    font_size: &mut f32,
    theme: &mut ReaderTheme,
    keep_screen_on: &mut bool,
    is_autoscrolling: &mut bool,
    ambient: &mut AmbientController,
    tts: &mut TtsController,
    dictionary: &DictionaryState,
    updater: &crate::updater::UpdateChecker,
    pro: &mut ProUpgradeState,
    actions: &mut SettingsActions,
) {
    match section {
        SettingsSectionId::ThemeAndDisplay => appearance::render_theme_and_display_section(
            ui,
            p,
            t,
            theme,
            library,
            library_dirty,
            settings,
            &mut actions.save_settings,
        ),
        SettingsSectionId::Language => {
            appearance::render_language_section(ui, p, t, settings, &mut actions.save_settings)
        }
        SettingsSectionId::Typography => typography::render_typography_section(
            ui,
            ctx,
            p,
            t,
            theme,
            settings,
            font_size,
            &mut actions.save_settings,
            pro,
        ),
        SettingsSectionId::FocusTools => {
            reading::render_focus_tools_section(ui, p, t, settings, &mut actions.save_settings, pro)
        }
        SettingsSectionId::NavigationDisplay => reading::render_navigation_display_section(
            ui,
            p,
            t,
            settings,
            keep_screen_on,
            &mut actions.save_settings,
        ),
        SettingsSectionId::AutoScroll => reading::render_autoscroll_section(
            ui,
            p,
            t,
            settings,
            is_autoscrolling,
            &mut actions.toggle_autoscroll,
            &mut actions.save_settings,
        ),
        SettingsSectionId::ComicsAndPdf => reading::render_comics_and_pdf_section(
            ui,
            p,
            t,
            settings,
            &mut actions.save_settings,
            pro,
        ),
        SettingsSectionId::Hotkeys => reading::render_hotkeys_section(ui, p, t),
        SettingsSectionId::Ambience => audio_dict::render_ambient_section(
            ui,
            p,
            t,
            settings,
            ambient,
            &mut actions.save_settings,
            &mut actions.open_ambient,
            pro,
        ),
        SettingsSectionId::TextToSpeech => audio_dict::render_tts_section(
            ui,
            p,
            t,
            settings,
            tts,
            &mut actions.save_settings,
            &mut actions.open_tts,
            pro,
        ),
        SettingsSectionId::Translation => audio_dict::render_translation_section(
            ui,
            p,
            t,
            settings,
            &mut actions.save_settings,
            pro,
        ),
        SettingsSectionId::DictionariesVocab => audio_dict::render_dictionary_vocab_section(
            ui,
            p,
            t,
            settings,
            dictionary,
            &mut actions.open_dictionary,
            &mut actions.open_vocabulary,
        ),
        SettingsSectionId::Encoding => render_encoding_section(
            ui,
            p,
            t,
            settings,
            &mut actions.save_settings,
            &mut actions.reload_text_encoding,
        ),
        SettingsSectionId::LibraryPin => {
            render_library_pin_section(ui, p, t, settings, &mut actions.save_settings)
        }
        SettingsSectionId::Updates => {
            render_updates_section(ui, p, t, settings, &mut actions.save_settings, updater)
        }
        SettingsSectionId::FileAssociations => render_associations_section(
            ui,
            p,
            t,
            settings,
            &mut actions.register_association,
            &mut actions.save_settings,
        ),
        SettingsSectionId::Storage => render_storage_section(ui, p, t, &mut actions.save_settings),
        SettingsSectionId::ProUpgrade => {
            render_pro_section(ui, p, t, settings, pro, &mut actions.save_settings)
        }
        SettingsSectionId::AboutVersion => render_version_and_licenses_section(ui, p, t),
    }
}
