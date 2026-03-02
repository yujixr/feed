use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use encoding_rs::Encoding;
use reqwest::Client;

use crate::config::ExtractorMethod;

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub url: String,
    pub published: Option<DateTime<Utc>>,
    pub feed_url: String,
    pub feed_name: String,
    pub extractor: ExtractorMethod,
    pub read: bool,
    pub rss_content: Option<String>,
}

/// Extract readable HTML from raw HTML using Readability. Returns (title, content_html).
/// Falls back to the original HTML when Readability cannot parse the input.
pub fn parse_readable_html(html: &str) -> (String, String) {
    match readability::Readability::new(html, None) {
        Ok(mut r) => match r.parse() {
            Some(article) => (
                article.title.unwrap_or_default(),
                article.content.unwrap_or_default(),
            ),
            None => (String::new(), html.to_string()),
        },
        Err(_) => (String::new(), html.to_string()),
    }
}

/// Return RSS content HTML as-is if non-empty.
pub(crate) fn extract_rss_html(rss_content: Option<&str>) -> Option<String> {
    rss_content.filter(|c| !c.is_empty()).map(|c| c.to_string())
}

/// Fetch a URL and extract readable HTML content. Returns (title, content_html).
pub(crate) async fn extract_readable_html(client: &Client, url: &str) -> Result<(String, String)> {
    let response = client
        .get(url)
        .header("User-Agent", "feed-cli/0.1")
        .send()
        .await
        .with_context(|| format!("Failed to fetch: {}", url))?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response from: {}", url))?;

    let html = decode_html_bytes(&bytes, content_type.as_deref());

    Ok(parse_readable_html(&html))
}

/// Render HTML (or plain text) to wrapped text.
pub fn html_to_text(html: &str, width: usize) -> String {
    html2text::from_read(html.as_bytes(), width).unwrap_or_default()
}

/// Fetch article using specified extraction method, returning HTML.
pub async fn extract_html(
    client: &Client,
    article_url: &str,
    method: &ExtractorMethod,
    rss_content: Option<&str>,
) -> Result<String> {
    match method {
        ExtractorMethod::Readability => match extract_readable_html(client, article_url).await {
            Ok((_title, html)) => Ok(html),
            Err(e) => extract_rss_html(rss_content).ok_or(e),
        },
        ExtractorMethod::RssContent => {
            if let Some(html) = extract_rss_html(rss_content) {
                Ok(html)
            } else {
                let (_title, html) = extract_readable_html(client, article_url).await?;
                Ok(html)
            }
        }
    }
}

