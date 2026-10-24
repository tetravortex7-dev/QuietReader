use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAX_IFO_BYTES: u64 = 1024 * 1024;
const MAX_INDEX_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRY_BYTES: u32 = 4 * 1024 * 1024;
const MAX_INDEX_ENTRIES: usize = 5_000_000;

#[derive(Clone, Debug)]
pub struct InstalledPack {
    pub id: String,
    pub name: String,
    pub wordcount: u64,
    pub folder: PathBuf,
    pub idx: PathBuf,
    pub dict: PathBuf,
    pub sametypesequence: String,
}

#[derive(Clone, Debug)]
pub struct IndexEntry {
    pub word: String,
    pub offset: u64,
    pub size: u32,
}

#[derive(Default)]
pub struct PackLookup {
    pub definitions: Vec<(String, String)>,
    pub suggestions: Vec<String>,
}

pub fn discover(dir: &Path) -> Vec<InstalledPack> {
    let mut packs = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return packs;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let folder = entry.path();
        if let Some(ifo) = find_ifo(&folder, 0) {
            if let Some(pack) = load_pack(&folder, &ifo) {
                packs.push(pack);
            }
        }
    }
    packs.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    packs
}

fn find_ifo(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 3 {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    let mut directories = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("ifo") {
            return Some(path);
        }
        if path.is_dir() {
            directories.push(path);
        }
    }
    for directory in directories {
        if let Some(found) = find_ifo(&directory, depth + 1) {
            return Some(found);
        }
    }
    None
}

fn load_pack(folder: &Path, ifo: &Path) -> Option<InstalledPack> {
    let metadata = fs::metadata(ifo).ok()?;
    if metadata.len() > MAX_IFO_BYTES {
        return None;
    }
    let text = fs::read_to_string(ifo).ok()?;
    let mut name = String::new();
    let mut wordcount = 0u64;
    let mut sametypesequence = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "bookname" => name = value.trim().to_string(),
            "wordcount" => wordcount = value.trim().parse().unwrap_or(0),
            "sametypesequence" => sametypesequence = value.trim().to_string(),
            _ => {}
        }
    }
    let parent = ifo.parent()?;
    let stem = ifo.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
    let idx = parent.join(format!("{stem}.idx"));
    let dict = parent.join(format!("{stem}.dict"));
    if !idx.is_file() || !dict.is_file() {
        return None;
    }
    let id = folder
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(stem)
        .to_string();
    if name.is_empty() {
        name = id.clone();
    }
    Some(InstalledPack {
        id,
        name,
        wordcount,
        folder: folder.to_path_buf(),
        idx,
        dict,
        sametypesequence,
    })
}

pub fn load_index(pack: &InstalledPack) -> Result<Vec<IndexEntry>, String> {
    let metadata = fs::metadata(&pack.idx).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_INDEX_BYTES {
        return Err("dictionary index is too large".to_string());
    }
    let data = fs::read(&pack.idx).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    let mut position = 0usize;
    while position < data.len() && entries.len() < MAX_INDEX_ENTRIES {
        let Some(relative_end) = data[position..].iter().position(|byte| *byte == 0) else {
            break;
        };
        let word_end = position + relative_end;
        let tail = word_end + 1;
        if tail + 8 > data.len() {
            break;
        }
        let word = String::from_utf8_lossy(&data[position..word_end]).to_string();
        let offset =
            u32::from_be_bytes([data[tail], data[tail + 1], data[tail + 2], data[tail + 3]]) as u64;
        let size = u32::from_be_bytes([
            data[tail + 4],
            data[tail + 5],
            data[tail + 6],
            data[tail + 7],
        ]);
        entries.push(IndexEntry { word, offset, size });
        position = tail + 8;
    }
    Ok(entries)
}

pub fn lookup(pack: &InstalledPack, index: &[IndexEntry], query: &str) -> PackLookup {
    let query = normalize_word(query);
    let mut result = PackLookup::default();
    if query.is_empty() {
        return result;
    }
    let mut seen_suggestions = Vec::new();
    for entry in index {
        let word = normalize_word(&entry.word);
        if word == query {
            if result.definitions.len() < 4 {
                if let Ok(text) = read_definition(pack, entry) {
                    if !text.is_empty() {
                        result.definitions.push((entry.word.clone(), text));
                    }
                }
            }
        } else if word.starts_with(&query)
            && result.definitions.is_empty()
            && result.suggestions.len() < 5
            && !seen_suggestions.contains(&entry.word)
        {
            seen_suggestions.push(entry.word.clone());
            result.suggestions.push(entry.word.clone());
        }
    }
    if !result.definitions.is_empty() {
        result.suggestions.clear();
    }
    result
}

