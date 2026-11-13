use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatFilterGroup {
    Word,
    Rtf,
    Pdf,
    Djvu,
    Comics,
    Epub,
    Fb2,
    Kindle,
    Text,
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatDescKey {
    Fb2,
    Fb2Zip,
    Epub,
    Mobi,
    Azw3,
    Log,
    Txt,
    Doc,
    Dot,
    Docx,
    Rtf,
    Pdf,
    Djvu,
    Cbz,
    Cbr,
    Cb7,
    Cbt,
    Zip,
    Rar,
    SevenZ,
    Tar,
    Md,
    Html,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatBackend {
    Pdf,
    Djvu,
    Fb2Zip,
    Comic,
    Epub,
    Fb2,
    Docx,
    Rtf,
    Doc,
    Txt,
    Html,
    Mobi,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtensionSpec {
    pub extension: &'static str,
    pub backends: &'static [FormatBackend],
    pub open_by_extension: bool,
    pub filter_group: FormatFilterGroup,
    pub associable: bool,
    pub badge: &'static str,
    pub mime_type: &'static str,
    pub desc_key: FormatDescKey,
    pub supports_encoding: bool,
}

pub const DEFAULT_BOOK_ASSOCIATIONS: &[&str] = &["fb2", "epub"];

pub const DIALOG_FILTER_GROUPS: &[FormatFilterGroup] = &[
    FormatFilterGroup::Word,
    FormatFilterGroup::Rtf,
    FormatFilterGroup::Pdf,
    FormatFilterGroup::Djvu,
    FormatFilterGroup::Comics,
    FormatFilterGroup::Epub,
    FormatFilterGroup::Fb2,
    FormatFilterGroup::Kindle,
    FormatFilterGroup::Text,
    FormatFilterGroup::Html,
];

pub static EXTENSION_SPECS: &[ExtensionSpec] = &[
    ExtensionSpec {
        extension: "fb2",
        backends: &[FormatBackend::Fb2],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Fb2,
        associable: true,
        badge: "FB2",
        mime_type: "application/x-fictionbook+xml",
        desc_key: FormatDescKey::Fb2,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "fb2.zip",
        backends: &[FormatBackend::Fb2Zip],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Fb2,
        associable: false,
        badge: "FB2",
        mime_type: "application/x-fictionbook+xml",
        desc_key: FormatDescKey::Fb2Zip,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "xml",
        backends: &[FormatBackend::Fb2],
        open_by_extension: false,
        filter_group: FormatFilterGroup::Fb2,
        associable: false,
        badge: "FB2",
        mime_type: "application/xml",
        desc_key: FormatDescKey::Xml,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "epub",
        backends: &[FormatBackend::Epub],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Epub,
        associable: true,
        badge: "EPUB",
        mime_type: "application/epub+zip",
        desc_key: FormatDescKey::Epub,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "mobi",
        backends: &[FormatBackend::Mobi],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Kindle,
        associable: true,
        badge: "MOBI",
        mime_type: "application/x-mobipocket-ebook",
        desc_key: FormatDescKey::Mobi,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "prc",
        backends: &[FormatBackend::Mobi],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Kindle,
        associable: false,
        badge: "PRC",
        mime_type: "application/x-mobipocket-ebook",
        desc_key: FormatDescKey::Mobi,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "azw",
        backends: &[FormatBackend::Mobi],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Kindle,
        associable: false,
        badge: "AZW",
        mime_type: "application/x-mobipocket-ebook",
        desc_key: FormatDescKey::Azw3,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "azw3",
        backends: &[FormatBackend::Mobi],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Kindle,
        associable: true,
        badge: "AZW3",
        mime_type: "application/x-mobi8-ebook",
        desc_key: FormatDescKey::Azw3,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "txt",
        backends: &[FormatBackend::Txt],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Text,
        associable: true,
        badge: "TXT",
        mime_type: "text/plain",
        desc_key: FormatDescKey::Txt,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "doc",
        backends: &[FormatBackend::Doc],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Word,
        associable: true,
        badge: "DOC",
        mime_type: "application/msword",
        desc_key: FormatDescKey::Doc,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "dot",
        backends: &[FormatBackend::Doc],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Word,
        associable: false,
        badge: "DOT",
        mime_type: "application/msword",
        desc_key: FormatDescKey::Dot,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "docx",
        backends: &[FormatBackend::Docx],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Word,
        associable: true,
        badge: "DOCX",
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        desc_key: FormatDescKey::Docx,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "rtf",
        backends: &[FormatBackend::Rtf],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Rtf,
        associable: true,
        badge: "RTF",
        mime_type: "application/rtf",
        desc_key: FormatDescKey::Rtf,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "pdf",
        backends: &[FormatBackend::Pdf],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Pdf,
        associable: true,
        badge: "PDF",
        mime_type: "application/pdf",
        desc_key: FormatDescKey::Pdf,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "djvu",
        backends: &[FormatBackend::Djvu],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Djvu,
        associable: true,
        badge: "DJVU",
        mime_type: "image/vnd.djvu",
        desc_key: FormatDescKey::Djvu,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "djv",
        backends: &[FormatBackend::Djvu],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Djvu,
        associable: false,
        badge: "DJVU",
        mime_type: "image/vnd.djvu",
        desc_key: FormatDescKey::Djvu,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "cbz",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: true,
        badge: "CBZ",
        mime_type: "application/vnd.comicbook+zip",
        desc_key: FormatDescKey::Cbz,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "cbr",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: true,
        badge: "CBR",
        mime_type: "application/vnd.comicbook-rar",
        desc_key: FormatDescKey::Cbr,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "cb7",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: true,
        badge: "CB7",
        mime_type: "application/x-cb7",
        desc_key: FormatDescKey::Cb7,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "cbt",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: true,
        badge: "CBT",
        mime_type: "application/vnd.comicbook+tar",
        desc_key: FormatDescKey::Cbt,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "zip",
        backends: &[FormatBackend::Fb2Zip, FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: false,
        badge: "CBZ",
        mime_type: "application/zip",
        desc_key: FormatDescKey::Zip,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "rar",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: false,
        badge: "CBR",
        mime_type: "application/vnd.rar",
        desc_key: FormatDescKey::Rar,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "7z",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: false,
        badge: "CB7",
        mime_type: "application/x-7z-compressed",
        desc_key: FormatDescKey::SevenZ,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "tar",
        backends: &[FormatBackend::Comic],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Comics,
        associable: false,
        badge: "CBT",
        mime_type: "application/x-tar",
        desc_key: FormatDescKey::Tar,
        supports_encoding: false,
    },
    ExtensionSpec {
        extension: "md",
        backends: &[FormatBackend::Txt],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Text,
        associable: true,
        badge: "MD",
        mime_type: "text/markdown",
        desc_key: FormatDescKey::Md,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "markdown",
        backends: &[FormatBackend::Txt],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Text,
        associable: false,
        badge: "MD",
        mime_type: "text/markdown",
        desc_key: FormatDescKey::Md,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "log",
        backends: &[FormatBackend::Txt],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Text,
        associable: false,
        badge: "LOG",
        mime_type: "text/plain",
        desc_key: FormatDescKey::Log,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "html",
        backends: &[FormatBackend::Html],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Html,
        associable: false,
        badge: "HTML",
        mime_type: "text/html",
        desc_key: FormatDescKey::Html,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "htm",
        backends: &[FormatBackend::Html],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Html,
        associable: false,
        badge: "HTML",
        mime_type: "text/html",
        desc_key: FormatDescKey::Html,
        supports_encoding: true,
    },
    ExtensionSpec {
        extension: "xhtml",
        backends: &[FormatBackend::Html],
        open_by_extension: true,
        filter_group: FormatFilterGroup::Html,
        associable: false,
        badge: "HTML",
        mime_type: "application/xhtml+xml",
        desc_key: FormatDescKey::Html,
        supports_encoding: true,
    },
];

pub(super) fn collect_backend_extensions(backend: FormatBackend) -> Vec<&'static str> {
    EXTENSION_SPECS
        .iter()
        .filter(|spec| spec.open_by_extension && spec.backends.contains(&backend))
        .map(|spec| spec.extension)
        .collect()
}

pub fn normalize_extension(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn compound_extension_lower(name_lower: &str) -> Option<&'static str> {
    EXTENSION_SPECS
        .iter()
        .filter(|spec| spec.extension.contains('.'))
        .find(|spec| {
            name_lower
                .strip_suffix(spec.extension)
                .is_some_and(|prefix| prefix.ends_with('.'))
        })
        .map(|spec| spec.extension)
}

pub fn extension_lower(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if let Some(compound) = compound_extension_lower(&name) {
        return compound.to_string();
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn document_file_stem(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    if let Some(compound) = compound_extension_lower(&lower) {
        let strip_len = compound.len() + 1;
        if name.len() > strip_len {
            return name[..name.len() - strip_len].to_string();
        }
    }
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name)
        .to_string()
}

pub fn find_extension_spec(ext: &str) -> Option<&'static ExtensionSpec> {
    let normalized = normalize_extension(ext);
    EXTENSION_SPECS
        .iter()
        .find(|spec| spec.extension == normalized)
}

pub fn all_picker_extensions() -> &'static [&'static str] {
    static EXTS: LazyLock<Vec<&'static str>> =
        LazyLock::new(|| EXTENSION_SPECS.iter().map(|spec| spec.extension).collect());
    &EXTS
}

pub fn filter_extensions(group: FormatFilterGroup) -> &'static [&'static str] {
    fn collect(target: FormatFilterGroup) -> Vec<&'static str> {
        EXTENSION_SPECS
            .iter()
            .filter(|spec| spec.filter_group == target)
            .map(|spec| spec.extension)
            .collect()
    }
    match group {
        FormatFilterGroup::Word => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Word));
            &EXTS
        }
        FormatFilterGroup::Rtf => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Rtf));
            &EXTS
        }
        FormatFilterGroup::Pdf => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Pdf));
            &EXTS
        }
        FormatFilterGroup::Djvu => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Djvu));
            &EXTS
        }
        FormatFilterGroup::Comics => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Comics));
            &EXTS
        }
        FormatFilterGroup::Epub => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Epub));
            &EXTS
        }
        FormatFilterGroup::Fb2 => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Fb2));
            &EXTS
        }
        FormatFilterGroup::Kindle => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Kindle));
            &EXTS
        }
        FormatFilterGroup::Text => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Text));
            &EXTS
        }
        FormatFilterGroup::Html => {
            static EXTS: LazyLock<Vec<&'static str>> =
                LazyLock::new(|| collect(FormatFilterGroup::Html));
            &EXTS
        }
    }
}

pub fn associable_extensions() -> &'static [&'static str] {
    static EXTS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        EXTENSION_SPECS
            .iter()
            .filter(|spec| spec.associable)
            .map(|spec| spec.extension)
            .collect()
    });
    &EXTS
}

pub fn mime_type_for_extension(ext: &str) -> &'static str {
    find_extension_spec(ext)
        .map(|spec| spec.mime_type)
        .unwrap_or("application/octet-stream")
}

pub fn description_key_for_extension(ext: &str) -> Option<FormatDescKey> {
    find_extension_spec(ext).map(|spec| spec.desc_key)
}

pub fn format_badge_for_path(path: &Path) -> String {
    if path.is_dir() {
        return "DIR".to_string();
    }
    let ext = extension_lower(path);
    if let Some(spec) = find_extension_spec(&ext) {
        return spec.badge.to_string();
    }
    if ext.is_empty() {
        "DOC".to_string()
    } else {
        ext.to_ascii_uppercase()
    }
}

pub fn supports_text_encoding(path: &Path) -> bool {
    let ext = extension_lower(path);
    find_extension_spec(&ext).is_some_and(|spec| spec.supports_encoding)
}
