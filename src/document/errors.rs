const PREFIX: char = '\x1e';
const SEP: char = '\x1f';
const SUB_PREFIX: char = '\x1c';
const SUB_SEP: char = '\x1d';

fn sanitize(s: &str) -> String {
    if s.contains([PREFIX, SEP, SUB_PREFIX, SUB_SEP]) {
        s.replace([PREFIX, SEP, SUB_PREFIX, SUB_SEP], " ")
    } else {
        s.to_string()
    }
}

fn encode_nested_arg(value: &str) -> String {
    if let Some((code, args)) = decode(value) {
        let mut out = String::with_capacity(value.len());
        out.push(SUB_PREFIX);
        out.push_str(code);
        for arg in args {
            out.push(SUB_SEP);
            out.push_str(&sanitize(arg));
        }
        out
    } else {
        sanitize(value)
    }
}

fn encode_with_nested_first(code: &str, first: &str, rest_args: &[&str]) -> String {
    let mut out = String::with_capacity(code.len() + first.len() + rest_args.len() * 12 + 4);
    out.push(PREFIX);
    out.push_str(code);
    out.push(SEP);
    out.push_str(&encode_nested_arg(first));
    for arg in rest_args {
        out.push(SEP);
        out.push_str(&sanitize(arg));
    }
    out
}

pub fn encode(code: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(code.len() + args.len() * 12 + 2);
    out.push(PREFIX);
    out.push_str(code);
    for arg in args {
        out.push(SEP);
        out.push_str(&sanitize(arg));
    }
    out
}

fn decode_with(raw: &str, prefix: char, sep: char) -> Option<(&str, Vec<&str>)> {
    let rest = raw.strip_prefix(prefix)?;
    let mut parts = rest.split(sep);
    let code = parts.next()?;
    Some((code, parts.collect()))
}

pub fn decode(raw: &str) -> Option<(&str, Vec<&str>)> {
    decode_with(raw, PREFIX, SEP)
}

pub fn decode_nested(raw: &str) -> Option<(&str, Vec<&str>)> {
    decode_with(raw, SUB_PREFIX, SUB_SEP)
}

