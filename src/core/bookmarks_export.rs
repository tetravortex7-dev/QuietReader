use super::{Bookmark, BookmarkColor, BookmarkKind};

pub fn bookmarks_to_markdown(
    book_title: &str,
    book_author: &str,
    bookmarks: &[Bookmark],
) -> String {
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

    let mut markdown = String::new();
    let title = if book_title.trim().is_empty() {
        "QuietReader Notes"
    } else {
        book_title.trim()
    };
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
