use anyhow::Context;
use chrono::{TimeZone, Utc};
use feed::config::{Config, FeedEntry};
use feed::display::{
    display_width, pad_or_truncate, render_article_list, render_feed_list, render_tag_list,
    truncate, DisplayItem,
};

fn sample_items() -> anyhow::Result<Vec<DisplayItem<'static>>> {
    Ok(vec![
        DisplayItem {
            title: "First Post",
            url: "https://example.com/post1",
            published: Some(
                Utc.with_ymd_and_hms(2026, 2, 28, 12, 0, 0)
                    .single()
                    .context("invalid datetime")?,
            ),
        },
        DisplayItem {
            title: "Second Post",
            url: "https://example.com/post2",
            published: Some(
                Utc.with_ymd_and_hms(2026, 2, 27, 8, 0, 0)
                    .single()
                    .context("invalid datetime")?,
            ),
        },
    ])
}

// render_article_list output contains title, date, and URL of each entry.
#[test]
fn test_format_feed_contains_entries() -> anyhow::Result<()> {
    let output = render_article_list("Test Blog", &sample_items()?);
    assert!(output.contains("First Post"));
    assert!(output.contains("2026-02-28"));
    assert!(output.contains("https://example.com/post1"));
    Ok(())
}

// render_feed_list shows "No feeds registered" when the config has no feeds.
#[test]
fn test_render_feed_list_empty() {
    let config = Config::default();
    let output = render_feed_list(&config);
    assert!(output.contains("No feeds registered"));
}

// render_feed_list shows name, URL, and tags for each feed.
#[test]
fn test_render_feed_list() {
    let mut config = Config::default();
    config.add_feed(FeedEntry {
        name: "Blog".to_string(),
        url: "https://example.com/feed".to_string(),
        tags: vec!["tech".to_string()],
        extractor: None,
    });
    let output = render_feed_list(&config);
    assert!(output.contains("Blog"));
    assert!(output.contains("https://example.com/feed"));
    assert!(output.contains("[tech]"));
}

// render_tag_list shows all tags from feeds.
#[test]
fn test_render_tag_list() {
    let mut config = Config::default();
    config.add_feed(FeedEntry {
        name: "A".to_string(),
        url: "https://a.com/feed".to_string(),
        tags: vec!["news".to_string(), "tech".to_string()],
        extractor: None,
    });
    let output = render_tag_list(&config);
    assert!(output.contains("news"));
    assert!(output.contains("tech"));
}

// render_tag_list shows "No tags found" when no tags exist.
#[test]
fn test_render_tag_list_empty() {
    let config = Config::default();
    let output = render_tag_list(&config);
    assert!(output.contains("No tags found"));
}

// Short strings are padded with spaces to reach the target width.
#[test]
fn test_pad_or_truncate_pads() {
    let result = pad_or_truncate("hi", 10);
    assert_eq!(display_width(&result), 10);
}

// Long strings are truncated with an ellipsis to fit the target width.
#[test]
fn test_pad_or_truncate_truncates() {
    let result = pad_or_truncate("this is a very long title", 10);
    assert!(display_width(&result) <= 10);
    assert!(result.contains('…'));
}

// ASCII characters have width 1 each.
#[test]
fn test_display_width_ascii() {
    assert_eq!(display_width("hello"), 5);
}

// CJK characters have width 2 each.
#[test]
fn test_display_width_cjk() {
    assert_eq!(display_width("日本語"), 6);
}

// Strings shorter than the limit are returned unchanged.
#[test]
fn test_truncate_short_string_unchanged() {
    assert_eq!(truncate("hi", 10), "hi");
}

// Strings exactly at the limit are returned unchanged (boundary).
#[test]
fn test_truncate_exact_width_unchanged() {
    assert_eq!(truncate("hello", 5), "hello");
}

// Strings longer than the limit are cut and end with "…".
#[test]
fn test_truncate_long_string() {
    let result = truncate("this is a very long string", 10);
    assert!(display_width(&result) <= 10);
    assert!(result.contains('…'));
}

// CJK strings are truncated without splitting a wide character.
#[test]
fn test_truncate_cjk_respects_width() {
    let result = truncate("日本語のテスト", 6);
    assert!(display_width(&result) <= 6);
    assert!(result.contains('…'));
}

// Width 1 only fits the ellipsis character.
#[test]
fn test_truncate_width_one() {
    let result = truncate("hello", 1);
    assert_eq!(result, "…");
    assert_eq!(display_width(&result), 1);
}

// Empty entry list shows "(no entries)".
#[test]
fn test_render_article_list_empty_entries() {
    let output = render_article_list("Empty Feed", &[]);
    assert!(output.contains("Empty Feed"));
    assert!(output.contains("(no entries)"));
}

// Articles without a published date show blank space in the date column.
#[test]
fn test_render_article_list_no_published_date() -> anyhow::Result<()> {
    let items = vec![DisplayItem {
        title: "No Date Post",
        url: "https://example.com/nodate",
        published: None,
    }];
    let output = render_article_list("Blog", &items);
    assert!(output.contains("No Date Post"));
    assert!(output.contains("          "));
    Ok(())
}

// CJK strings are correctly padded to the target display width.
#[test]
fn test_pad_or_truncate_cjk() {
    let result = pad_or_truncate("日本", 10);
    assert_eq!(display_width(&result), 10);
}
