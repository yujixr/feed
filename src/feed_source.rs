use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::cache::HttpMetadata;

#[derive(Clone)]
pub struct RawFeed {
    pub title: String,
    pub entries: Vec<RawEntry>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Clone)]
pub struct RawEntry {
    pub title: String,
    pub url: String,
    pub published: Option<DateTime<Utc>>,
    pub rss_content: Option<String>,
}

pub enum FetchResult {
    Fetched(RawFeed),
    NotModified,
}

impl RawFeed {
    /// Parse RSS/Atom feed from bytes.
    pub fn parse(data: &[u8], etag: Option<String>, last_modified: Option<String>) -> Result<Self> {
        let feed = parser::parse(data).context("Failed to parse feed")?;

        let title = feed
            .title
            .map(|t| t.content)
            .unwrap_or_else(|| "(untitled)".to_string());

        let entries = feed
            .entries
            .into_iter()
            .map(|entry| {
                let entry_title = entry
                    .title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "(untitled)".to_string());

                let url = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_default();

                let published = entry
                    .published
                    .or(entry.updated)
                    .map(|dt| dt.with_timezone(&Utc));

                let rss_content = entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.clone())
                    .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()));

                RawEntry {
                    title: entry_title,
                    url,
                    published,
                    rss_content,
                }
            })
            .collect();

        Ok(RawFeed {
            title,
            entries,
            etag,
            last_modified,
        })
    }
}

/// Fetch a feed with conditional GET support.
pub async fn fetch(client: &Client, url: &str, metadata: &HttpMetadata) -> Result<FetchResult> {
    let mut request = client.get(url).header("User-Agent", "feed-cli/0.1");

    if let Some(etag) = &metadata.etag {
        request = request.header("If-None-Match", etag.as_str());
    }
    if let Some(lm) = &metadata.last_modified {
        request = request.header("If-Modified-Since", lm.as_str());
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to fetch: {}", url))?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FetchResult::NotModified);
    }

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response from: {}", url))?;

    let feed = RawFeed::parse(&bytes, etag, last_modified)?;

    Ok(FetchResult::Fetched(feed))
}

/// Discover RSS/Atom feed URLs from HTML via autodiscovery.
pub fn discover_feed_urls(html: &str, base_url: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(
        r#"link[rel="alternate"][type="application/rss+xml"], link[rel="alternate"][type="application/atom+xml"]"#,
    )
    .expect("valid CSS selector");

    let base = Url::parse(base_url).context("Invalid base URL")?;

    let urls: Vec<String> = document
        .select(&selector)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| base.join(href).ok())
        .map(|u| u.to_string())
        .collect();

    if urls.is_empty() {
        bail!("No RSS/Atom feed found at {}", base_url);
    }

    Ok(urls)
}

/// Resolve a URL to a feed URL (follows HTML autodiscovery if needed).
pub async fn resolve_feed_url(client: &Client, url: &str) -> Result<String> {
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
        .unwrap_or("")
        .to_string();

    if !is_html_content_type(&content_type) {
        return Ok(url.to_string());
    }

    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from: {}", url))?;

    let feed_urls = discover_feed_urls(&body, url)?;
    feed_urls.into_iter().next().context("No feed URL found")
}

/// Returns true if the Content-Type header value indicates HTML content.
fn is_html_content_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .map(|ct| ct.trim().eq_ignore_ascii_case("text/html"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // is_html_content_type returns true only for "text/html" (case-insensitive, ignoring params).
    #[test]
    fn test_is_html_content_type() {
        assert!(is_html_content_type("text/html"));
        assert!(is_html_content_type("text/html; charset=utf-8"));
        assert!(is_html_content_type("TEXT/HTML"));
        assert!(!is_html_content_type("application/rss+xml"));
        assert!(!is_html_content_type("application/atom+xml"));
        assert!(!is_html_content_type("application/xml"));
        assert!(!is_html_content_type("text/xml"));
        assert!(!is_html_content_type(""));
        assert!(!is_html_content_type("  "));
        assert!(is_html_content_type("text/html ; charset=utf-8"));
    }
}
