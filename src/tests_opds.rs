use crate::opds::*;

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
        detect_format_and_ext("application/pdf", ""),
        Some(("PDF", "pdf"))
    );
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
