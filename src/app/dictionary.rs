use crate::app::theme::ReaderTheme;
use crate::dictionary::{catalog, install, stardict, wikipedia};
use crate::i18n::UiLanguage;
use eframe::egui::{self, RichText};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

const WIKI_LANGS: [&str; 13] = [
    "en", "ru", "de", "fr", "es", "it", "pt", "zh", "ja", "ar", "tr", "uk", "id",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DictionaryTab {
    Dictionary,
    Wikipedia,
}

pub struct PackResult {
    pub pack: String,
    pub entries: Vec<(String, String)>,
}

enum DownloadMessage {
    Progress(u64, u64),
    Done(Result<PathBuf, String>),
}

enum CatalogMessage {
    Done(Result<Vec<catalog::CatalogEntry>, String>),
}

struct DownloadJob {
    id: String,
    received: u64,
    total: u64,
    rx: Receiver<DownloadMessage>,
}

pub struct DictionaryState {
    pub visible: bool,
    pub catalog_visible: bool,
    pub tab: DictionaryTab,
    pub query: String,
    pub results: Vec<PackResult>,
    pub suggestions: Vec<String>,
    pub lookup_error: Option<String>,
    pub installed: Vec<stardict::InstalledPack>,
    pub packs_dir: PathBuf,
    pub catalog: Vec<catalog::CatalogEntry>,
    pub catalog_filter: String,
    pub catalog_loading: bool,
    pub catalog_error: Option<String>,
    pub wiki_lang: String,
    pub wiki_loading: bool,
    pub wiki_requested: bool,
    pub wiki_article: Option<wikipedia::WikiArticle>,
    pub wiki_error: Option<String>,
    pub download_error: Option<String>,
    pub words_visible: bool,
    pub words_changed: bool,
    pub words_notice: Option<String>,
    indexes: HashMap<PathBuf, Arc<Vec<stardict::IndexEntry>>>,
    catalog_rx: Option<Receiver<CatalogMessage>>,
    download: Option<DownloadJob>,
    wiki_rx: Option<Receiver<Result<wikipedia::WikiArticle, String>>>,
}

impl DictionaryState {
    pub fn new() -> Self {
        let packs_dir = crate::dictionary::data_dir();
        let installed = stardict::discover(&packs_dir);
        let catalog = catalog::load_cached(&packs_dir).unwrap_or_default();
        let wiki_lang = UiLanguage::Auto.resolve().code().to_string();
        Self {
            visible: false,
            catalog_visible: false,
            tab: DictionaryTab::Dictionary,
            query: String::new(),
            results: Vec::new(),
            suggestions: Vec::new(),
            lookup_error: None,
            installed,
            packs_dir,
            catalog,
            catalog_filter: String::new(),
            catalog_loading: false,
            catalog_error: None,
            wiki_lang,
            wiki_loading: false,
            wiki_requested: false,
            wiki_article: None,
            wiki_error: None,
            download_error: None,
            words_visible: false,
            words_changed: false,
            words_notice: None,
            indexes: HashMap::new(),
            catalog_rx: None,
            download: None,
            wiki_rx: None,
        }
    }

    pub fn open_word(&mut self, word: String, lang: UiLanguage, wikipedia: bool) {
        self.query = word;
        self.visible = true;
        self.refresh_installed();
        self.wiki_lang = lang.resolve().code().to_string();
        self.wiki_article = None;
        self.wiki_error = None;
        self.wiki_requested = false;
        self.tab = if wikipedia {
            DictionaryTab::Wikipedia
        } else {
            DictionaryTab::Dictionary
        };
        self.lookup();
        if wikipedia {
            self.request_wiki();
        }
    }

    pub fn lookup(&mut self) {
        let query = self.query.trim().to_string();
        self.results.clear();
        self.suggestions.clear();
        self.lookup_error = None;
        if query.is_empty() {
            return;
        }
        let packs = self.installed.clone();
        for pack in packs {
            let index = if let Some(index) = self.indexes.get(&pack.idx) {
                index.clone()
            } else {
                match stardict::load_index(&pack) {
                    Ok(entries) => {
                        let index = Arc::new(entries);
                        self.indexes.insert(pack.idx.clone(), index.clone());
                        index
                    }
                    Err(error) => {
                        self.lookup_error = Some(error);
                        continue;
                    }
                }
            };
            let found = stardict::lookup(&pack, &index, &query);
            if !found.definitions.is_empty() {
                self.results.push(PackResult {
                    pack: pack.name.clone(),
                    entries: found.definitions,
                });
            }
            for suggestion in found.suggestions {
                if !self.suggestions.contains(&suggestion) && self.suggestions.len() < 8 {
                    self.suggestions.push(suggestion);
                }
            }
        }
    }

    pub fn refresh_installed(&mut self) {
        self.installed = stardict::discover(&self.packs_dir);
        self.indexes
            .retain(|path, _| self.installed.iter().any(|pack| &pack.idx == path));
    }

    pub fn open_catalog(&mut self) {
        self.catalog_visible = true;
        if self.catalog.is_empty() && !self.catalog_loading {
            self.refresh_catalog(false);
        }
    }

    pub fn refresh_catalog(&mut self, force: bool) {
        if self.catalog_loading {
            return;
        }
        let dir = self.packs_dir.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = catalog::load_or_fetch(&dir, force);
            let _ = sender.send(CatalogMessage::Done(result));
        });
        self.catalog_rx = Some(receiver);
        self.catalog_loading = true;
        self.catalog_error = None;
    }

    pub fn start_download(&mut self, entry: catalog::CatalogEntry) {
        if self.download.is_some() {
            return;
        }
        let dir = self.packs_dir.clone();
        let (sender, receiver) = mpsc::channel();
        let progress_sender = sender.clone();
        let id = entry.id.clone();
        std::thread::spawn(move || {
            let mut progress = move |received: u64, total: u64| {
                let _ = progress_sender.send(DownloadMessage::Progress(received, total));
            };
            let result = install::install(&entry, &dir, &mut progress);
            let _ = sender.send(DownloadMessage::Done(result));
        });
        self.download_error = None;
        self.download = Some(DownloadJob {
            id,
            received: 0,
            total: 0,
            rx: receiver,
        });
    }

    pub fn delete_pack(&mut self, id: &str) {
        let dir = self.packs_dir.join(id);
        let _ = std::fs::remove_dir_all(dir);
        self.refresh_installed();
    }

    pub fn request_wiki(&mut self) {
        let code = self.wiki_lang.clone();
        self.request_wiki_code(code);
    }

    pub fn request_wiki_code(&mut self, code: String) {
        let query = self.query.trim().to_string();
        if query.is_empty() {
            return;
        }
        self.wiki_lang = code.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(wikipedia::fetch(&code, &query));
        });
        self.wiki_rx = Some(receiver);
        self.wiki_loading = true;
        self.wiki_requested = true;
        self.wiki_article = None;
        self.wiki_error = None;
    }

    pub fn poll(&mut self) {
        let catalog_message = self
            .catalog_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(CatalogMessage::Done(result)) = catalog_message {
            self.catalog_rx = None;
            self.catalog_loading = false;
            match result {
                Ok(entries) => {
                    self.catalog = entries;
                    self.catalog_error = None;
                }
                Err(error) => self.catalog_error = Some(error),
            }
        }

        let mut download_messages = Vec::new();
        if let Some(job) = &self.download {
            while let Ok(message) = job.rx.try_recv() {
                download_messages.push(message);
            }
        }
        for message in download_messages {
            match message {
                DownloadMessage::Progress(received, total) => {
                    if let Some(job) = &mut self.download {
                        job.received = received;
                        job.total = total;
                    }
                }
                DownloadMessage::Done(result) => match result {
                    Ok(_) => {
                        self.download = None;
                        self.download_error = None;
                        self.refresh_installed();
                        if !self.query.trim().is_empty() {
                            self.lookup();
                        }
                    }
                    Err(error) => {
                        self.download = None;
                        self.download_error = Some(error);
                    }
                },
            }
        }

        let wiki_message = self
            .wiki_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = wiki_message {
            self.wiki_rx = None;
            self.wiki_loading = false;
            match result {
                Ok(article) => {
                    self.wiki_article = Some(article);
                    self.wiki_error = None;
                }
                Err(error) => self.wiki_error = Some(error),
            }
        }
    }

    fn download_progress(&self, id: &str) -> Option<f32> {
        let job = self.download.as_ref()?;
        if job.id != id {
            return None;
        }
        if job.total == 0 {
            Some(0.0)
        } else {
            Some((job.received as f32 / job.total as f32).clamp(0.0, 1.0))
        }
    }
}

