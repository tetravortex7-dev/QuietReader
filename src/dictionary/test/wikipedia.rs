#[test]
fn parses_wikipedia_article() {
    let json = r#"{
      "query": {
        "pages": [
          {
            "title": "Apple",
            "extract": "An apple is a fruit.",
            "fullurl": "https://en.wikipedia.org/wiki/Apple"
          }
        ]
      }
    }"#;
    let article = crate::dictionary::wikipedia::parse_article(json).unwrap();
    assert_eq!(article.title, "Apple");
    assert_eq!(article.extract, "An apple is a fruit.");
    assert_eq!(article.url, "https://en.wikipedia.org/wiki/Apple");
}

#[test]
fn rejects_missing_or_empty_article() {
    assert!(crate::dictionary::wikipedia::parse_article(r#"{"query":{}}"#).is_err());
    assert!(
        crate::dictionary::wikipedia::parse_article(
            r#"{"query":{"pages":[{"title":"Apple","extract":""}]}}"#
        )
        .is_err()
    );
}
