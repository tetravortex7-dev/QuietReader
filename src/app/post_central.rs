use super::{Dialogs, DictionaryView, ProUpgradeDialog, QuietReaderApp, SearchView, TranslateView};
use crate::app::library_view::LibraryActions;
use crate::core::library::{LibraryMode, LibraryStore, restore_latest_snapshot};
use crate::core::library_export::export_library;
use crate::core::license::ProFeature;
use crate::document::DocumentViewKind;
use eframe::egui;
use std::path::PathBuf;

pub(super) struct PostCentralActions {
    pub library_actions: LibraryActions,
    pub close_tab_action: bool,
    pub on_open_file: bool,
    pub on_open_folder: bool,
    pub on_load_recent: Option<PathBuf>,
    pub save_settings_action: bool,
    pub pro_required: Option<ProFeature>,
    pub ambient_allowed: bool,
    pub neural_tts_allowed: bool,
}

pub(super) fn render(
    app: &mut QuietReaderApp,
    ctx: &egui::Context,
    actions: &mut PostCentralActions,
) {
    let mut should_save_library = actions.library_actions.save_library;
    if actions.library_actions.pick_library_folder
        && let Some(path) = Dialogs::pick_library_folder(app.settings.ui_language)
    {
        app.library.mode = LibraryMode::Folder;
        app.library.library_folder = Some(path.clone());
        app.library_dirty = true;
        app.library_view.request_scan(path);
        app.show_library = true;
        should_save_library = true;
    }
    if should_save_library {
        app.library_dirty = true;
        app.save_library_now();
    }
    if actions.library_actions.open_file {
        app.open_file_dialog();
    }
    if actions.library_actions.open_folder {
        app.open_folder_dialog();
    }
    if let Some(path) = actions.library_actions.open_path.take() {
        app.load_file(path);
    }
    if actions.library_actions.export_all
        && let Some(destination) = Dialogs::pick_export_folder(app.settings.ui_language)
    {
        let t = app.settings.ui_language.t();
        app.status_message = Some(match export_library(&destination, &app.library.entries) {
            Ok(report) => t
                .library_export_done
                .replace("{exported}", &report.exported.to_string())
                .replace("{skipped}", &report.skipped.to_string())
                .replace("{folder}", &report.destination.display().to_string()),
            Err(error) => t.library_export_failed.replace("{}", &error),
        });
    }
    if actions.library_actions.restore_snapshot {
        let t = app.settings.ui_language.t();
        app.status_message = Some(match restore_latest_snapshot(&app.library_path) {
            Ok(snapshot) => {
                app.library = LibraryStore::load_from_path(&app.library_path);
                app.library_dirty = false;
                app.show_library = true;
                if app.library.mode == LibraryMode::Folder
                    && let Some(folder) = app.library.library_folder.clone()
                {
                    app.library_view.request_scan(folder);
                }
                t.library_restore_done
                    .replace("{}", &snapshot.display().to_string())
            }
            Err(error) => {
                if error == "no library snapshots" {
                    t.library_restore_missing.to_string()
                } else {
                    t.library_restore_failed.replace("{}", &error)
                }
            }
        });
    }
    if app.library.mode == LibraryMode::ReaderOnly && app.document.is_some() {
        app.show_library = false;
    }

    let mut pdf_page_target = None;
    if !app.show_library
        && matches!(
            app.view_kind(),
            DocumentViewKind::Text | DocumentViewKind::Pdf
        )
        && app.document.is_some()
    {
        SearchView::render(
            ctx,
            &mut app.search,
            app.document.as_ref(),
            &mut app.current_chapter,
            &mut app.last_rendered_chapter,
            &mut app.pending_scroll_y,
            &mut pdf_page_target,
            app.font_size,
            &app.theme,
            app.settings.ui_language,
        );
    }
    if let Some(page) = pdf_page_target {
        app.pdf_view.go_to_page(page);
    }

    let has_text_content = app
        .document
        .as_ref()
        .map(|d| d.text_content().is_some())
        .unwrap_or(false);
    let book_lang = app
        .document
        .as_deref()
        .map(crate::core::ReaderCore::detect_document_language);
    if !actions.neural_tts_allowed
        && app.tts.status().engine == crate::app::tts::TtsEngineKind::Edge
    {
        app.tts.set_engine(crate::app::tts::TtsEngineKind::System);
    }
    let mut on_tts_play = false;
    app.tts.render_panel(
        ctx,
        &app.theme,
        has_text_content,
        book_lang,
        &mut on_tts_play,
        app.settings.ui_language,
        actions.neural_tts_allowed,
        &mut actions.pro_required,
    );
    let tts_preferences = app.tts.preferences();
    if app.settings.tts != tts_preferences {
        app.settings.tts = tts_preferences;
        app.mark_settings_dirty();
    }
    if on_tts_play {
        let start_para = app.last_tts_paragraph.unwrap_or_else(|| {
            if let Some(doc) = &app.document {
                crate::core::ReaderCore::estimate_visible_paragraph(
                    doc.as_ref(),
                    app.current_chapter,
                    app.current_scroll_y,
                    app.font_size,
                )
            } else {
                0
            }
        });
        app.start_tts_at_paragraph(start_para);
    }

    if actions.ambient_allowed {
        app.ambient.set_tts_active(app.tts.status().is_playing);
        if app.ambient.tick(ctx) {
            app.settings.ambient = app.ambient.settings.clone();
            app.mark_settings_dirty();
        }
        if !app.show_library
            && app
                .ambient
                .render(ctx, &app.theme, app.settings.ui_language)
        {
            app.settings.ambient = app.ambient.settings.clone();
            app.mark_settings_dirty();
        }
    } else {
        app.ambient.suspend();
    }

    if actions.close_tab_action {
        app.close_tab(app.active_tab);
    }
    if actions.on_open_file {
        app.open_file_dialog();
    }
    if actions.on_open_folder {
        app.open_folder_dialog();
    }
    if let Some(path) = actions.on_load_recent.take() {
        app.load_file(path);
    }

    let mut on_register_assoc = false;
    Dialogs::render_association_dialog(
        ctx,
        &mut app.show_association_dialog,
        &mut app.dont_show_association_again,
        &mut app.settings,
        &app.theme,
        &mut on_register_assoc,
        &mut actions.save_settings_action,
    );
    if on_register_assoc {
        app.register_file_association();
    }

    let mut recent_file_to_open = None;
    Dialogs::render_recent_dialog(
        ctx,
        &mut app.show_recent_dialog,
        &app.current_file_path,
        &mut app.settings,
        &app.theme,
        &mut recent_file_to_open,
        &mut actions.save_settings_action,
    );
    if let Some(path) = recent_file_to_open {
        app.load_file(path);
    }

    app.opds_state.poll(&mut app.settings);
    let mut opds_file_to_open = None;
    app.opds_state.render(
        ctx,
        &mut app.show_opds_dialog,
        &mut app.settings,
        &app.theme,
        &mut opds_file_to_open,
        &mut actions.pro_required,
    );
    if let Some(path) = opds_file_to_open {
        app.load_file(path);
        app.status_message = Some(
            app.settings
                .ui_language
                .t()
                .opds_book_downloaded_opened
                .to_string(),
        );
    }

    let mut on_speak_translated = None;
    let translation_allowed = app.settings.can_use(ProFeature::Translation);
    TranslateView::render(
        &mut app.translate,
        ctx,
        &app.theme,
        &mut app.settings,
        &mut on_speak_translated,
        &mut actions.save_settings_action,
        translation_allowed,
        &mut actions.pro_required,
    );
    if let Some(text) = on_speak_translated {
        app.tts.show_panel = true;
        app.tts.play_chapter(app.current_chapter, 0, vec![text]);
    }

    DictionaryView::render(
        &mut app.dictionary,
        ctx,
        &app.theme,
        app.settings.ui_language,
        &mut app.settings,
    );
    if app.dictionary.words_changed {
        app.dictionary.words_changed = false;
        app.mark_settings_dirty();
    }

    if let Some(feature) = actions.pro_required {
        app.pro_upgrade.request(feature);
    }
    ProUpgradeDialog::render(
        ctx,
        &mut app.pro_upgrade,
        &mut app.settings,
        &app.theme,
        &mut actions.save_settings_action,
    );
}
