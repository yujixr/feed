use anyhow::Context;
use feed::feed_source::{self, RawFeed};

// RSS XML is parsed into RawFeed with correct title, entries, URLs, and dates.
#[test]
fn test_parse_rss_feed() -> anyhow::Result<()> {
    let xml = include_bytes!("fixtures/sample_rss.xml");
    let feed = RawFeed::parse(xml, None, None)?;

    assert_eq!(feed.title, "Test Blog");
    assert_eq!(feed.entries.len(), 2);
    assert_eq!(feed.entries[0].title, "First Post");
    assert_eq!(feed.entries[0].url, "https://example.com/post1");
    assert!(feed.entries[0].published.is_some());
    assert_eq!(
        feed.entries[0]
            .published
            .context("expected published date")?
            .format("%Y-%m-%d")
            .to_string(),
        "2026-02-28"
    );
    assert_eq!(feed.entries[1].title, "Second Post");
    Ok(())
}

// Missing <title> in channel and item both fall back to "(untitled)".
#[test]
fn test_parse_empty_title() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <item>
          <link>https://example.com/post</link>
        </item>
      </channel>
    </rss>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    assert_eq!(feed.title, "(untitled)");
    assert_eq!(feed.entries[0].title, "(untitled)");
    Ok(())
}

// discover_feed_urls finds an RSS <link> tag in HTML.
#[test]
fn test_discover_rss_link_from_html() -> anyhow::Result<()> {
    let html = include_str!("fixtures/page_with_rss.html");
    let base = "https://example.com/";
    let urls = feed_source::discover_feed_urls(html, base)?;
    assert_eq!(urls, vec!["https://example.com/feed.xml"]);
    Ok(())
}

// discover_feed_urls returns all feed links when a page has multiple (RSS + Atom).
#[test]
fn test_discover_multiple_feeds_returns_all() -> anyhow::Result<()> {
    let html = include_str!("fixtures/page_with_multiple_feeds.html");
    let base = "https://example.com/";
    let urls = feed_source::discover_feed_urls(html, base)?;
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://example.com/rss.xml");
    assert_eq!(urls[1], "https://example.com/atom.xml");
    Ok(())
}

// discover_feed_urls returns an error when no feed link exists.
#[test]
fn test_discover_no_feed_returns_error() {
    let html = include_str!("fixtures/page_without_feed.html");
    let base = "https://example.com/";
    let result = feed_source::discover_feed_urls(html, base);
    assert!(result.is_err());
}

// Relative feed URLs in <link> are resolved against the base URL.
#[test]
fn test_discover_relative_url_resolved() -> anyhow::Result<()> {
    let html = include_str!("fixtures/page_with_relative_feed.html");
    let base = "https://example.com/blog/index.html";
    let urls = feed_source::discover_feed_urls(html, base)?;
    assert_eq!(urls, vec!["https://example.com/feed.xml"]);
    Ok(())
}

// <content:encoded> takes priority over <description> for rss_content.
#[test]
fn test_parse_rss_with_content_encoded() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
      <channel>
        <title>Blog</title>
        <item>
          <title>Post</title>
          <link>https://example.com/post</link>
          <description>Summary text</description>
          <content:encoded><![CDATA[<p>Full article content</p>]]></content:encoded>
        </item>
      </channel>
    </rss>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    let entry = &feed.entries[0];
    let content = entry
        .rss_content
        .as_ref()
        .context("rss_content should exist")?;
    assert!(content.contains("Full article content"));
    Ok(())
}

// <description> is used as rss_content when <content:encoded> is absent.
#[test]
fn test_parse_rss_with_description_only() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Blog</title>
        <item>
          <title>Post</title>
          <link>https://example.com/post</link>
          <description>Summary text only</description>
        </item>
      </channel>
    </rss>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    let entry = &feed.entries[0];
    let content = entry
        .rss_content
        .as_ref()
        .context("rss_content should exist")?;
    assert!(content.contains("Summary text only"));
    Ok(())
}

// rss_content is None when neither <content:encoded> nor <description> exists.
#[test]
fn test_parse_rss_no_content_or_description() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Blog</title>
        <item>
          <title>Post</title>
          <link>https://example.com/post</link>
        </item>
      </channel>
    </rss>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    assert!(feed.entries[0].rss_content.is_none());
    Ok(())
}

// Atom <content> is stored as rss_content, taking priority over <summary>.
#[test]
fn test_parse_atom_with_content() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
      <title>Atom Blog</title>
      <entry>
        <title>Atom Post</title>
        <link href="https://example.com/atom-post"/>
        <id>urn:uuid:1234</id>
        <updated>2026-03-01T00:00:00Z</updated>
        <summary>Atom summary</summary>
        <content type="html"><![CDATA[<p>Full atom content</p>]]></content>
      </entry>
    </feed>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    assert_eq!(feed.title, "Atom Blog");
    let entry = &feed.entries[0];
    let content = entry
        .rss_content
        .as_ref()
        .context("rss_content should exist")?;
    assert!(content.contains("Full atom content"));
    Ok(())
}

// Atom <summary> is used as rss_content when <content> is absent.
#[test]
fn test_parse_atom_with_summary_only() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
      <title>Atom Blog</title>
      <entry>
        <title>Atom Post</title>
        <link href="https://example.com/atom-post"/>
        <id>urn:uuid:5678</id>
        <updated>2026-03-01T00:00:00Z</updated>
        <summary>Atom summary only</summary>
      </entry>
    </feed>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    let entry = &feed.entries[0];
    let content = entry
        .rss_content
        .as_ref()
        .context("rss_content should exist")?;
    assert!(content.contains("Atom summary only"));
    Ok(())
}

// ETag and Last-Modified from HTTP response are preserved in RawFeed.
#[test]
fn test_parse_rss_preserves_etag_and_last_modified() -> anyhow::Result<()> {
    let xml = include_bytes!("fixtures/sample_rss.xml");
    let feed = RawFeed::parse(
        xml,
        Some("\"etag-123\"".to_string()),
        Some("Sat, 01 Mar 2026 00:00:00 GMT".to_string()),
    )?;
    assert_eq!(feed.etag.as_deref(), Some("\"etag-123\""));
    assert_eq!(
        feed.last_modified.as_deref(),
        Some("Sat, 01 Mar 2026 00:00:00 GMT")
    );
    Ok(())
}

// Atom <updated> is used as published date when <published> is missing.
#[test]
fn test_parse_rss_entry_uses_updated_when_no_published() -> anyhow::Result<()> {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
      <title>Blog</title>
      <entry>
        <title>Updated Post</title>
        <link href="https://example.com/updated"/>
        <id>urn:uuid:9999</id>
        <updated>2026-02-15T10:00:00Z</updated>
      </entry>
    </feed>"#;
    let feed = RawFeed::parse(xml, None, None)?;
    let entry = &feed.entries[0];
    assert!(entry.published.is_some());
    assert_eq!(
        entry.published.unwrap().format("%Y-%m-%d").to_string(),
        "2026-02-15"
    );
    Ok(())
}
