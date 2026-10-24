use std::time::Duration;

use serde_json::Value;

use super::read_limited;

const ARTICLE_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct WikiArticle {
    pub title: String,
    pub extract: String,
    pub url: String,
}

pub fn fetch(lang: &str, query: &str) -> Result<WikiArticle, String> {
    let lang = sanitize_lang(lang);
    let encoded: String = url::form_urlencoded::byte_serialize(query.trim().as_bytes()).collect();
    if encoded.is_empty() {
        return Err("empty query".to_string());
    }
    let url = format!(
        "https://{lang}.wikipedia.org/w/api.php?action=query&generator=search&gsrsearch={encoded}&gsrlimit=1&prop=extracts%7Cinfo&inprop=url&explaintext=1&exintro=1&format=json&formatversion=2"
    );
    let agent = crate::net::agent(Duration::from_secs(12));
    let mut response = agent
        .get(&url)
        .header(
            "User-Agent",
            "QuietReader/0.13 (https://github.com/tetravortex7-dev/QuietReader)",
        )
        .call()
        .map_err(|error| error.to_string())?;
    let data = read_limited(&mut response.body_mut().as_reader(), ARTICLE_LIMIT)?;
    let text = String::from_utf8_lossy(&data);
    parse_article(&text)
}

pub fn parse_article(json: &str) -> Result<WikiArticle, String> {
    let value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let page = value
        .pointer("/query/pages")
        .and_then(Value::as_array)
        .and_then(|pages| pages.first())
        .ok_or_else(|| "article not found".to_string())?;
    let title = page
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let extract = page
        .get("extract")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let url = page
        .get("fullurl")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if title.is_empty() || extract.is_empty() {
        return Err("article not found".to_string());
    }
    Ok(WikiArticle {
        title,
        extract,
        url,
    })
}

fn sanitize_lang(lang: &str) -> String {
    let filtered: String = lang
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .take(12)
        .collect();
    if filtered.is_empty() {
        "en".to_string()
    } else {
        filtered.to_lowercase()
    }
}
