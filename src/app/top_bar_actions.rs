use super::{QuietReaderApp, TabBarAction, TabsBar, render_top_bar};
use crate::app::toc_view::SidebarTab;
use crate::core::library::LibraryMode;
use crate::core::license::ProFeature;
use crate::document::DocumentViewKind;
use eframe::egui::{self, Panel};

pub(super) fn render(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pro_required: &mut Option<ProFeature>,
) {
    let ptr_top = ctx
        .input(|i| i.pointer.latest_pos().map(|p| p.y < 72.0))
        .unwrap_or(false);
    let update_tag = match app.updater.status() {
        crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
            Some(info.latest_version.clone())
        }
        _ => None,
    };
    if !app.is_fullscreen || ptr_top || app.show_settings_dialog {
        let license_active = app.settings.can_use(ProFeature::NeuralTts);
        let actions = render_top_bar(
            ui,
            app.document.is_some() && !app.show_library,
            app.library.mode != LibraryMode::ReaderOnly,
            app.is_fullscreen,
            app.show_toc,
            app.settings.effective_reading_ruler(),
            license_active,
            app.settings.ui_language,
            update_tag.as_deref(),
        );
        if actions.open_file {
            app.open_file_dialog();
        }
        if actions.open_folder {
            app.open_folder_dialog();
        }
        if actions.toggle_recent {
            app.show_recent_dialog = !app.show_recent_dialog;
        }
        if actions.toggle_library {
            app.show_library = !app.show_library;
            if app.show_library {
                app.show_recent_dialog = false;
            }
        }
        if actions.toggle_opds {
            app.show_opds_dialog = !app.show_opds_dialog;
        }
        if !app.show_library {
            if actions.toggle_toc {
                app.show_toc = !app.show_toc;
                if app.show_toc {
                    app.sidebar_tab = SidebarTab::Toc;
                }
            }
            if actions.toggle_bookmark {
                app.toggle_quick_bookmark();
            }
            if actions.toggle_bookmarks_panel {
                if !app.show_toc {
                    app.show_toc = true;
                    app.sidebar_tab = SidebarTab::Bookmarks;
                } else if app.sidebar_tab == SidebarTab::Bookmarks {
                    app.show_toc = false;
                } else {
                    app.sidebar_tab = SidebarTab::Bookmarks;
                }
            }
            if actions.toggle_search {
                if matches!(
                    app.view_kind(),
                    DocumentViewKind::Text | DocumentViewKind::Pdf
                ) && app.document.is_some()
                {
                    app.search.toggle();
                }
            }
            if actions.toggle_tts {
                app.tts.toggle_panel();
                if app.tts.show_panel && !app.tts.status().is_playing {
                    if let Some(doc) = &app.document {
                        let visible_para = crate::core::ReaderCore::estimate_visible_paragraph(
                            doc.as_ref(),
                            app.current_chapter,
                            app.current_scroll_y,
                            app.font_size,
                        );
                        app.last_tts_paragraph = Some(visible_para);
                    }
                }
            }
            if actions.toggle_reading_ruler {
                if app.settings.can_use(ProFeature::FocusTools) {
                    app.settings.reading_ruler = !app.settings.reading_ruler;
                    app.mark_settings_dirty();
                } else {
                    *pro_required = Some(ProFeature::FocusTools);
                }
            }
            if actions.toggle_ambient {
                if app.settings.can_use(ProFeature::AmbientAudio) {
                    app.ambient.toggle_panel();
                } else {
                    *pro_required = Some(ProFeature::AmbientAudio);
                }
            }
        }
        if actions.open_license {
            app.pro_upgrade.open(None);
        }
        if actions.toggle_fullscreen {
            app.set_fullscreen_mode(ctx, !app.is_fullscreen);
        }
        if actions.toggle_settings {
            app.show_settings_dialog = !app.show_settings_dialog;
        }
        if actions.open_update {
            if let crate::updater::UpdateStatus::UpdateAvailable(ref info) = app.updater.status() {
                ctx.open_url(egui::OpenUrl::new_tab(&info.release_url));
            }
        }

        if !app.tabs.is_empty() {
            let mut tab_action = TabBarAction::None;
            Panel::top("tabs_bar").show(ui, |ui| {
                tab_action = TabsBar::render(
                    ui,
                    &app.tabs,
                    app.active_tab,
                    &app.theme,
                    app.settings.ui_language,
                );
            });
            match tab_action {
                TabBarAction::None => {}
                TabBarAction::Activate(idx) => app.activate_tab(idx),
                TabBarAction::Close(idx) => app.close_tab(idx),
                TabBarAction::OpenNewFile => app.open_file_dialog(),
            }
        }
    }
}
