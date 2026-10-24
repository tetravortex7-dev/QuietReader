use super::{Bookmark, BookmarkColor, BookmarkKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NotesFormat {
    #[default]
    Markdown,
    Csv,
    Html,
    Text,
}

impl NotesFormat {
    pub fn extension(self) -> &'static str {
        match self {
            NotesFormat::Markdown => "md",
            NotesFormat::Csv => "csv",
            NotesFormat::Html => "html",
            NotesFormat::Text => "txt",
        }
    }

    pub fn render(self, book_title: &str, book_author: &str, bookmarks: &[Bookmark]) -> String {
        match self {
            NotesFormat::Markdown => bookmarks_to_markdown(book_title, book_author, bookmarks),
            NotesFormat::Csv => bookmarks_to_csv(book_title, book_author, bookmarks),
            NotesFormat::Html => bookmarks_to_html(book_title, book_author, bookmarks),
            NotesFormat::Text => bookmarks_to_text(book_title, book_author, bookmarks),
        }
    }
}

fn sorted_bookmarks(bookmarks: &[Bookmark]) -> Vec<Bookmark> {
    let mut sorted = bookmarks.to_vec();
    sorted.sort_by(|a, b| {
        a.chapter
            .cmp(&b.chapter)
            .then(
                a.scroll_y
                    .partial_cmp(&b.scroll_y)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.timestamp.cmp(&b.timestamp))
    });
    sorted
}

fn display_title(book_title: &str) -> &str {
    if book_title.trim().is_empty() {
        "QuietReader Notes"
    } else {
        book_title.trim()
    }
}

pub fn bookmarks_to_markdown(
    book_title: &str,
    book_author: &str,
    bookmarks: &[Bookmark],
) -> String {
    let sorted = sorted_bookmarks(bookmarks);
    let mut markdown = String::new();
    let title = display_title(book_title);
    markdown.push_str(&format!("# {title}\n\n"));
    if !book_author.trim().is_empty() {
        markdown.push_str(&format!("**Author:** {}\n\n", book_author.trim()));
    }
    markdown.push_str("---\n\n");

    let mut current_chapter: Option<(usize, String)> = None;
    for bookmark in &sorted {
        let chapter = chapter_title(bookmark);
        if current_chapter.as_ref() != Some(&(bookmark.chapter, chapter.clone())) {
            markdown.push_str(&format!("## {chapter}\n\n"));
            current_chapter = Some((bookmark.chapter, chapter.clone()));
        }

        let color = color_label(bookmark.color);
        markdown.push_str(&format!(
            "### {color} — {:.1}%\n\n",
            bookmark.progress_percent
        ));
        markdown.push_str(&format!("- **Color:** {color}\n"));
        markdown.push_str(&format!("- **Chapter:** {chapter}\n"));
        markdown.push_str(&format!(
            "- **Progress:** {:.1}%\n",
            bookmark.progress_percent
        ));
        markdown.push_str(&format!("- **Kind:** {}\n\n", kind_label(bookmark.kind)));

        if !bookmark.text.trim().is_empty() {
            for line in bookmark.text.lines() {
                markdown.push_str(&format!("> {line}\n"));
            }
            markdown.push('\n');
        }

        if !bookmark.note.trim().is_empty() {
            markdown.push_str(&format!("**Note:** {}\n\n", bookmark.note.trim()));
        }

        markdown.push_str("---\n\n");
    }

    markdown
}

pub fn bookmarks_to_csv(book_title: &str, book_author: &str, bookmarks: &[Bookmark]) -> String {
    let title = display_title(book_title);
    let author = book_author.trim();
    let mut csv = String::from("Book,Author,Chapter,Color,Kind,Progress %,Text,Note\n");
    for bookmark in sorted_bookmarks(bookmarks) {
        let fields = [
            title.to_string(),
            author.to_string(),
            chapter_title(&bookmark),
            color_label(bookmark.color).to_string(),
            kind_label(bookmark.kind).to_string(),
            format!("{:.1}", bookmark.progress_percent),
            bookmark.text.trim().to_string(),
            bookmark.note.trim().to_string(),
        ];
        let row: Vec<String> = fields.iter().map(|field| csv_field(field)).collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }
    csv
}

