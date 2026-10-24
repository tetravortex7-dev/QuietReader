use super::{BodyNode, SectionNode};
use crate::document::strip_formatting_tags;
use std::collections::HashMap;

pub(super) fn is_notes_body(name: Option<&str>) -> bool {
    matches!(
        name.unwrap_or_default().to_ascii_lowercase().as_str(),
        "notes" | "comments"
    )
}

pub(super) fn collect_body(
    body: &BodyNode,
    notes: &mut HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut ordered = Vec::new();
    for section in &body.sections {
        collect_section(section, notes, &mut ordered);
    }
    ordered
}

fn collect_section(
    section: &SectionNode,
    notes: &mut HashMap<String, String>,
    ordered: &mut Vec<(String, String)>,
) {
    if let Some(id) = &section.id {
        let mut paragraphs: Vec<String> = section
            .paragraphs
            .iter()
            .map(|paragraph| normalize_note_text(&strip_formatting_tags(paragraph)))
            .filter(|paragraph| !paragraph.is_empty())
            .collect();
        if paragraphs.is_empty()
            && let Some(title) = &section.title
        {
            let title = normalize_note_text(&strip_formatting_tags(title));
            if !title.is_empty() {
                paragraphs.push(title);
            }
        }
        if !paragraphs.is_empty() {
            let text = paragraphs.join("\n");
            notes.insert(id.clone(), text.clone());
            ordered.push((id.clone(), text));
        }
    }
    for subsection in &section.subsections {
        collect_section(subsection, notes, ordered);
    }
}

fn normalize_note_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