pub struct DictionaryView;

impl DictionaryView {
    pub fn render(
        state: &mut DictionaryState,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        lang: UiLanguage,
        settings: &mut crate::settings::AppSettings,
    ) {
        state.poll();
        let t = lang.t();
        let mut save_request: Option<(String, String, String)> = None;
        if state.visible {
            let mut open = state.visible;
            egui::Window::new(t.dictionary_title)
                .open(&mut open)
                .resizable(true)
                .default_width(470.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                state.tab == DictionaryTab::Dictionary,
                                t.dictionary_title,
                            )
                            .clicked()
                        {
                            state.tab = DictionaryTab::Dictionary;
                        }
                        if ui
                            .selectable_label(
                                state.tab == DictionaryTab::Wikipedia,
                                t.wikipedia_tab,
                            )
                            .clicked()
                        {
                            state.tab = DictionaryTab::Wikipedia;
                            if !state.wiki_requested && !state.wiki_loading {
                                state.request_wiki();
                            }
                        }
                    });
                    ui.add_space(4.0);
                    let mut search = false;
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 80.0).max(120.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut state.query)
                                .hint_text(t.search_placeholder)
                                .desired_width(width),
                        );
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            search = true;
                        }
                        if ui.button(t.dictionary_search).clicked() {
                            search = true;
                        }
                    });
                    if search {
                        state.lookup();
                        if state.tab == DictionaryTab::Wikipedia {
                            state.request_wiki();
                        }
                    }
                    ui.separator();
                    match state.tab {
                        DictionaryTab::Dictionary => {
                            save_request = render_dictionary_tab(state, ui, theme, &t);
                        }
                        DictionaryTab::Wikipedia => render_wikipedia_tab(state, ui, theme, &t),
                    }
                });
            state.visible = open;
        }

        if let Some((word, definition, pack)) = save_request {
            let added = settings.save_word(crate::settings::SavedWord {
                word,
                definition,
                pack,
                language: lang.resolve().code().to_string(),
                book: String::new(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0),
            });
            if added {
                state.words_changed = true;
                state.words_notice = Some(t.dictionary_word_saved.to_string());
            }
        }

        if state.words_visible {
            render_words(state, ctx, theme, &t, settings);
        }

        if state.catalog_visible {
            render_catalog(state, ctx, theme, &t);
        }
    }
}

