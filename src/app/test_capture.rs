use super::QuietReaderApp;
use super::dialogs::SettingsTab;
use super::theme::ReaderTheme;
use crate::core::library::{LibraryMode, ShelfViewMode, UiDesignMode};
use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn is_active() -> bool {
    std::env::var_os("QUIETREADER_CAPTURE_OUT").is_some()
}

pub fn step(app: &mut QuietReaderApp, ctx: &egui::Context) {
    let Ok(out_path) = std::env::var("QUIETREADER_CAPTURE_OUT") else {
        return;
    };

    let state_id = egui::Id::unique("quietreader_test_capture_state");
    let (mut frame_count, mut doc_applied, mut settled_frames, mut requested, done_flag): (
        u32,
        bool,
        u32,
        bool,
        Option<Arc<AtomicBool>>,
    ) = ctx
        .data(|d| d.get_temp(state_id))
        .unwrap_or((0, false, 0, false, None));

    if let Some(flag) = &done_flag
        && flag.load(Ordering::Acquire)
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        return;
    }

    let has_book_env =
        std::env::var("QUIETREADER_CAPTURE_BOOK").is_ok_and(|p| std::path::Path::new(&p).exists());

    if frame_count == 0 {
        app.show_library = true;
        app.show_association_dialog = false;
        app.show_settings_dialog = false;
        app.show_recent_dialog = false;
        app.show_opds_dialog = false;
        app.settings.text_paragraph_mode = std::env::var("QUIETREADER_CAPTURE_PARAGRAPH")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        app.focus_read.rsvp.active = false;
        app.show_toc = std::env::var("QUIETREADER_CAPTURE_TOC")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

        if let Ok(book_path) = std::env::var("QUIETREADER_CAPTURE_BOOK") {
            let p = std::path::PathBuf::from(book_path);
            if p.exists() {
                app.show_library = false;
                app.load_file(p);
            }
        }
        if let Ok(two_cols) = std::env::var("QUIETREADER_CAPTURE_TWO_COLS") {
            match two_cols.to_lowercase().as_str() {
                "1" | "true" | "yes" => app.settings.text_two_columns = true,
                "0" | "false" | "no" => app.settings.text_two_columns = false,
                _ => {}
            }
        }
        if let Ok(tab_name) = std::env::var("QUIETREADER_CAPTURE_SETTINGS") {
            app.settings_tab = match tab_name.to_lowercase().as_str() {
                "appearance" => SettingsTab::Appearance,
                "fonts" | "typography" => SettingsTab::Fonts,
                "reading" | "focus" => SettingsTab::Reading,
                "ambient" | "tts" | "ambience" => SettingsTab::AmbienceSpeech,
                "dict" | "dictionary" | "translate" => SettingsTab::DictionariesTranslation,
                "system" => SettingsTab::System,
                "about" | "pro" => SettingsTab::About,
                _ => SettingsTab::Appearance,
            };
            app.show_settings_dialog = true;
        }
        if let Ok(query) = std::env::var("QUIETREADER_CAPTURE_SETTINGS_SEARCH") {
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::unique("settings_search_query"), query);
            });
        }

        if let Ok(theme_name) = std::env::var("QUIETREADER_CAPTURE_THEME") {
            match theme_name.to_lowercase().as_str() {
                "dark" => app.theme = ReaderTheme::Dark,
                "light" => app.theme = ReaderTheme::Light,
                "sepia" => app.theme = ReaderTheme::Sepia,
                "oled" => app.theme = ReaderTheme::Oled,
                "warm" => app.theme = ReaderTheme::Warm,
                "mint" => app.theme = ReaderTheme::Mint,
                "twilight" => app.theme = ReaderTheme::Twilight,
                _ => {}
            }
        }
        if let Ok(design_name) = std::env::var("QUIETREADER_CAPTURE_DESIGN") {
            match design_name.to_lowercase().as_str() {
                "cozy" => app.library.design = UiDesignMode::Cozy,
                "studio" => app.library.design = UiDesignMode::Studio,
                _ => {}
            }
        }
        if let Ok(mode_name) = std::env::var("QUIETREADER_CAPTURE_MODE") {
            match mode_name.to_lowercase().as_str() {
                "shelf" => app.library.mode = LibraryMode::Shelf,
                "folder" => app.library.mode = LibraryMode::Folder,
                "reader" => app.library.mode = LibraryMode::ReaderOnly,
                _ => {}
            }
        }
        if let Ok(view_name) = std::env::var("QUIETREADER_CAPTURE_VIEW") {
            match view_name.to_lowercase().as_str() {
                "mini" => app.library.shelf_view = ShelfViewMode::Mini,
                "standard" => app.library.shelf_view = ShelfViewMode::Standard,
                "maxi" => app.library.shelf_view = ShelfViewMode::Maxi,
                _ => {}
            }
        }
        if let Ok(size_str) = std::env::var("QUIETREADER_CAPTURE_SIZE")
            && let Some((w_s, h_s)) = size_str.split_once('x')
            && let (Ok(w), Ok(h)) = (w_s.parse::<f32>(), h_s.parse::<f32>())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
    }

    frame_count = frame_count.saturating_add(1);
    let ready = !has_book_env || app.document.is_some();
    if ready && !doc_applied {
        doc_applied = true;
        if let Ok(ch_str) = std::env::var("QUIETREADER_CAPTURE_CHAPTER")
            && let Ok(ch_idx) = ch_str.parse::<usize>()
        {
            let max_ch = app
                .document
                .as_ref()
                .map(|d| d.chapters().len().saturating_sub(1))
                .unwrap_or(0);
            let target = ch_idx.min(max_ch);
            app.current_chapter = target;
            app.last_rendered_chapter = target;
            app.pending_scroll_y = Some(0.0);
            app.current_scroll_y = 0.0;
        }
        if let Ok(note_id) = std::env::var("QUIETREADER_CAPTURE_FOOTNOTE") {
            let pinned_id = egui::Id::unique((
                "quietreader_footnote_pinned",
                &app.current_file_path,
                app.current_chapter,
            ));
            ctx.data_mut(|d| {
                d.insert_temp(pinned_id, note_id);
            });
        }
        if std::env::var("QUIETREADER_CAPTURE_BOOKMARK")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        {
            let exists = super::reader_chrome::has_quick_bookmark(
                &app.settings,
                app.current_file_path.as_deref(),
                app.current_chapter,
                app.current_scroll_y,
                None,
            );
            if !exists {
                app.toggle_quick_bookmark();
            }
            if !std::env::var("QUIETREADER_CAPTURE_KEEP_STATUS")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            {
                app.status_message = None;
            }
        }
    }
    if doc_applied {
        settled_frames = settled_frames.saturating_add(1);
    }

    let flag = done_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));

    if frame_count >= 12 && settled_frames >= 6 && !requested {
        requested = true;
        let path_clone = out_path.clone();
        let flag_clone = Arc::clone(&flag);
        let ctx_clone = ctx.clone();
        ctx.request_screenshot(move |color_image: Arc<egui::ColorImage>| {
            if let Some(parent) = std::path::Path::new(&path_clone).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let width = color_image.size[0] as u32;
            let height = color_image.size[1] as u32;
            let rgba: Vec<u8> = color_image
                .pixels
                .iter()
                .flat_map(|c| c.to_srgba_unmultiplied())
                .collect();
            let _ = image::save_buffer_with_format(
                &path_clone,
                &rgba,
                width,
                height,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            );
            flag_clone.store(true, Ordering::Release);
            ctx_clone.request_repaint();
        });
    }

    ctx.data_mut(|d| {
        d.insert_temp(
            state_id,
            (
                frame_count,
                doc_applied,
                settled_frames,
                requested,
                Some(flag),
            ),
        );
    });
    ctx.request_repaint();
}
