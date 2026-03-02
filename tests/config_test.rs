use feed::config::{Config, ContentConfig, ExtractorMethod, FeedEntry};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_load_nonexistent_returns_empty() -> anyhow::Result<()> {
    let path = Path::new("/tmp/nonexistent_feed_config.yaml");
    let config = Config::load(path)?;
    assert!(config.feeds.is_empty());
    Ok(())
}

#[test]
fn test_save_and_load_roundtrip() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("config.yaml");

    let mut config = Config {
        feeds: vec![],
        ..Default::default()
    };
    config.add_feed(FeedEntry {
        name: "Test".to_string(),
        url: "https://example.com/feed.xml".to_string(),
        tags: vec!["tech".to_string()],
        extractor: None,
    });
    config.save(&path)?;

    let loaded = Config::load(&path)?;
    assert_eq!(loaded.feeds.len(), 1);
    assert_eq!(loaded.feeds[0].name, "Test");
    assert_eq!(loaded.feeds[0].tags, vec!["tech"]);
    Ok(())
}

#[test]
fn test_add_feed_deduplicates_by_url() {
    let mut config = Config {
        feeds: vec![],
        ..Default::default()
    };
    config.add_feed(FeedEntry {
        name: "Old".to_string(),
        url: "https://example.com/feed.xml".to_string(),
        tags: vec![],
        extractor: None,
    });
    config.add_feed(FeedEntry {
        name: "New".to_string(),
        url: "https://example.com/feed.xml".to_string(),
        tags: vec!["updated".to_string()],
        extractor: None,
    });
    assert_eq!(config.feeds.len(), 1);
    assert_eq!(config.feeds[0].name, "New");
}

#[test]
fn test_remove_feed_by_name_case_insensitive() {
    let mut config = Config {
        feeds: vec![FeedEntry {
            name: "Rust Blog".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            tags: vec![],
            extractor: None,
        }],
        ..Default::default()
    };
    assert!(config.remove_feed("rust blog"));
    assert!(config.feeds.is_empty());
}

#[test]
fn test_find_feed_case_insensitive() {
    let config = Config {
        feeds: vec![FeedEntry {
            name: "Rust Blog".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            tags: vec![],
            extractor: None,
        }],
        ..Default::default()
    };
    assert!(config.find_feed("rust blog").is_some());
    assert!(config.find_feed("https://example.com/feed.xml").is_some());
    assert!(config.find_feed("nonexistent").is_none());
}

#[test]
fn test_feeds_by_tag() {
    let config = Config {
        feeds: vec![
            FeedEntry {
                name: "A".to_string(),
                url: "https://a.com/feed".to_string(),
                tags: vec!["tech".to_string(), "rust".to_string()],
                extractor: None,
            },
            FeedEntry {
                name: "B".to_string(),
                url: "https://b.com/feed".to_string(),
                tags: vec!["news".to_string()],
                extractor: None,
            },
        ],
        ..Default::default()
    };
    let tech = config.feeds_by_tag("tech");
    assert_eq!(tech.len(), 1);
    assert_eq!(tech[0].name, "A");
}

#[test]
fn test_all_tags_sorted_and_deduped() {
    let config = Config {
        feeds: vec![
            FeedEntry {
                name: "A".to_string(),
                url: "https://a.com".to_string(),
                tags: vec!["rust".to_string(), "tech".to_string()],
                extractor: None,
            },
            FeedEntry {
                name: "B".to_string(),
                url: "https://b.com".to_string(),
                tags: vec!["tech".to_string(), "news".to_string()],
                extractor: None,
            },
        ],
        ..Default::default()
    };
    assert_eq!(config.all_tags(), vec!["news", "rust", "tech"]);
}

#[test]
fn test_config_roundtrip_yaml() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let config_path = dir.path().join("config.yaml");

    let yaml =
        "feeds:\n- name: Test Feed\n  url: https://example.com/feed.xml\n  tags:\n  - tech\n  - rust\n";
    std::fs::write(&config_path, yaml)?;

    let content = std::fs::read_to_string(&config_path)?;
    assert!(content.contains("Test Feed"));
    assert!(content.contains("https://example.com/feed.xml"));
    assert!(content.contains("tech"));
    Ok(())
}

#[test]
fn test_cache_config_defaults() {
    let config = Config {
        feeds: vec![],
        ..Default::default()
    };
    assert_eq!(config.cache.retention_days, 90);
    assert!(config.cache.path.is_none());
}

#[test]
fn test_cache_config_with_path() -> anyhow::Result<()> {
    let yaml = "feeds: []\ncache:\n  path: /tmp/my_feed_cache\n";
    let config: Config = serde_yml::from_str(yaml)?;
    assert_eq!(config.cache.path.as_deref(), Some("/tmp/my_feed_cache"));
    Ok(())
}

#[test]
fn test_cache_config_path_not_serialized_when_none() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("config.yaml");
    let config = Config {
        feeds: vec![],
        ..Default::default()
    };
    config.save(&path)?;
    let content = std::fs::read_to_string(&path)?;
    assert!(!content.contains("path:"));
    Ok(())
}

#[test]
fn test_content_config_defaults() -> anyhow::Result<()> {
    let config: Config = serde_yml::from_str("feeds: []")?;
    assert_eq!(config.content.extractor, ExtractorMethod::Readability);
    Ok(())
}

#[test]
fn test_content_config_rss() -> anyhow::Result<()> {
    let yaml = "feeds: []\ncontent:\n  extractor: rss_content\n";
    let config: Config = serde_yml::from_str(yaml)?;
    assert_eq!(config.content.extractor, ExtractorMethod::RssContent);
    Ok(())
}

#[test]
fn test_auto_mark_read_default_true() -> anyhow::Result<()> {
    let config: Config = serde_yml::from_str("feeds: []")?;
    assert!(config.content.auto_mark_read);
    Ok(())
}

#[test]
fn test_auto_mark_read_explicit_false() -> anyhow::Result<()> {
    let yaml = "feeds: []\ncontent:\n  auto_mark_read: false\n";
    let config: Config = serde_yml::from_str(yaml)?;
    assert!(!config.content.auto_mark_read);
    Ok(())
}

#[test]
fn test_auto_mark_read_roundtrip() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("config.yaml");
    let config = Config {
        feeds: vec![],
        content: ContentConfig {
            extractor: ExtractorMethod::Readability,
            auto_mark_read: false,
        },
        ..Default::default()
    };
    config.save(&path)?;
    let loaded = Config::load(&path)?;
    assert!(!loaded.content.auto_mark_read);
    Ok(())
}

// --- TuiConfig ---

#[test]
fn test_tui_config_default_disabled() -> anyhow::Result<()> {
    let config: Config = serde_yml::from_str("feeds: []")?;
    assert_eq!(config.tui.auto_refresh_interval, 0);
    Ok(())
}

#[test]
fn test_tui_config_auto_refresh_interval() -> anyhow::Result<()> {
    let yaml = "feeds: []\ntui:\n  auto_refresh_interval: 300\n";
    let config: Config = serde_yml::from_str(yaml)?;
    assert_eq!(config.tui.auto_refresh_interval, 300);
    Ok(())
}

#[test]
fn test_tui_config_roundtrip() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("config.yaml");
    let config = Config {
        feeds: vec![],
        tui: feed::config::TuiConfig {
            auto_refresh_interval: 600,
        },
        ..Default::default()
    };
    config.save(&path)?;
    let loaded = Config::load(&path)?;
    assert_eq!(loaded.tui.auto_refresh_interval, 600);
    Ok(())
}