fn render_dictionary_tab(
    state: &mut DictionaryState,
    ui: &mut egui::Ui,
    theme: &ReaderTheme,
    t: &crate::i18n::Translations,
) -> Option<(String, String, String)> {
    let mut save_request = None;
    ui.horizontal(|ui| {
        if ui.button(t.dictionary_download).clicked() {
            state.open_catalog();
        }
        if ui.button(t.vocabulary_open).clicked() {
            state.words_visible = true;
        }
    });
    if let Some(notice) = state.words_notice.as_deref() {
        ui.label(RichText::new(notice).small().color(theme.accent_color()));
    }
    if state.installed.is_empty() {
        ui.label(t.dictionary_no_packs);
        return save_request;
    }
    if state.lookup_error.is_some() {
        ui.label(RichText::new(t.dictionary_error).color(theme.accent_color()));
        if let Some(error) = state.lookup_error.as_deref() {
            ui.label(
                RichText::new(error)
                    .small()
                    .color(theme.text_color().gamma_multiply(0.7)),
            );
        }
    }
    if !state.suggestions.is_empty() {
        let suggestions = state.suggestions.clone();
        ui.horizontal_wrapped(|ui| {
            for suggestion in suggestions {
                if ui.button(&suggestion).clicked() {
                    state.query = suggestion;
                    state.lookup();
                }
            }
        });
    }
    egui::ScrollArea::vertical()
        .max_height(340.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if state.results.is_empty() {
                if state.lookup_error.is_none() && !state.query.trim().is_empty() {
                    ui.label(t.dictionary_not_found);
                }
                return;
            }
            for pack in &state.results {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&pack.pack)
                            .small()
                            .strong()
                            .color(theme.accent_color()),
                    );
                    if let Some((word, definition)) = pack.entries.first()
                        && ui.small_button(t.dictionary_save_word).clicked()
                    {
                        save_request = Some((word.clone(), definition.clone(), pack.pack.clone()));
                    }
                });
                for (word, definition) in &pack.entries {
                    if word.to_lowercase() != state.query.trim().to_lowercase() {
                        ui.label(RichText::new(word).small().italics());
                    }
                    ui.label(RichText::new(definition).color(theme.text_color()));
                    ui.separator();
                }
            }
        });
    save_request
}