fn read_definition(pack: &InstalledPack, entry: &IndexEntry) -> Result<String, String> {
    if entry.size == 0 || entry.size > MAX_ENTRY_BYTES {
        return Err("dictionary entry is too large".to_string());
    }
    let mut file = fs::File::open(&pack.dict).map_err(|error| error.to_string())?;
    file.seek(SeekFrom::Start(entry.offset))
        .map_err(|error| error.to_string())?;
    let mut data = vec![0u8; entry.size as usize];
    file.read_exact(&mut data)
        .map_err(|error| error.to_string())?;
    Ok(parse_definition(&data, &pack.sametypesequence))
}

pub fn parse_definition(data: &[u8], sametypesequence: &str) -> String {
    let mut fields: Vec<(char, &[u8])> = Vec::new();
    if sametypesequence.is_empty() {
        let mut position = 0usize;
        while position < data.len() {
            let kind = data[position] as char;
            position += 1;
            let Some(relative_end) = data[position..].iter().position(|byte| *byte == 0) else {
                fields.push((kind, &data[position..]));
                break;
            };
            let end = position + relative_end;
            fields.push((kind, &data[position..end]));
            position = end + 1;
        }
    } else {
        let mut position = 0usize;
        for kind in sametypesequence.chars() {
            if position > data.len() {
                break;
            }
            let Some(relative_end) = data[position..].iter().position(|byte| *byte == 0) else {
                fields.push((kind, &data[position..]));
                break;
            };
            let end = position + relative_end;
            fields.push((kind, &data[position..end]));
            position = end + 1;
        }
    }

    let mut parts = Vec::new();
    for (kind, bytes) in fields {
        let raw = String::from_utf8_lossy(bytes);
        let text = match kind {
            'h' | 'x' | 'g' => clean_markup(&raw),
            'm' | 'l' | 'w' | 'k' => raw.to_string(),
            't' => format!("/{}/", raw.trim()),
            'y' => raw.to_string(),
            _ => String::new(),
        };
        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_string());
        }
    }
    let joined = parts.join("\n");
    if joined.chars().count() > 12_000 {
        joined.chars().take(12_000).collect()
    } else {
        joined
    }
}

fn clean_markup(text: &str) -> String {
    let mut replaced = String::with_capacity(text.len());
    let lower = text.to_ascii_lowercase();
    let mut position = 0usize;
    while position < text.len() {
        let rest = &lower[position..];
        let newline_tag = [
            "<br>", "<br/>", "<br />", "</p>", "</div>", "</li>", "</tr>",
        ]
        .iter()
        .find(|tag| rest.starts_with(**tag));
        if let Some(tag) = newline_tag {
            replaced.push('\n');
            position += tag.len();
            continue;
        }
        let ch = text[position..].chars().next().unwrap_or(' ');
        replaced.push(ch);
        position += ch.len_utf8();
    }

    let mut without_tags = String::with_capacity(replaced.len());
    let mut inside_tag = false;
    for ch in replaced.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            without_tags.push(ch);
        }
    }

    let unescaped = quick_xml::escape::unescape(&without_tags)
        .map(|value| value.into_owned())
        .unwrap_or(without_tags);
    let unescaped = unescaped.replace("&nbsp;", " ").replace('\u{a0}', " ");
    let mut output = String::with_capacity(unescaped.len());
    let mut last_was_space = false;
    for ch in unescaped.chars() {
        if ch == '\n' {
            output.push('\n');
            last_was_space = false;
        } else if ch.is_whitespace() {
            if !last_was_space {
                output.push(' ');
                last_was_space = true;
            }
        } else {
            output.push(ch);
            last_was_space = false;
        }
    }
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn normalize_word(value: &str) -> String {
    let trimmed = value.trim().trim_matches(|ch: char| {
        !ch.is_alphanumeric() && ch != '-' && ch != '\'' && ch != '\u{2019}'
    });
    trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
