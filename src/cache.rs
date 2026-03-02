use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::feed_source::RawFeed;

#[derive(Clone, Debug)]
pub struct CacheStore {
    data_dir: PathBuf,
}

impl CacheStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub(crate) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    fn feed_path(&self, url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        self.data_dir.join(format!("{}.json", &hash[..16]))
    }

    pub fn load_http_metadata(&self, url: &str) -> HttpMetadata {
        let path = self.feed_path(url);
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                return HttpMetadata {
                    etag: None,
                    last_modified: None,
                }
            }
        };
        match serde_json::from_str::<CachedFeed>(&content) {
            Ok(cached) => HttpMetadata {
                etag: cached.etag,
                last_modified: cached.last_modified,
            },
            Err(_) => HttpMetadata {
                etag: None,
                last_modified: None,
            },
        }
    }

    /// Load cached articles for a feed URL
    pub fn load_feed(&self, url: &str) -> Option<CachedFeed> {
        let path = self.feed_path(url);
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save fetched results to cache, merging with existing entries (deduplicated by URL)
    pub fn save_feed(
        &self,
        url: &str,
        feed: &RawFeed,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<()> {
        fs::create_dir_all(&self.data_dir).with_context(|| {
            format!(
                "Failed to create data directory: {}",
                self.data_dir.display()
            )
        })?;

        let path = self.feed_path(url);

        // Load existing cache
        let mut articles: Vec<CachedArticle> = if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(existing) = serde_json::from_str::<CachedFeed>(&content) {
                existing.articles
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Build set of URLs present in the current fetch
        let fetched_urls: HashSet<&str> = feed.entries.iter().map(|e| e.url.as_str()).collect();

        let now = Utc::now();

        // Update last_seen for existing entries that are still in the feed
        for article in &mut articles {
            if fetched_urls.contains(article.url.as_str()) {
                article.last_seen = now;
            }
        }

        // Merge new entries (deduplicate by URL, preserve read status)
        let existing_urls: HashSet<String> = articles.iter().map(|e| e.url.clone()).collect();
        for entry in &feed.entries {
            if !entry.url.is_empty() && !existing_urls.contains(&entry.url) {
                articles.push(CachedArticle {
                    title: entry.title.clone(),
                    url: entry.url.clone(),
                    published: entry.published,
                    read: false,
                    rss_content: entry.rss_content.clone(),
                    last_seen: now,
                });
            }
        }

        // Sort by datetime (newest first)
        articles.sort_by(|a, b| b.published.cmp(&a.published));

        let cached = CachedFeed {
            feed_url: url.to_string(),
            feed_title: feed.title.clone(),
            last_fetched: Utc::now(),
            etag: etag.map(String::from),
            last_modified: last_modified.map(String::from),
            articles,
        };

        let json = serde_json::to_string(&cached).context("Failed to serialize cache")?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write cache: {}", path.display()))?;

        Ok(())
    }

    /// Remove entries older than retention_days from all cache files
    pub fn purge_old_entries(&self, retention_days: i32) -> Result<()> {
        if retention_days <= 0 {
            return Ok(()); // 0=forever, negative=cache disabled
        }

        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);

        if !self.data_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = fs::read_to_string(&path)?;
            if let Ok(mut cached) = serde_json::from_str::<CachedFeed>(&content) {
                let before = cached.articles.len();
                cached.articles.retain(|e| e.last_seen > cutoff);
                if cached.articles.len() != before {
                    if cached.articles.is_empty() {
                        fs::remove_file(&path)?;
                    } else {
                        let json = serde_json::to_string(&cached)?;
                        fs::write(&path, json)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Set the read status of an article in a specific feed's cache file (O(1) file lookup).
    pub fn set_read_status(&self, feed_url: &str, article_url: &str, read: bool) -> Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        let path = self.feed_path(feed_url);
        if !path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&path)?;
        if let Ok(mut cached) = serde_json::from_str::<CachedFeed>(&content) {
            let mut changed = false;
            for e in &mut cached.articles {
                if e.url == article_url && e.read != read {
                    e.read = read;
                    changed = true;
                }
            }
            if changed {
                let json = serde_json::to_string(&cached)?;
                fs::write(&path, json)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedArticle {
    pub title: String,
    pub url: String,
    pub published: Option<DateTime<Utc>>,
    #[serde(default)]
    pub read: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rss_content: Option<String>,
    /// Timestamp when this URL was last seen in a feed fetch.
    #[serde(default = "Utc::now")]
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedFeed {
    pub feed_url: String,
    pub feed_title: String,
    pub last_fetched: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    pub articles: Vec<CachedArticle>,
}

pub struct HttpMetadata {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Determine data directory.
/// Priority: config cache.path > XDG_DATA_HOME/feed/
pub fn data_dir(config_path: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = config_path {
        Ok(PathBuf::from(p))
    } else {
        let strategy =
            etcetera::choose_base_strategy().context("Could not determine home directory")?;
        Ok(strategy.data_dir().join("feed"))
    }
}
