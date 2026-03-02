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

#[test]
fn test_format_feed_contains_title() -> anyhow::Result<()> {
    let output = render_article_list("Test Blog", &sample_items()?);
    assert!(output.contains("Test Blog"));
    Ok(())
}

#[test]
fn test_format_feed_contains_entries() -> anyhow::Result<()> {
    let output = render_article_list("Test Blog", &sample_items()?);
    assert!(output.contains("First Post"));
    assert!(output.contains("2026-02-28"));
    assert!(output.contains("https://example.com/post1"));
    Ok(())
}

#[test]
fn test_format_feed_with_limit() -> anyhow::Result<()> {
    let items = sample_items()?;
    let limited = &items[..1];
    let output = render_article_list("Test Blog", limited);
    assert!(output.contains("First Post"));
    assert!(!output.contains("Second Post"));
    Ok(())
}

#[test]
fn test_render_feed_list_empty() {
    let config = Config::default();
    let output = render_feed_list(&config);
    assert!(output.contains("No feeds registered"));
}

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

#[test]
fn test_render_tag_list_empty() {
    let config = Config::default();
    let output = render_tag_list(&config);
    assert!(output.contains("No tags found"));
}

#[test]
fn test_pad_or_truncate_pads() {
    let result = pad_or_truncate("hi", 10);
    assert_eq!(display_width(&result), 10);
}

#[test]
fn test_pad_or_truncate_truncates() {
    let result = pad_or_truncate("this is a very long title", 10);
    assert!(display_width(&result) <= 10);
    assert!(result.contains('…'));
}

// --- display_width tests ---

#[test]
fn test_display_width_ascii() {
    assert_eq!(display_width("hello"), 5);
}

#[test]
fn test_display_width_empty() {
    assert_eq!(display_width(""), 0);
}

#[test]
fn test_display_width_cjk() {
    // CJK characters are 2 columns wide
    assert_eq!(display_width("日本語"), 6);
}

#[test]
fn test_display_width_mixed() {
    // "Hello" (5) + "世界" (4) = 9
    assert_eq!(display_width("Hello世界"), 9);
}

#[test]
fn test_display_width_emoji() {
    // Emoji width varies, but should be > 0
    assert!(display_width("🦀") > 0);
}

// --- truncate tests ---

#[test]
fn test_truncate_short_string_unchanged() {
    assert_eq!(truncate("hi", 10), "hi");
}

#[test]
fn test_truncate_exact_width_unchanged() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn test_truncate_long_string() {
    let result = truncate("this is a very long string", 10);
    assert!(display_width(&result) <= 10);
    assert!(result.contains('…'));
}

#[test]
fn test_truncate_cjk_respects_width() {
    // "日本語のテスト" = 14 columns; truncate to 6
    let result = truncate("日本語のテスト", 6);
    assert!(display_width(&result) <= 6);
    assert!(result.contains('…'));
}

#[test]
fn test_truncate_width_one() {
    let result = truncate("hello", 1);
    // Only the ellipsis should fit
    assert_eq!(result, "…");
    assert_eq!(display_width(&result), 1);
}

#[test]
fn test_truncate_empty_string() {
    assert_eq!(truncate("", 10), "");
}

// --- render_article_list edge cases ---

#[test]
fn test_render_article_list_empty_entries() {
    let output = render_article_list("Empty Feed", &[]);
    assert!(output.contains("Empty Feed"));
    assert!(output.contains("(no entries)"));
}

#[test]
fn test_render_article_list_no_published_date() -> anyhow::Result<()> {
    let items = vec![DisplayItem {
        title: "No Date Post",
        url: "https://example.com/nodate",
        published: None,
    }];
    let output = render_article_list("Blog", &items);
    assert!(output.contains("No Date Post"));
    // Should have spaces in place of date
    assert!(output.contains("          "));
    Ok(())
}

// --- pad_or_truncate additional tests ---

#[test]
fn test_pad_or_truncate_cjk() {
    let result = pad_or_truncate("日本", 10);
    assert_eq!(display_width(&result), 10);
}

#[test]
fn test_pad_or_truncate_exact_width() {
    let result = pad_or_truncate("hello", 5);
    assert_eq!(result, "hello");
    assert_eq!(display_width(&result), 5);
}