fn render_wikipedia_tab(
    state: &mut DictionaryState,
    ui: &mut egui::Ui,
    theme: &ReaderTheme,
    t: &crate::i18n::Translations,
) {
    ui.horizontal(|ui| {
        ui.label(t.dictionary_wiki_language);
        let current = state.wiki_lang.clone();
        egui::ComboBox::from_id_salt("quietreader_wiki_lang")
            .selected_text(current.clone())
            .show_ui(ui, |ui| {
                for code in WIKI_LANGS {
                    if ui.selectable_label(current == code, code).clicked() {
                        state.request_wiki_code(code.to_string());
                    }
                }
            });
        if ui.button(t.dictionary_search).clicked() {
            state.request_wiki();
        }
    });
    if state.wiki_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t.dictionary_loading);
        });
        return;
    }
    if let Some(article) = &state.wiki_article {
        ui.heading(&article.title);
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(RichText::new(&article.extract).color(theme.text_color()));
            });
        if !article.url.is_empty() && ui.button(t.wikipedia_open).clicked() {
            crate::platform::open_url(&article.url);
        }
        return;
    }
    if state.wiki_error.is_some() || state.wiki_requested {
        ui.label(t.wikipedia_not_found);
    }
}

fn render_catalog(
    state: &mut DictionaryState,
    ctx: &egui::Context,
    theme: &ReaderTheme,
    t: &crate::i18n::Translations,
) {
    let mut open = state.catalog_visible;
    let mut delete_id = None;
    let mut start_download = None;
    egui::Window::new(t.dictionary_catalog)
        .open(&mut open)
        .resizable(true)
        .default_width(580.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(t.dictionary_refresh).clicked() {
                    state.refresh_catalog(true);
                }
                if state.catalog_loading {
                    ui.spinner();
                    ui.label(t.dictionary_loading);
                }
            });
            if let Some(error) = state
                .catalog_error
                .as_deref()
                .or(state.download_error.as_deref())
            {
                ui.label(RichText::new(t.dictionary_error).color(theme.accent_color()));
                ui.label(
                    RichText::new(error)
                        .small()
                        .color(theme.text_color().gamma_multiply(0.7)),
                );
            }
            ui.add(
                egui::TextEdit::singleline(&mut state.catalog_filter)
                    .hint_text(t.search_placeholder)
                    .desired_width(f32::INFINITY),
            );
            let filter = state.catalog_filter.trim().to_lowercase();
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for entry in &state.catalog {
                        if !filter.is_empty()
                            && !entry.id.to_lowercase().contains(&filter)
                            && !entry.headwords.to_string().contains(&filter)
                        {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            let installed = state.installed.iter().any(|pack| pack.id == entry.id);
                            ui.label(RichText::new(&entry.id).strong().color(theme.text_color()));
                            ui.label(
                                RichText::new(format!(
                                    "{} · {:.1} MB",
                                    entry.headwords,
                                    entry.size_mb()
                                ))
                                .small()
                                .color(theme.text_color().gamma_multiply(0.7)),
                            );
                            if installed {
                                if ui.button(t.dictionary_delete).clicked() {
                                    delete_id = Some(entry.id.clone());
                                }
                            } else if let Some(progress) = state.download_progress(&entry.id) {
                                ui.add(
                                    egui::ProgressBar::new(progress)
                                        .desired_width(140.0)
                                        .show_percentage(),
                                );
                            } else if state.download.is_none()
                                && ui.button(t.dictionary_download_pack).clicked()
                            {
                                start_download = Some(entry.clone());
                            }
                        });
                    }
                });
            ui.separator();
            ui.label(
                RichText::new(t.dictionary_source_note)
                    .small()
                    .color(theme.text_color().gamma_multiply(0.65)),
            );
        });
    state.catalog_visible = open;
    if let Some(id) = delete_id {
        state.delete_pack(&id);
    }
    if let Some(entry) = start_download {
        state.start_download(entry);
    }
}

