use crate::document::{
    BookChapter, BookMetadata, Document, DocumentViewKind, HasToc, PageContent, TextContent,
    TocItem,
};
use crate::i18n::UiLanguage;
use std::sync::Arc;

pub struct LocalizedDocument {
    inner: Arc<dyn Document>,
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl LocalizedDocument {
    pub fn wrap(inner: Arc<dyn Document>, lang: UiLanguage) -> Arc<dyn Document> {
        let page_based = matches!(
            inner.view_kind(),
            DocumentViewKind::Pdf | DocumentViewKind::Comic
        );
        let metadata = BookMetadata {
            title: localize_text(&inner.metadata().title, lang),
            author: localize_text(&inner.metadata().author, lang),
        };
        let chapters = inner
            .chapters()
            .iter()
            .map(|chapter| BookChapter {
                id: chapter.id.clone(),
                title: localize_text(&chapter.title, lang),
                paragraphs: chapter
                    .paragraphs
                    .iter()
                    .map(|paragraph| localize_paragraph(paragraph, lang, page_based))
                    .collect(),
            })
            .collect();
        let toc = inner
            .toc()
            .iter()
            .map(|item| localize_toc(item, lang))
            .collect();
        Arc::new(Self {
            inner,
            metadata,
            chapters,
            toc,
        })
    }
}

impl Document for LocalizedDocument {
    fn metadata(&self) -> &BookMetadata {
        &self.metadata
    }

    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }

    fn toc(&self) -> &[TocItem] {
        &self.toc
    }

    fn format_name(&self) -> &'static str {
        self.inner.format_name()
    }

    fn has_custom_toc(&self) -> bool {
        self.inner.has_custom_toc()
    }

    fn view_kind(&self) -> DocumentViewKind {
        self.inner.view_kind()
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        self.inner.page_content()
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        self.inner.text_content()
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        self.inner.toc_capability()
    }
}

fn localize_toc(item: &TocItem, lang: UiLanguage) -> TocItem {
    TocItem {
        id: item.id.clone(),
        title: localize_text(&item.title, lang),
        chapter_index: item.chapter_index,
        children: item
            .children
            .iter()
            .map(|child| localize_toc(child, lang))
            .collect(),
    }
}

fn localize_paragraph(paragraph: &str, lang: UiLanguage, page_based: bool) -> String {
    if paragraph == "Документ не содержит текста." || page_based {
        localize_text(paragraph, lang)
    } else {
        paragraph.to_string()
    }
}

fn localize_text(value: &str, lang: UiLanguage) -> String {
    let t = lang.t();
    let text = value.trim();
    match text {
        "Без названия" => return t.extra.default_untitled.to_string(),
        "Неизвестный автор" => return t.extra.default_unknown_author.to_string(),
        "Комикс" => return t.extra.default_comic_title.to_string(),
        "Документ Word" => return t.extra.default_word_document.to_string(),
        "Документ PDF" => return t.extra.default_pdf_document.to_string(),
        "Документ DjVu" => return t.extra.default_djvu_document.to_string(),
        "Документ MOBI" => return t.extra.default_mobi_document.to_string(),
        "Текстовый документ" => return t.extra.default_text_document.to_string(),
        "Документ не содержит текста." => {
            return t.extra.default_no_text.to_string();
        }
        "Раздел" => return t.extra.default_section_plain.to_string(),
        _ => {}
    }

    if let Some(rest) = text.strip_prefix("Всего страниц: ")
        && let Ok(count) = rest.trim().parse::<usize>()
    {
        return crate::i18n::fill(t.extra.comic_total_pages, &[("{n}", &count.to_string())]);
    }

    if let Some(rest) = text.strip_prefix("Страницы ")
        && let Some((range, total)) = rest.split_once(" (всего ")
        && let Some((start, end)) = range.split_once('–')
        && let Some(total) = total.strip_suffix(')')
        && let (Ok(start), Ok(end), Ok(total)) = (
            start.trim().parse::<usize>(),
            end.trim().parse::<usize>(),
            total.trim().parse::<usize>(),
        )
    {
        return crate::i18n::fill(
            t.extra.comic_pages_range,
            &[
                ("{start}", &start.to_string()),
                ("{end}", &end.to_string()),
                ("{count}", &total.to_string()),
            ],
        );
    }

    if let Some(rest) = text.strip_prefix("Глава ")
        && let Some((chapter, part)) = rest.split_once(" (часть ")
        && let Some(part) = part.strip_suffix(')')
        && let (Ok(chapter), Ok(part)) = (
            chapter.trim().parse::<usize>(),
            part.trim().parse::<usize>(),
        )
    {
        return crate::i18n::fill(
            t.extra.default_chapter_part,
            &[
                ("{chapter}", &chapter.to_string()),
                ("{part}", &part.to_string()),
            ],
        );
    }

    if let Some(rest) = text.strip_prefix("Глава ")
        && let Ok(chapter) = rest.trim().parse::<usize>()
    {
        return crate::i18n::fill(t.extra.default_chapter, &[("{n}", &chapter.to_string())]);
    }

    if let Some((title, range)) = text.split_once(" (строки ")
        && let Some(range) = range.strip_suffix(')')
        && let Some((start, end)) = range.split_once('–')
        && let (Ok(start), Ok(end)) = (start.trim().parse::<usize>(), end.trim().parse::<usize>())
    {
        let localized_title = localize_text(title, lang);
        return crate::i18n::fill(
            t.extra.default_lines_range,
            &[
                ("{title}", &localized_title),
                ("{start}", &start.to_string()),
                ("{end}", &end.to_string()),
            ],
        );
    }

    if let Some(rest) = text.strip_prefix("Раздел ")
        && let Some((section, page)) = rest.split_once(" (страница ")
        && let Some(page) = page.strip_suffix(')')
        && let (Ok(section), Ok(page)) = (
            section.trim().parse::<usize>(),
            page.trim().parse::<usize>(),
        )
    {
        return crate::i18n::fill(
            t.extra.default_section_page,
            &[
                ("{section}", &section.to_string()),
                ("{page}", &page.to_string()),
            ],
        );
    }

    if let Some(rest) = text.strip_prefix("Раздел ")
        && let Ok(section) = rest.trim().parse::<usize>()
    {
        return crate::i18n::fill(t.extra.default_section, &[("{n}", &section.to_string())]);
    }

    if let Some(rest) = text.strip_prefix("Страница ")
        && let Ok(page) = rest.trim().parse::<usize>()
    {
        return crate::i18n::fill(t.extra.default_page, &[("{n}", &page.to_string())]);
    }

    value.to_string()
}
