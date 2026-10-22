use crate::i18n::{UiLanguage, format_opds_error};
use crate::opds::client::{TempFileGuard, make_unique_filepath, read_limited_text};
use crate::opds::*;
use crate::test::util::unique_test_dir;

#[test]
fn test_resolve_relative_url() {
    assert_eq!(
        resolve_relative_url("https://example.com/opds/catalog.atom", "/search?q=1"),
        "https://example.com/search?q=1"
    );
    assert_eq!(
        resolve_relative_url("https://example.com/opds/catalog.atom", "next.atom"),
        "https://example.com/opds/next.atom"
    );
    assert_eq!(
        resolve_relative_url("https://example.com/catalog", "sub/books.atom"),
        "https://example.com/sub/books.atom"
    );
    assert_eq!(
        resolve_relative_url("http://example.com/a/b/", "c.atom"),
        "http://example.com/a/b/c.atom"
    );
    assert_eq!(
        resolve_relative_url(
            "https://example.com/page",
            "https://cdn.example.com/book.epub"
        ),
        "https://cdn.example.com/book.epub"
    );
}

#[test]
fn test_percent_encode_query() {
    assert_eq!(percent_encode_query("hello world"), "hello+world");
    assert_eq!(
        percent_encode_query("tolstoy&dostoevsky"),
        "tolstoy%26dostoevsky"
    );
}

#[test]
fn test_base64_encode() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"admin:secret123"), "YWRtaW46c2VjcmV0MTIz");
}

#[test]
fn test_detect_format_and_ext() {
    assert_eq!(
        detect_format_and_ext("application/epub+zip", "book"),
        Some(("EPUB", "epub"))
    );
    assert_eq!(
        detect_format_and_ext("application/octet-stream", "https://site/book.mobi"),
        Some(("MOBI", "mobi"))
    );
    assert_eq!(
        detect_format_and_ext("application/x-fictionbook+xml", ""),
        Some(("FB2", "fb2"))
    );
    assert_eq!(
        detect_format_and_ext(
            "application/octet-stream",
            "https://site/download/book.fb2.zip"
        ),
        Some(("FB2.ZIP", "fb2.zip"))
    );
    assert_eq!(
        detect_format_and_ext("application/pdf", ""),
        Some(("PDF", "pdf"))
    );
}

#[test]
fn test_make_unique_filepath_keeps_fb2_zip_extension() {
    let dir = unique_test_dir("opds_fb2_zip_ext");
    let path = make_unique_filepath(&dir, "Author", "Book", "fb2.zip");
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some("Author - Book.fb2.zip")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_parse_simple_feed() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>urn:catalog:root</id>
  <title>Test Catalog</title>
  <link rel="self" href="/opds" type="application/atom+xml"/>
  <link rel="next" href="/opds?page=2" type="application/atom+xml"/>
  <link rel="search" type="application/atom+xml" href="/search?q={searchTerms}"/>
  <entry>
    <id>urn:book:1</id>
    <title>War and Peace</title>
    <author><name>Leo Tolstoy</name></author>
    <summary>A legendary novel &amp; historical epic.</summary>
    <link rel="http://opds-spec.org/acquisition" href="/download/war_and_peace.epub" type="application/epub+zip"/>
    <link rel="http://opds-spec.org/image/thumbnail" href="/covers/war.jpg" type="image/jpeg"/>
  </entry>
  <entry>
    <id>urn:nav:fiction</id>
    <title>Fiction Books</title>
    <link rel="subsection" href="/opds/fiction" type="application/atom+xml;profile=opds-catalog"/>
  </entry>
</feed>"#;

    let feed = parse_opds_feed(xml, "https://example.com/opds").expect("Parse failed");
    assert_eq!(feed.title, "Test Catalog");
    assert_eq!(
        feed.next_url,
        Some("https://example.com/opds?page=2".to_string())
    );
    assert_eq!(
        feed.search_template,
        Some("https://example.com/search?q={searchTerms}".to_string())
    );
    assert_eq!(feed.entries.len(), 2);

    let book = &feed.entries[0];
    assert_eq!(book.title, "War and Peace");
    assert_eq!(book.authors, vec!["Leo Tolstoy".to_string()]);
    assert_eq!(
        book.summary,
        Some("A legendary novel & historical epic.".to_string())
    );
    assert_eq!(book.acquisition_links.len(), 1);
    assert_eq!(book.acquisition_links[0].format_label, "EPUB");
    assert_eq!(
        book.thumbnail_url,
        Some("https://example.com/covers/war.jpg".to_string())
    );

    let cat = &feed.entries[1];
    assert_eq!(cat.title, "Fiction Books");
    assert_eq!(
        cat.navigation_link,
        Some("https://example.com/opds/fiction".to_string())
    );
}

