use anyhow::Context;
use chrono::Utc;
use feed::article::Article;
use feed::article_store::{ArticleStore, FilterParams};
use feed::cache::CacheStore;
use feed::config::{Config, ExtractorMethod, FeedEntry};
use feed::feed_source::{RawEntry, RawFeed};
use tempfile::tempdir;

fn make_article(title: &str, read: bool, days_ago: i64) -> Article {
    Article {
        title: title.to_string(),
        url: format!("https://example.com/{}", title.replace(' ', "-")),
        published: Some(Utc::now() - chrono::Duration::days(days_ago)),
        feed_url: "https://example.com/feed".to_string(),
        feed_name: "Test Blog".to_string(),
        extractor: ExtractorMethod::default(),
        read,
        rss_content: None,
    }
}

#[test]
fn test_query_filters_read_articles() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("unread1", false, 1),
        make_article("read1", true, 2),
        make_article("unread2", false, 3),
    ]);

    let params = FilterParams::default(); // show_read=false
    let indices = store.query(&params);
    assert_eq!(indices.len(), 2);
    for &i in &indices {
        assert!(!store.get(i).context("article not found")?.read);
    }
    Ok(())
}

#[test]
fn test_query_show_all_includes_read() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("unread1", false, 1),
        make_article("read1", true, 2),
    ]);

    let params = FilterParams {
        show_read: true,
        ..Default::default()
    };
    let indices = store.query(&params);
    assert_eq!(indices.len(), 2);
    Ok(())
}

#[test]
fn test_query_from_date_filter() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("recent", false, 1),
        make_article("old", false, 30),
    ]);

    let cutoff = Utc::now() - chrono::Duration::days(7);
    let params = FilterParams {
        show_read: true,
        from: Some(cutoff),
        limit: None,
    };
    let indices = store.query(&params);
    assert_eq!(indices.len(), 1);
    assert_eq!(
        store.get(indices[0]).context("article not found")?.title,
        "recent"
    );
    Ok(())
}

#[test]
fn test_query_limit() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("a", false, 1),
        make_article("b", false, 2),
        make_article("c", false, 3),
    ]);

    let params = FilterParams {
        show_read: true,
        from: None,
        limit: Some(2),
    };
    let indices = store.query(&params);
    assert_eq!(indices.len(), 2);
    Ok(())
}

#[test]
fn test_query_combined_filters() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("recent-unread", false, 1),
        make_article("recent-read", true, 2),
        make_article("old-unread", false, 30),
        make_article("old-read", true, 31),
    ]);

    let cutoff = Utc::now() - chrono::Duration::days(7);
    let params = FilterParams {
        show_read: false,
        from: Some(cutoff),
        limit: Some(10),
    };
    let indices = store.query(&params);
    assert_eq!(indices.len(), 1);
    assert_eq!(
        store.get(indices[0]).context("article not found")?.title,
        "recent-unread"
    );
    Ok(())
}

#[tokio::test]
async fn test_mark_read() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![
        make_article("article1", false, 1),
        make_article("article2", false, 2),
    ]);

    store.mark_read(0);

    let all = store.query(&FilterParams {
        show_read: true,
        ..Default::default()
    });
    assert!(store.get(all[0]).context("article1 not found")?.read); // article1 now read
    assert!(!store.get(all[1]).context("article2 not found")?.read); // article2 unchanged
    Ok(())
}

#[tokio::test]
async fn test_toggle_read() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.path().to_path_buf());
    store.set_articles(vec![make_article("article1", false, 1)]);

    store.toggle_read(0);
    let all = store.query(&FilterParams {
        show_read: true,
        ..Default::default()
    });
    assert!(store.get(all[0]).context("article not found")?.read);

    store.toggle_read(0);
    let all = store.query(&FilterParams {
        show_read: true,
        ..Default::default()
    });
    assert!(!store.get(all[0]).context("article not found")?.read);
    Ok(())
}

// --- load_from_cache tests (via ArticleStore::fetch with cached_only=true) ---