fn render_words(
    state: &mut DictionaryState,
    ctx: &egui::Context,
    theme: &ReaderTheme,
    t: &crate::i18n::Translations,
    settings: &mut crate::settings::AppSettings,
) {
    let mut open = state.words_visible;
    let mut delete: Option<usize> = None;
    let mut export = false;
    egui::Window::new(t.vocabulary_title)
        .open(&mut open)
        .resizable(true)
        .default_width(520.0)
        .show(ctx, |ui| {
            if settings.saved_words.is_empty() {
                ui.label(t.vocabulary_empty);
            } else {
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (index, entry) in settings.saved_words.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&entry.word)
                                        .strong()
                                        .color(theme.text_color()),
                                );
                                if !entry.pack.is_empty() {
                                    ui.label(
                                        RichText::new(&entry.pack)
                                            .small()
                                            .color(theme.text_color().gamma_multiply(0.65)),
                                    );
                                }
                                if ui.small_button(t.vocabulary_delete).clicked() {
                                    delete = Some(index);
                                }
                            });
                            ui.label(RichText::new(&entry.definition).color(theme.text_color()));
                            ui.separator();
                        }
                    });
                ui.add_space(4.0);
                if ui.button(t.vocabulary_export).clicked() {
                    export = true;
                }
            }
            if let Some(notice) = state.words_notice.as_deref() {
                ui.label(RichText::new(notice).small().color(theme.accent_color()));
            }
        });
    state.words_visible = open;
    if let Some(index) = delete
        && index < settings.saved_words.len()
    {
        settings.saved_words.remove(index);
        state.words_changed = true;
    }
    if export
        && let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("quietreader_words.csv")
            .save_file()
        && std::fs::write(&path, words_to_csv(&settings.saved_words)).is_ok()
    {
        state.words_notice = Some(t.vocabulary_exported.to_string());
    }
}

pub(crate) fn words_to_csv(words: &[crate::settings::SavedWord]) -> String {
    let mut csv = String::from("Word,Definition,Pack,Language,Book\n");
    for word in words {
        let fields = [
            &word.word,
            &word.definition,
            &word.pack,
            &word.language,
            &word.book,
        ];
        let row: Vec<String> = fields.iter().map(|value| csv_field(value)).collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }
    csv
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