pub fn fallback_display(raw: &str) -> String {
    let Some((code, args)) = decode(raw) else {
        return raw.to_string();
    };
    let a = |i: usize| -> &str { args.get(i).copied().unwrap_or("") };
    match code {
        "err:io_open" => format!("Failed to open {}: {}", a(0), a(1)),
        "err:io_read" => format!("Failed to read {}: {}", a(0), a(1)),
        "err:io_meta" => format!("Failed to read metadata for {}: {}", a(0), a(1)),
        "err:not_found" => format!("File or folder not found: {}", a(0)),
        "err:arch_io" => format!("Archive read error: {}", a(0)),
        "err:cache_dir" => format!("Failed to create comic cache directory: {}", a(0)),
        "err:cache_file" => format!("Failed to create page cache file: {}", a(0)),
        "err:cache_perm" => format!("Failed to restrict page cache permissions: {}", a(0)),
        "err:cache_save" => format!("Failed to save page cache: {}", a(0)),
        "err:file_limit" => format!("{} file exceeds size limit", a(0)),
        "err:file_limit_mb" => format!("{} file exceeds {} MB size limit", a(0), a(1)),
        "err:data_limit" => format!("{} data exceeds size limit", a(0)),
        "err:data_limit_mb" => format!("{} data exceeds {} MB size limit", a(0), a(1)),
        "err:decomp_limit_mb" => format!("Decompressed {} text exceeds {} MB limit", a(0), a(1)),
        "err:arch_parse" => format!("Failed to parse {} archive: {}", a(0), a(1)),
        "err:arch_read" => format!("Failed to read {} archive: {}", a(0), a(1)),
        "err:arch_many" => format!("{} archive exceeds file count limit ({})", a(0), a(1)),
        "err:entry_limit" => format!("File '{}' exceeds size limit", a(0)),
        "err:entry_limit_mb" => format!("File '{}' exceeds {} MB limit", a(0), a(1)),
        "err:entry_limit_bytes" => format!("Archive entry '{}' exceeds {} bytes limit", a(0), a(1)),
        "err:total_limit" => format!("{} unpacked archive exceeds total limit", a(0)),
        "err:total_limit_mb" => {
            format!("Unpacked {} archive exceeds {} MB total limit", a(0), a(1))
        }
        "err:total_limit_bytes" => format!("Unpacked archive exceeds {} bytes total limit", a(0)),
        "err:arch_unsafe" => format!("Unsafe path in {} archive: {}", a(0), a(1)),
        "err:unsafe_path" => "Unsafe path".to_string(),
        "err:entry_missing" => format!("File '{}' not found in {}", a(1), a(0)),
        "err:entry_err" => format!("Error reading '{}': {}", a(0), a(1)),
        "err:entry_read" => format!("Failed to read '{}': {}", a(0), a(1)),
        "err:cache_unavail" => format!("{} cache unavailable", a(0)),
        "err:fb2_zip_missing" => "No FB2 file found in ZIP archive".to_string(),
        "err:fb2_no_root" => "Missing root <FictionBook> element in FB2 document".to_string(),
        "err:fb2_extract" => "Failed to extract content from corrupted FB2 document".to_string(),
        "err:epub_no_opf" => "Rootfile (OPF) not found".to_string(),
        "err:epub_opf" => format!("Failed to parse OPF: {}", a(0)),
        "err:epub_container" => format!("Failed to parse container.xml: {}", a(0)),
        "err:epub_empty_spine" => "EPUB spine contains no chapters".to_string(),
        "err:comic_xml" => format!("Failed to parse ComicInfo.xml: {}", a(0)),
        "err:comic_unsup" => format!("Unsupported comic format: {}", a(0)),
        "err:comic_no_imgs" => "No supported images found in archive or folder".to_string(),
        "err:unsup_fmt" => format!("Unsupported file format: {}", a(0)),
        "err:short_file" => format!("File is too short for {} format", a(0)),
        "err:bad_header" => format!("Invalid {} header ({})", a(0), a(1)),
        "err:file_empty" => format!("Invalid {} document (file is empty)", a(0)),
        "err:bad_sector" => format!("Invalid sector size in {} file", a(0)),
        "err:parse_failed" => format!("Failed to parse {} document: {}", a(0), a(1)),
        "err:parse_corrupt" => format!("Failed to parse {} document: corrupted file", a(0)),
        "err:decoder_panic" => format!("Decoder panic while parsing {}", a(0)),
        "err:no_pages" => format!("{} document contains no pages", a(0)),
        "err:no_read_text" => format!("No readable text found in {} document", a(0)),
        "err:empty_doc" => format!("{} document contains no text", a(0)),
        "err:binary_file" => "File contains binary data and is not a text document".to_string(),
        "err:extract_chaps" => "Failed to extract chapters from book".to_string(),
        "err:page_oob" => format!("Page {} is out of bounds", a(0)),
        "err:page_missing" => format!("Page {} not found", a(0)),
        "err:page_get" => format!("Failed to load page {}: {}", a(0), a(1)),
        "err:page_bad_dims" => format!("Page {} has invalid dimensions (0x0)", a(0)),
        "err:page_zero" => format!("Page {} has zero size", a(0)),
        "err:page_render" => format!("Failed to render {} page {}: {}", a(0), a(1), a(2)),
        "err:page_buf" => format!(
            "Invalid buffer size for page {}: expected {}, got {}",
            a(0),
            a(1),
            a(2)
        ),
        "err:page_panic" => format!("Decoder panic while rendering page {}", a(0)),
        "err:img_missing" => "Image not found".to_string(),
        "err:img_empty" => "Empty image data".to_string(),
        "err:img_fmt" => format!("Image format error: {}", a(0)),
        "err:img_zero" => "Image has zero size".to_string(),
        "err:img_limit" => format!("Image dimensions ({}x{}) exceed limit", a(0), a(1)),
        "err:img_decode" => format!("Failed to decode image: {}", a(0)),
        _ => raw.to_string(),
    }
}

pub fn io_open(target: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:io_open", &[target, &err.to_string()])
}

pub fn io_read(target: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:io_read", &[target, &err.to_string()])
}

pub fn io_metadata(fmt: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:io_meta", &[fmt, &err.to_string()])
}

pub fn io_not_found(path: &str) -> String {
    encode("err:not_found", &[path])
}

pub fn io_archive_read(err: &dyn std::fmt::Display) -> String {
    encode("err:arch_io", &[&err.to_string()])
}

pub fn cache_dir_create(err: &dyn std::fmt::Display) -> String {
    encode("err:cache_dir", &[&err.to_string()])
}

pub fn cache_file_create(err: &dyn std::fmt::Display) -> String {
    encode("err:cache_file", &[&err.to_string()])
}

#[cfg_attr(not(unix), allow(dead_code))]
pub fn cache_permissions(err: &dyn std::fmt::Display) -> String {
    encode("err:cache_perm", &[&err.to_string()])
}

pub fn cache_save_page(err: &dyn std::fmt::Display) -> String {
    encode("err:cache_save", &[&err.to_string()])
}