#[tokio::test]
async fn test_fetch_cached_only_loads_articles() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cache = CacheStore::new(dir.path());
    let feed_url = "https://example.com/feed.xml";

    // Pre-populate cache
    let raw_feed = RawFeed {
        title: "Cached Blog".to_string(),
        entries: vec![
            RawEntry {
                title: "Post A".to_string(),
                url: "https://example.com/a".to_string(),
                published: Some(Utc::now()),
                rss_content: Some("<p>content A</p>".to_string()),
            },
            RawEntry {
                title: "Post B".to_string(),
                url: "https://example.com/b".to_string(),
                published: Some(Utc::now() - chrono::Duration::days(1)),
                rss_content: None,
            },
        ],
        etag: None,
        last_modified: None,
    };
    cache.save_feed(feed_url, &raw_feed, None, None)?;

    // Mark one as read
    cache.set_read_status(feed_url, "https://example.com/a", true)?;

    let feeds = vec![FeedEntry {
        name: "Cached Blog".to_string(),
        url: feed_url.to_string(),
        tags: vec![],
        extractor: None,
    }];
    let mut store = ArticleStore::new(feeds, Config::default(), dir.path().to_path_buf());
    store.fetch(true).await;

    assert_eq!(store.len(), 2);

    // Sorted by published date descending — Post A is newer
    let first = store.get(0).context("first article not found")?;
    assert_eq!(first.title, "Post A");
    assert_eq!(first.feed_url, feed_url);
    assert_eq!(first.feed_name, "Cached Blog");
    assert!(first.read);
    assert_eq!(first.rss_content, Some("<p>content A</p>".to_string()));

    let second = store.get(1).context("second article not found")?;
    assert_eq!(second.title, "Post B");
    assert!(!second.read);
    assert!(second.rss_content.is_none());

    Ok(())
}

#[tokio::test]
async fn test_fetch_cached_only_empty_cache() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let feeds = vec![FeedEntry {
        name: "Empty".to_string(),
        url: "https://example.com/empty".to_string(),
        tags: vec![],
        extractor: None,
    }];
    let mut store = ArticleStore::new(feeds, Config::default(), dir.path().to_path_buf());
    store.fetch(true).await;

    assert!(store.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_fetch_cached_only_uses_feed_extractor() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cache = CacheStore::new(dir.path());
    let feed_url = "https://example.com/feed.xml";

    let raw_feed = RawFeed {
        title: "Blog".to_string(),
        entries: vec![RawEntry {
            title: "Post".to_string(),
            url: "https://example.com/post".to_string(),
            published: Some(Utc::now()),
            rss_content: None,
        }],
        etag: None,
        last_modified: None,
    };
    cache.save_feed(feed_url, &raw_feed, None, None)?;

    // Set per-feed extractor to RssContent
    let feeds = vec![FeedEntry {
        name: "Blog".to_string(),
        url: feed_url.to_string(),
        tags: vec![],
        extractor: Some(ExtractorMethod::RssContent),
    }];
    let mut store = ArticleStore::new(feeds, Config::default(), dir.path().to_path_buf());
    store.fetch(true).await;

    assert_eq!(store.len(), 1);
    let article = store.get(0).context("article not found")?;
    assert!(matches!(article.extractor, ExtractorMethod::RssContent));
    Ok(())
}

#[tokio::test]
async fn test_fetch_cached_only_multiple_feeds() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cache = CacheStore::new(dir.path());

    // Populate two feeds
    let feed1 = RawFeed {
        title: "Blog A".to_string(),
        entries: vec![RawEntry {
            title: "A1".to_string(),
            url: "https://a.com/1".to_string(),
            published: Some(Utc::now()),
            rss_content: None,
        }],
        etag: None,
        last_modified: None,
    };
    let feed2 = RawFeed {
        title: "Blog B".to_string(),
        entries: vec![RawEntry {
            title: "B1".to_string(),
            url: "https://b.com/1".to_string(),
            published: Some(Utc::now() - chrono::Duration::hours(1)),
            rss_content: None,
        }],
        etag: None,
        last_modified: None,
    };
    cache.save_feed("https://a.com/feed", &feed1, None, None)?;
    cache.save_feed("https://b.com/feed", &feed2, None, None)?;

    let feeds = vec![
        FeedEntry {
            name: "Blog A".to_string(),
            url: "https://a.com/feed".to_string(),
            tags: vec![],
            extractor: None,
        },
        FeedEntry {
            name: "Blog B".to_string(),
            url: "https://b.com/feed".to_string(),
            tags: vec![],
            extractor: None,
        },
    ];
    let mut store = ArticleStore::new(feeds, Config::default(), dir.path().to_path_buf());
    store.fetch(true).await;

    assert_eq!(store.len(), 2);
    // Sorted by published date descending — A1 is newer
    assert_eq!(store.get(0).context("first")?.title, "A1");
    assert_eq!(store.get(0).context("first")?.feed_name, "Blog A");
    assert_eq!(store.get(1).context("second")?.title, "B1");
    assert_eq!(store.get(1).context("second")?.feed_name, "Blog B");
    Ok(())
}