/// Decode raw bytes to a UTF-8 string, detecting encoding from Content-Type header
/// and HTML meta tags. Falls back to UTF-8 (lossy).
fn decode_html_bytes(bytes: &[u8], content_type: Option<&str>) -> String {
    let charset = detect_charset(content_type, bytes);

    if let Some(charset) = charset {
        if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
            if encoding != encoding_rs::UTF_8 {
                let (decoded, _, _) = encoding.decode(bytes);
                return decoded.into_owned();
            }
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

/// Detect charset from Content-Type header or HTML meta tags.
fn detect_charset(content_type: Option<&str>, bytes: &[u8]) -> Option<String> {
    // Try Content-Type header first (e.g. "text/html; charset=shift_jis")
    if let Some(ct) = content_type {
        let found = ct.split(';').skip(1).find_map(|param| {
            let mut parts = param.trim().splitn(2, '=');
            let key = parts.next()?.trim();
            let value = parts.next()?.trim().trim_matches('"');
            key.eq_ignore_ascii_case("charset").then_some(value)
        });
        if let Some(charset) = found {
            return Some(charset.to_string());
        }
    }

    // Fall back to HTML meta tags in the first few KB
    let head = &bytes[..bytes.len().min(4096)];
    let lossy = String::from_utf8_lossy(head);
    let lower = lossy.to_ascii_lowercase();

    let pos = lower.find("charset")?;
    let rest = &lossy[pos + 7..];
    let rest = rest.trim_start_matches(|c: char| c == '=' || c.is_ascii_whitespace());
    let charset: String = rest
        .trim_start_matches(['"', '\''])
        .chars()
        .take_while(|c| !matches!(c, '"' | '\'' | ';' | '>' | ' '))
        .collect();
    (!charset.is_empty()).then_some(charset)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- detect_charset tests ---

    // charset is extracted from the Content-Type header value.
    #[test]
    fn test_detect_charset_from_content_type() {
        let result = detect_charset(Some("text/html; charset=utf-8"), b"");
        assert_eq!(result, Some("utf-8".to_string()));
    }

    // "Charset" (uppercase C) is treated the same as "charset".
    #[test]
    fn test_detect_charset_case_insensitive_key() {
        let result = detect_charset(Some("text/html; Charset=UTF-8"), b"");
        assert_eq!(result, Some("UTF-8".to_string()));
    }

    // Quotes around the charset value are stripped (e.g. charset="shift_jis").
    #[test]
    fn test_detect_charset_quoted_value() {
        let result = detect_charset(Some("text/html; charset=\"shift_jis\""), b"");
        assert_eq!(result, Some("shift_jis".to_string()));
    }

    // charset is detected from an HTML <meta charset="..."> tag when no header is present.
    #[test]
    fn test_detect_charset_from_meta_tag() {
        let html = br#"<html><head><meta charset="euc-jp"></head></html>"#;
        let result = detect_charset(None, html);
        assert_eq!(result, Some("euc-jp".to_string()));
    }

    // charset is detected from a <meta http-equiv="Content-Type"> tag.
    #[test]
    fn test_detect_charset_from_meta_http_equiv() {
        let html = br#"<html><head><meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1"></head></html>"#;
        let result = detect_charset(None, html);
        assert_eq!(result, Some("iso-8859-1".to_string()));
    }

    // None is returned when neither header nor meta tag specifies a charset.
    #[test]
    fn test_detect_charset_none_when_absent() {
        let result = detect_charset(None, b"<html><body>hello world</body></html>");
        assert_eq!(result, None);
    }

    // Content-Type header wins over a conflicting <meta charset> in the HTML body.
    #[test]
    fn test_detect_charset_content_type_takes_priority() {
        let html = br#"<html><head><meta charset="euc-jp"></head></html>"#;
        let result = detect_charset(Some("text/html; charset=shift_jis"), html);
        assert_eq!(result, Some("shift_jis".to_string()));
    }

    // --- decode_html_bytes tests ---

    // UTF-8 Japanese text passes through unchanged when charset=utf-8 is specified.
    #[test]
    fn test_decode_utf8_japanese() {
        let result = decode_html_bytes("こんにちは".as_bytes(), Some("text/html; charset=utf-8"));
        assert_eq!(result, "こんにちは");
    }

    // Shift_JIS encoded bytes are correctly decoded to UTF-8.
    #[test]
    fn test_decode_shift_jis() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("テスト");
        let result = decode_html_bytes(&bytes, Some("text/html; charset=shift_jis"));
        assert_eq!(result, "テスト");
    }

    // ISO-8859-1 (Latin-1) encoded bytes are correctly decoded to UTF-8.
    #[test]
    fn test_decode_iso_8859_1() {
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode("café");
        let result = decode_html_bytes(&bytes, Some("text/html; charset=iso-8859-1"));
        assert_eq!(result, "café");
    }

    // EUC-JP encoded bytes are correctly decoded to UTF-8.
    #[test]
    fn test_decode_euc_jp() {
        let (bytes, _, _) = encoding_rs::EUC_JP.encode("日本語");
        let result = decode_html_bytes(&bytes, Some("text/html; charset=euc-jp"));
        assert_eq!(result, "日本語");
    }

    // Charset is auto-detected from <meta charset> when no Content-Type header is given.
    #[test]
    fn test_decode_charset_from_meta_tag() {
        let html = "<html><head><meta charset=\"shift_jis\"></head><body>テスト</body></html>";
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(html);
        let result = decode_html_bytes(&bytes, None);
        assert!(result.contains("テスト"));
    }

    // Bytes without any charset hint fall back to UTF-8 (lossy).
    #[test]
    fn test_decode_no_charset_lossy_utf8() {
        let result = decode_html_bytes(b"plain ascii", None);
        assert_eq!(result, "plain ascii");
    }

    // An unrecognized encoding label falls through to UTF-8 lossy instead of failing.
    #[test]
    fn test_decode_unknown_encoding_label() {
        let result = decode_html_bytes(b"hello", Some("text/html; charset=bogus-encoding"));
        assert_eq!(result, "hello");
    }
}
