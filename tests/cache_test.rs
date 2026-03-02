use anyhow::Context;
use chrono::{TimeZone, Utc};
use feed::cache::{data_dir, CacheStore};
use feed::feed_source::{RawEntry, RawFeed};
use tempfile::TempDir;

fn sample_feed() -> anyhow::Result<RawFeed> {
    Ok(RawFeed {
        title: "Test Blog".to_string(),
        entries: vec![
            RawEntry {
                title: "New Post".to_string(),
                url: "https://example.com/new".to_string(),
                published: Some(Utc::now()),
                rss_content: None,
            },
            RawEntry {
                title: "Old Post".to_string(),
                url: "https://example.com/old".to_string(),
                published: Some(
                    Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
                        .single()
                        .context("invalid date")?,
                ),
                rss_content: None,
            },
        ],
        etag: None,
        last_modified: None,
    })
}

// Saved feed can be loaded back with correct title and article count.
#[test]
fn test_save_and_load_cache() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let feed = sample_feed()?;
    let url = "https://example.com/feed.xml";

    cache.save_feed(url, &feed, None, None)?;
    let loaded = cache.load_feed(url).context("cache not found")?;

    assert_eq!(loaded.feed_title, "Test Blog");
    assert_eq!(loaded.articles.len(), 2);
    Ok(())
}

// Saving a feed twice merges entries by URL (no duplicates).
#[test]
fn test_cache_merges_entries() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let url = "https://example.com/feed.xml";

    let feed1 = RawFeed {
        title: "Blog".to_string(),
        entries: vec![RawEntry {
            title: "Post 1".to_string(),
            url: "https://example.com/1".to_string(),
            published: Some(Utc::now()),
            rss_content: None,
        }],
        etag: None,
        last_modified: None,
    };
    cache.save_feed(url, &feed1, None, None)?;

    let feed2 = RawFeed {
        title: "Blog".to_string(),
        entries: vec![
            RawEntry {
                title: "Post 1".to_string(),
                url: "https://example.com/1".to_string(),
                published: Some(Utc::now()),
                rss_content: None,
            },
            RawEntry {
                title: "Post 2".to_string(),
                url: "https://example.com/2".to_string(),
                published: Some(Utc::now()),
                rss_content: None,
            },
        ],
        etag: None,
        last_modified: None,
    };
    cache.save_feed(url, &feed2, None, None)?;

    let loaded = cache.load_feed(url).context("cache not found")?;
    assert_eq!(loaded.articles.len(), 2);
    Ok(())
}

// Loading a URL that was never cached returns None.
#[test]
fn test_load_nonexistent_cache() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    assert!(cache.load_feed("https://nonexistent.com").is_none());
    Ok(())
}

// purge_old_entries removes entries whose last_seen is older than retention days.
#[test]
fn test_purge_removes_entries_not_seen_recently() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let url = "https://example.com/feed.xml";

    let feed = sample_feed()?;
    cache.save_feed(url, &feed, None, None)?;

    // Patch the cache file: set last_seen to 60 days ago for "Old Post"
    let cache_path = dir.path().join({
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("{}.json", &hash[..16])
    });
    let content = std::fs::read_to_string(&cache_path)?;
    let mut cached: serde_json::Value = serde_json::from_str(&content)?;
    let entries = cached["articles"]
        .as_array_mut()
        .context("articles is not an array")?;
    for entry in entries.iter_mut() {
        if entry["url"] == "https://example.com/old" {
            let old_time = Utc::now() - chrono::Duration::days(60);
            entry["last_seen"] = serde_json::Value::String(old_time.to_rfc3339());
        }
    }
    std::fs::write(&cache_path, serde_json::to_string(&cached)?)?;

    cache.purge_old_entries(30)?;

    let loaded = cache.load_feed(url).context("cache not found")?;
    assert_eq!(loaded.articles.len(), 1);
    assert_eq!(loaded.articles[0].title, "New Post");
    Ok(())
}

// data_dir returns the custom path when one is provided.
#[test]
fn test_data_dir_config_path() -> anyhow::Result<()> {
    let dir = data_dir(Some("/tmp/my_cache"))?;
    assert_eq!(
        dir.to_str().context("path is not valid UTF-8")?,
        "/tmp/my_cache"
    );
    Ok(())
}

// Read status is preserved when the feed is saved again (re-fetch).
#[test]
fn test_save_cache_preserves_read_status() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let feed_url = "https://example.com/feed.xml";
    let feed = sample_feed()?;

    cache.save_feed(feed_url, &feed, None, None)?;
    cache.set_read_status(feed_url, "https://example.com/new", true)?;

    cache.save_feed(feed_url, &feed, None, None)?;

    let loaded = cache.load_feed(feed_url).context("cache not found")?;
    let entry = loaded
        .articles
        .iter()
        .find(|e| e.url == "https://example.com/new")
        .context("article not found")?;
    assert!(entry.read);
    Ok(())
}

// set_read_status toggles read/unread and persists the change.
#[test]
fn test_set_read_status() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let feed_url = "https://example.com/feed.xml";
    let feed = sample_feed()?;

    cache.save_feed(feed_url, &feed, None, None)?;

    let article_url = "https://example.com/new";
    cache.set_read_status(feed_url, article_url, true)?;

    let loaded = cache.load_feed(feed_url).context("cache not found")?;
    let entry = loaded
        .articles
        .iter()
        .find(|e| e.url == article_url)
        .context("article not found")?;
    assert!(entry.read);

    cache.set_read_status(feed_url, article_url, false)?;

    let loaded = cache.load_feed(feed_url).context("cache not found")?;
    let entry = loaded
        .articles
        .iter()
        .find(|e| e.url == article_url)
        .context("article not found")?;
    assert!(!entry.read);
    Ok(())
}

// ETag and Last-Modified are saved and can be loaded back.
#[test]
fn test_save_and_load_http_metadata() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cache = CacheStore::new(dir.path());
    let url = "https://example.com/feed.xml";
    let feed = sample_feed()?;

    let etag = Some("\"abc123\"");
    let last_modified = Some("Sat, 01 Mar 2026 00:00:00 GMT");

    cache.save_feed(url, &feed, etag, last_modified)?;

    let meta = cache.load_http_metadata(url);
    assert_eq!(meta.etag.as_deref(), etag);
    assert_eq!(meta.last_modified.as_deref(), last_modified);
    Ok(())
}