pub fn bookmarks_to_html(book_title: &str, book_author: &str, bookmarks: &[Bookmark]) -> String {
    let title = display_title(book_title);
    let author = book_author.trim();
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    html.push_str(
        "<style>body{font-family:Georgia,serif;max-width:46rem;margin:2rem auto;padding:0 1rem;line-height:1.6;color:#222}h1{margin-bottom:.2rem}.author{color:#666;margin-top:0}section{margin-top:2rem}h2{border-bottom:1px solid #ddd;padding-bottom:.3rem}.meta{color:#666;font-size:.9rem;margin-bottom:.4rem}blockquote{margin:.4rem 0;padding-left:1rem;border-left:3px solid #ccc}blockquote p{margin:.3rem 0}.note{background:#f6f6f6;padding:.5rem .75rem;border-radius:4px}</style>\n</head>\n<body>\n",
    );
    html.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));
    if !author.is_empty() {
        html.push_str(&format!(
            "<p class=\"author\">{}</p>\n",
            escape_html(author)
        ));
    }

    let mut current_chapter: Option<(usize, String)> = None;
    for bookmark in &sorted_bookmarks(bookmarks) {
        let chapter = chapter_title(bookmark);
        if current_chapter.as_ref() != Some(&(bookmark.chapter, chapter.clone())) {
            if current_chapter.is_some() {
                html.push_str("</section>\n");
            }
            html.push_str(&format!("<section>\n<h2>{}</h2>\n", escape_html(&chapter)));
            current_chapter = Some((bookmark.chapter, chapter.clone()));
        }
        html.push_str(&format!(
            "<article>\n<p class=\"meta\">{} · {} · {:.1}%</p>\n",
            color_label(bookmark.color),
            kind_label(bookmark.kind),
            bookmark.progress_percent
        ));
        if !bookmark.text.trim().is_empty() {
            html.push_str("<blockquote>\n");
            for line in bookmark.text.trim().lines() {
                html.push_str(&format!("<p>{}</p>\n", escape_html(line)));
            }
            html.push_str("</blockquote>\n");
        }
        if !bookmark.note.trim().is_empty() {
            html.push_str(&format!(
                "<p class=\"note\">{}</p>\n",
                escape_html(bookmark.note.trim())
            ));
        }
        html.push_str("</article>\n");
    }
    if current_chapter.is_some() {
        html.push_str("</section>\n");
    }
    html.push_str("</body>\n</html>\n");
    html
}

pub fn bookmarks_to_text(book_title: &str, book_author: &str, bookmarks: &[Bookmark]) -> String {
    let title = display_title(book_title);
    let author = book_author.trim();
    let mut text = String::new();
    text.push_str(title);
    text.push('\n');
    if !author.is_empty() {
        text.push_str(&format!("Author: {author}\n"));
    }
    text.push_str(&"=".repeat(title.chars().count().max(8)));
    text.push_str("\n\n");

    let mut current_chapter: Option<(usize, String)> = None;
    for bookmark in &sorted_bookmarks(bookmarks) {
        let chapter = chapter_title(bookmark);
        if current_chapter.as_ref() != Some(&(bookmark.chapter, chapter.clone())) {
            text.push_str(&format!("{chapter}\n"));
            text.push_str(&"-".repeat(chapter.chars().count().max(4)));
            text.push_str("\n\n");
            current_chapter = Some((bookmark.chapter, chapter.clone()));
        }
        text.push_str(&format!(
            "{} · {} · {:.1}%\n",
            color_label(bookmark.color),
            kind_label(bookmark.kind),
            bookmark.progress_percent
        ));
        if !bookmark.text.trim().is_empty() {
            for line in bookmark.text.trim().lines() {
                text.push_str(&format!("> {line}\n"));
            }
            text.push('\n');
        }
        if !bookmark.note.trim().is_empty() {
            text.push_str(&format!("Note: {}\n", bookmark.note.trim()));
        }
        text.push('\n');
    }
    text
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn color_label(color: BookmarkColor) -> &'static str {
    match color {
        BookmarkColor::Yellow => "Yellow",
        BookmarkColor::Green => "Green",
        BookmarkColor::Blue => "Blue",
        BookmarkColor::Red => "Red",
    }
}

fn kind_label(kind: BookmarkKind) -> &'static str {
    match kind {
        BookmarkKind::Bookmark => "Bookmark",
        BookmarkKind::Quote => "Quote",
    }
}

fn chapter_title(bookmark: &Bookmark) -> String {
    let label = bookmark.label.trim();
    if label.is_empty() {
        format!("Chapter {}", bookmark.chapter + 1)
    } else {
        label.to_string()
    }
}