#[test]
fn test_parse_opensearch_description() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>Catalog Search</ShortName>
  <Description>Search eBooks</Description>
  <Url type="application/atom+xml" template="/search?query={searchTerms}"/>
</OpenSearchDescription>"#;

    let tmpl = parse_opensearch_template(xml, "https://openbooks.com/catalog/")
        .expect("OpenSearch parse failed");
    assert_eq!(tmpl, "https://openbooks.com/search?query={searchTerms}");
}

#[test]
fn test_default_catalogs_count() {
    let cats = default_opds_catalogs();
    assert_eq!(cats.len(), 3);
    assert!(cats.iter().any(|c| c.name.contains("Standard Ebooks")));
    assert!(cats.iter().any(|c| c.name.contains("Gutenberg")));
    assert!(cats.iter().any(|c| c.name.contains("Feedbooks")));
}

fn test_catalog(url: &str, username: Option<&str>, password: Option<&str>) -> OpdsCatalog {
    OpdsCatalog {
        id: "test".to_string(),
        name: "Test".to_string(),
        url: url.to_string(),
        username: username.map(str::to_string),
        password: password.map(str::to_string),
        is_default: false,
    }
}

#[test]
fn test_auth_rejects_plain_http_catalog() {
    let catalog = test_catalog("http://example.com/opds", Some("user"), Some("pass"));
    assert_eq!(
        OpdsAuth::from_catalog(&catalog),
        Err(OpdsAuthError::HttpsRequired)
    );
}

#[test]
fn test_auth_allows_http_catalog_without_credentials() {
    let catalog = test_catalog("http://example.com/opds", None, None);
    assert!(OpdsAuth::from_catalog(&catalog).unwrap().is_none());
}

#[test]
fn test_auth_header_is_scoped_to_catalog_origin() {
    let catalog = test_catalog("https://example.com/opds", Some("user"), Some("pass"));
    let auth = OpdsAuth::from_catalog(&catalog).unwrap().unwrap();

    assert_eq!(
        auth.header_for("https://example.com/books/1.epub"),
        Some("Basic dXNlcjpwYXNz".to_string())
    );
    assert_eq!(
        auth.header_for("https://example.com:443/books/1.epub"),
        Some("Basic dXNlcjpwYXNz".to_string())
    );
    assert!(
        auth.header_for("https://cdn.example.com/books/1.epub")
            .is_none()
    );
    assert!(
        auth.header_for("https://example.com:8443/books/1.epub")
            .is_none()
    );
    assert!(auth.header_for("http://example.com/books/1.epub").is_none());
}

#[test]
fn test_validate_catalog_url() {
    assert_eq!(
        validate_catalog_url("https://example.com/opds", true),
        Ok(())
    );
    assert_eq!(
        validate_catalog_url("http://example.com/opds", false),
        Ok(())
    );
    assert_eq!(
        validate_catalog_url("http://example.com/opds", true),
        Err(OpdsAuthError::HttpsRequired)
    );
    assert_eq!(
        validate_catalog_url("ftp://example.com/opds", false),
        Err(OpdsAuthError::UnsupportedScheme)
    );
    assert_eq!(
        validate_catalog_url("example.com/opds", false),
        Err(OpdsAuthError::InvalidUrl)
    );
}

#[test]
fn test_read_limited_text_accepts_exact_limit() {
    let mut data = std::io::Cursor::new(vec![b'a'; 10]);
    assert_eq!(read_limited_text(&mut data, 10).unwrap(), "aaaaaaaaaa");
}

#[test]
fn test_read_limited_text_rejects_oversized_response() {
    let mut data = std::io::Cursor::new(vec![b'a'; 11]);
    assert!(matches!(
        read_limited_text(&mut data, 10),
        Err(OpdsError::ResponseTooLarge { limit: 10 })
    ));
}

#[test]
fn test_temp_file_guard_removes_file_on_drop() {
    let dir = unique_test_dir("opds_temp_guard");
    let path = dir.join("book.part");
    std::fs::write(&path, b"data").unwrap();
    {
        let _guard = TempFileGuard::new(path.clone());
    }
    assert!(!path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_temp_file_guard_keeps_file_after_disarm() {
    let dir = unique_test_dir("opds_temp_guard_keep");
    let path = dir.join("book.part");
    std::fs::write(&path, b"data").unwrap();
    TempFileGuard::new(path.clone()).disarm();
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_opds_limit_errors_are_localized() {
    let error = OpdsError::DownloadTooLarge {
        limit: 1024 * 1024 * 1024,
    };
    let ru = format_opds_error(&error, UiLanguage::Ru);
    let en = format_opds_error(&error, UiLanguage::En);
    assert!(ru.contains("1024"));
    assert!(en.contains("1024"));
    assert_ne!(ru, en);
}