pub fn file_too_large(fmt: &str) -> String {
    encode("err:file_limit", &[fmt])
}

pub fn file_too_large_mb(fmt: &str, mb: u64) -> String {
    encode("err:file_limit_mb", &[fmt, &mb.to_string()])
}

pub fn data_too_large(fmt: &str) -> String {
    encode("err:data_limit", &[fmt])
}

pub fn data_too_large_mb(fmt: &str, mb: u64) -> String {
    encode("err:data_limit_mb", &[fmt, &mb.to_string()])
}

pub fn decompressed_too_large_mb(fmt: &str, mb: u64) -> String {
    encode("err:decomp_limit_mb", &[fmt, &mb.to_string()])
}

pub fn archive_parse(fmt: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:arch_parse", &[fmt, &err.to_string()])
}

pub fn archive_read(fmt: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:arch_read", &[fmt, &err.to_string()])
}

pub fn archive_too_many_files(fmt: &str, limit: usize) -> String {
    encode("err:arch_many", &[fmt, &limit.to_string()])
}

pub fn archive_entry_too_large(entry: &str) -> String {
    encode("err:entry_limit", &[entry])
}

pub fn archive_entry_too_large_mb(entry: &str, mb: u64) -> String {
    encode("err:entry_limit_mb", &[entry, &mb.to_string()])
}

pub fn archive_entry_too_large_bytes(bytes: u64) -> String {
    encode("err:entry_limit_bytes", &["", &bytes.to_string()])
}

pub fn archive_total_too_large(fmt: &str) -> String {
    encode("err:total_limit", &[fmt])
}

pub fn archive_total_too_large_mb(fmt: &str, mb: u64) -> String {
    encode("err:total_limit_mb", &[fmt, &mb.to_string()])
}

pub fn archive_total_too_large_bytes(bytes: u64) -> String {
    encode("err:total_limit_bytes", &[&bytes.to_string()])
}

pub fn archive_unsafe_path(fmt: &str, entry: &str) -> String {
    encode("err:arch_unsafe", &[fmt, entry])
}

pub fn unsafe_path() -> String {
    encode("err:unsafe_path", &[])
}

pub fn archive_entry_not_found(fmt: &str, entry: &str) -> String {
    encode("err:entry_missing", &[fmt, entry])
}

pub fn archive_entry_error(entry: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:entry_err", &[entry, &err.to_string()])
}

pub fn archive_entry_read(entry: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:entry_read", &[entry, &err.to_string()])
}

pub fn archive_cache_unavailable(fmt: &str) -> String {
    encode("err:cache_unavail", &[fmt])
}

pub fn fb2_missing_in_zip() -> String {
    encode("err:fb2_zip_missing", &[])
}

pub fn fb2_missing_root() -> String {
    encode("err:fb2_no_root", &[])
}

pub fn fb2_extract_failed() -> String {
    encode("err:fb2_extract", &[])
}

pub fn epub_no_rootfile() -> String {
    encode("err:epub_no_opf", &[])
}

pub fn epub_parse_opf(err: &dyn std::fmt::Display) -> String {
    encode("err:epub_opf", &[&err.to_string()])
}

pub fn epub_parse_container(err: &dyn std::fmt::Display) -> String {
    encode("err:epub_container", &[&err.to_string()])
}

pub fn epub_empty_spine() -> String {
    encode("err:epub_empty_spine", &[])
}

pub fn comic_xml_parse(err: &dyn std::fmt::Display) -> String {
    encode("err:comic_xml", &[&err.to_string()])
}

pub fn comic_unsupported_format(ext: &str) -> String {
    encode("err:comic_unsup", &[ext])
}

pub fn comic_no_images() -> String {
    encode("err:comic_no_imgs", &[])
}

pub fn unsupported_format(path: &str) -> String {
    encode("err:unsup_fmt", &[path])
}

pub fn file_too_short(fmt: &str) -> String {
    encode("err:short_file", &[fmt])
}

pub fn invalid_header(fmt: &str, detail: &str) -> String {
    encode("err:bad_header", &[fmt, detail])
}

pub fn file_empty(fmt: &str) -> String {
    encode("err:file_empty", &[fmt])
}

pub fn invalid_sector_size(fmt: &str) -> String {
    encode("err:bad_sector", &[fmt])
}

pub fn parse_failed(fmt: &str, err: &dyn std::fmt::Display) -> String {
    encode("err:parse_failed", &[fmt, &err.to_string()])
}

pub fn parse_corrupted(fmt: &str) -> String {
    encode("err:parse_corrupt", &[fmt])
}

