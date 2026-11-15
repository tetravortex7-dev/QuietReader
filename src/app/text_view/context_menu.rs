use crate::app::icons::{Icon, IconButton, IconLabel};
use crate::app::theme::ReaderTheme;
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, Vec2};
use std::path::PathBuf;

pub(crate) struct ContextMenuParams<'a> {
    pub doc: &'a dyn Document,
    pub current_chapter: usize,
    pub can_copy: bool,
    pub single_text: &'a str,
    pub tts_paragraph_idx: usize,
    pub translation_provider: crate::settings::TranslationProvider,
    pub t: &'a crate::i18n::Translations,
    pub on_start_tts: &'a mut Option<usize>,
    pub on_translate: &'a mut Option<String>,
    pub on_dictionary: Option<&'a mut Option<String>>,
    pub on_wikipedia: Option<&'a mut Option<String>>,
    pub on_save_quote: &'a mut Option<String>,
}

pub(crate) fn render_text_context_menu(ui: &mut egui::Ui, params: ContextMenuParams<'_>) {
    let ContextMenuParams {
        doc,
        current_chapter,
        can_copy,
        single_text,
        tts_paragraph_idx,
        translation_provider,
        t,
        on_start_tts,
        on_translate,
        on_dictionary,
        on_wikipedia,
        on_save_quote,
    } = params;

    let selected_text = crate::app::selection::get_selected_text(ui.ctx());
    let has_selection = selected_text.as_ref().is_some_and(|s| !s.trim().is_empty())
        || crate::app::selection::has_active_selection(ui.ctx());
    if selected_text.is_none() && has_selection {
        crate::app::selection::request_selection_capture(ui.ctx());
    }
    let prov_name = translation_provider.name();

    if ui
        .add(
            IconButton::new(Icon::Speech, t.read_aloud_from_here)
                .min_size(Vec2::new(ui.available_width(), 26.0)),
        )
        .clicked()
    {
        *on_start_tts = Some(tts_paragraph_idx);
        ui.close();
    }
    if ui
        .add(
            IconButton::new(Icon::Globe, t.translate_paragraph)
                .min_size(Vec2::new(ui.available_width(), 26.0)),
        )
        .on_hover_text(format!("{} ({})", t.translate_paragraph, prov_name))
        .clicked()
    {
        *on_translate = Some(single_text.to_string());
        ui.close();
    }
    let sel_btn = ui.add_enabled(
        has_selection,
        IconButton::new(Icon::Globe, t.translate_selection)
            .min_size(Vec2::new(ui.available_width(), 26.0)),
    );
    if sel_btn.clicked() {
        let sel = selected_text
            .clone()
            .or_else(|| crate::app::selection::get_selected_text(ui.ctx()));
        if let Some(text) = sel {
            *on_translate = Some(text);
        }
        ui.close();
    }
    if let Some(on_dict) = on_dictionary {
        let dict_btn = ui.add_enabled(
            has_selection,
            IconButton::new(Icon::Book, t.dictionary_lookup)
                .min_size(Vec2::new(ui.available_width(), 26.0)),
        );
        if dict_btn.clicked() {
            let sel = selected_text
                .clone()
                .or_else(|| crate::app::selection::get_selected_text(ui.ctx()));
            if let Some(text) = sel {
                *on_dict = Some(text);
            }
            ui.close();
        }
        if !has_selection {
            dict_btn.on_disabled_hover_text(t.dictionary_lookup);
        }
    }
    if let Some(on_wiki) = on_wikipedia {
        let wiki_btn = ui.add_enabled(
            has_selection,
            IconButton::new(Icon::Globe, t.wikipedia_lookup)
                .min_size(Vec2::new(ui.available_width(), 26.0)),
        );
        if wiki_btn.clicked() {
            let sel = selected_text
                .clone()
                .or_else(|| crate::app::selection::get_selected_text(ui.ctx()));
            if let Some(text) = sel {
                *on_wiki = Some(text);
            }
            ui.close();
        }
        if !has_selection {
            wiki_btn.on_disabled_hover_text(t.wikipedia_lookup);
        }
    }
    let quote_btn = ui.add_enabled(
        has_selection,
        IconButton::new(Icon::Quote, t.save_quote_bookmark)
            .min_size(Vec2::new(ui.available_width(), 26.0)),
    );
    if quote_btn.clicked() {
        let sel = selected_text
            .clone()
            .or_else(|| crate::app::selection::get_selected_text(ui.ctx()));
        if let Some(text) = sel {
            *on_save_quote = Some(text);
        }
        ui.close();
    }
    if has_selection {
        sel_btn.on_hover_text(format!("{} ({})", t.translate_selection, prov_name));
    } else {
        sel_btn.on_disabled_hover_text(t.translate_selection_tip);
    }

    let copy_btn = ui
        .add_enabled(can_copy, egui::Button::new(t.copy))
        .on_disabled_hover_text(t.extra.copy_restricted);
    if copy_btn.clicked() && can_copy {
        let sel = selected_text
            .clone()
            .or_else(|| crate::app::selection::get_selected_text(ui.ctx()));
        if let Some(text) = sel {
            ui.ctx().copy_text(text);
        } else {
            ui.ctx().copy_text(single_text.to_string());
        }
        ui.close();
    }

    let copy_section_btn = ui
        .add_enabled(can_copy, egui::Button::new(t.copy_section))
        .on_disabled_hover_text(t.extra.copy_restricted);
    if copy_section_btn.clicked() && can_copy {
        if let Some(full) = crate::core::ReaderCore::extract_chapter_text(doc, current_chapter) {
            ui.ctx().copy_text(full);
        }
        ui.close();
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_empty_reader_welcome(
    ui: &mut egui::Ui,
    theme: &ReaderTheme,
    settings: &AppSettings,
    t: &crate::i18n::Translations,
    on_open_file: &mut bool,
    on_open_folder: &mut bool,
    on_open_opds: &mut bool,
    on_load_recent: &mut Option<PathBuf>,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.add(
            IconLabel::new(Icon::Book, "QuietReader")
                .size(36.0)
                .color(theme.accent_color())
                .strong(),
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new(t.app_subtitle)
                .size(18.0)
                .color(theme.text_color()),
        );
        ui.add_space(30.0);

        ui.horizontal(|ui| {
            let open_btn = IconButton::new(Icon::OpenFile, t.choose_book_btn)
                .font_size(17.0)
                .min_size(Vec2::new(220.0, 46.0));
            if ui
                .add(open_btn)
                .on_hover_text(t.open_file_tooltip)
                .clicked()
            {
                *on_open_file = true;
            }

            let open_dir_btn = IconButton::new(Icon::Folder, t.open_manga_folder_btn)
                .font_size(17.0)
                .min_size(Vec2::new(200.0, 46.0));
            if ui
                .add(open_dir_btn)
                .on_hover_text(t.images_folder_hint)
                .clicked()
            {
                *on_open_folder = true;
            }

            let opds_btn = IconButton::new(Icon::Globe, t.opds_catalogs)
                .font_size(17.0)
                .min_size(Vec2::new(200.0, 46.0));
            if ui
                .add(opds_btn)
                .on_hover_text(t.opds_catalogs_tooltip)
                .clicked()
            {
                *on_open_opds = true;
            }
        });

        ui.add_space(16.0);
        ui.label(
            RichText::new(t.drag_drop_hint)
                .size(14.0)
                .color(Color32::from_gray(140)),
        );

        if !settings.recent_books.is_empty() {
            ui.add_space(28.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(RichText::new(t.recent_books_header).strong().size(16.0));
            ui.add_space(10.0);

            for book in settings.recent_books.iter().take(4) {
                let sec_str = t.section_num.replace("{}", &(book.chapter + 1).to_string());
                let title = if book.title.is_empty() {
                    t.extra.default_untitled
                } else {
                    &book.title
                };
                let label = if book.author.is_empty() {
                    format!("{title} ({sec_str})")
                } else {
                    format!("{} — {title} ({sec_str})", book.author)
                };
                if ui
                    .add(IconButton::new(Icon::Book, &label).font_size(14.0))
                    .clicked()
                {
                    *on_load_recent = Some(book.path.clone());
                }
                ui.add_space(4.0);
            }
        }
    });
}