pub fn decoder_panic(fmt: &str) -> String {
    encode("err:decoder_panic", &[fmt])
}

pub fn no_pages(fmt: &str) -> String {
    encode("err:no_pages", &[fmt])
}

pub fn no_readable_text(fmt: &str) -> String {
    encode("err:no_read_text", &[fmt])
}

pub fn empty_document(fmt: &str) -> String {
    encode("err:empty_doc", &[fmt])
}

pub fn binary_file() -> String {
    encode("err:binary_file", &[])
}

pub fn extract_chapters_failed() -> String {
    encode("err:extract_chaps", &[])
}

pub fn page_out_of_bounds(page: usize) -> String {
    encode("err:page_oob", &[&page.to_string()])
}

pub fn page_not_found(page: usize) -> String {
    encode("err:page_missing", &[&page.to_string()])
}

pub fn page_get_error(page: usize, err: &dyn std::fmt::Debug) -> String {
    encode("err:page_get", &[&page.to_string(), &format!("{err:?}")])
}

pub fn page_invalid_dims(page: usize) -> String {
    encode("err:page_bad_dims", &[&page.to_string()])
}

pub fn page_zero_size(page: usize) -> String {
    encode("err:page_zero", &[&page.to_string()])
}

pub fn page_render_error(fmt: &str, page: usize, err: &dyn std::fmt::Debug) -> String {
    encode(
        "err:page_render",
        &[fmt, &page.to_string(), &format!("{err:?}")],
    )
}

pub fn page_buffer_mismatch(page: usize, expected: usize, actual: usize) -> String {
    encode(
        "err:page_buf",
        &[
            &page.to_string(),
            &expected.to_string(),
            &actual.to_string(),
        ],
    )
}

pub fn page_render_panic(page: usize) -> String {
    encode("err:page_panic", &[&page.to_string()])
}

pub fn image_not_found() -> String {
    encode("err:img_missing", &[])
}

pub fn image_empty() -> String {
    encode("err:img_empty", &[])
}

pub fn image_format_error(err: &dyn std::fmt::Display) -> String {
    encode("err:img_fmt", &[&err.to_string()])
}

pub fn image_zero_size() -> String {
    encode("err:img_zero", &[])
}

pub fn image_too_large(w: u32, h: u32) -> String {
    encode("err:img_limit", &[&w.to_string(), &h.to_string()])
}

pub fn image_decode_failed(err: &dyn std::fmt::Display) -> String {
    encode("err:img_decode", &[&err.to_string()])
}

pub mod tokens {
    use super::encode;

    pub const UNTITLED: &str = "\x1e@title:untitled";
    pub const UNKNOWN_AUTHOR: &str = "\x1e@author:unknown";
    pub const NOTES: &str = "\x1e@doc:notes";
    pub const NO_TEXT: &str = "\x1e@doc:no_text";
    pub const SECTION_PLAIN: &str = "\x1e@sec:plain";
    pub const TITLE_COMIC: &str = "\x1e@title:comic";
    pub const TITLE_WORD: &str = "\x1e@title:word";
    pub const TITLE_PDF: &str = "\x1e@title:pdf";
    pub const TITLE_DJVU: &str = "\x1e@title:djvu";
    pub const TITLE_MOBI: &str = "\x1e@title:mobi";
    pub const TITLE_RTF: &str = "\x1e@title:rtf";
    pub const TITLE_TEXT: &str = "\x1e@title:text";
    pub const TITLE_HTML: &str = "\x1e@title:html";

    pub fn section(n: usize) -> String {
        encode("@sec", &[&n.to_string()])
    }

    pub fn page(n: usize) -> String {
        encode("@page", &[&n.to_string()])
    }

    pub fn chapter(n: usize) -> String {
        encode("@chap", &[&n.to_string()])
    }

    pub fn section_page(section: usize, page: usize) -> String {
        encode("@sec_page", &[&section.to_string(), &page.to_string()])
    }

    pub fn chapter_part(title: &str, part: usize) -> String {
        super::encode_with_nested_first("@chap_part", title, &[&part.to_string()])
    }

    pub fn lines_range(title: &str, start: usize, end: usize) -> String {
        super::encode_with_nested_first("@lines", title, &[&start.to_string(), &end.to_string()])
    }

    pub fn comic_total_pages(n: usize) -> String {
        encode("@comic_total", &[&n.to_string()])
    }

    pub fn comic_pages_range(start: usize, end: usize, count: usize) -> String {
        encode(
            "@comic_range",
            &[&start.to_string(), &end.to_string(), &count.to_string()],
        )
    }
}
