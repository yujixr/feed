use anyhow::{Context, Result};
use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub feeds: Vec<FeedEntry>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub content: ContentConfig,
    #[serde(default)]
    pub tui: TuiConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extractor: Option<ExtractorMethod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    /// Article retention period (days). 0=forever, -1=disabled
    #[serde(default = "CacheConfig::default_retention_days")]
    pub retention_days: i32,
    /// Cache directory path. If omitted, uses $XDG_DATA_HOME/feed/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            path: None,
        }
    }
}

impl CacheConfig {
    fn default_retention_days() -> i32 {
        90
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorMethod {
    #[default]
    Readability,
    RssContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentConfig {
    #[serde(default)]
    pub extractor: ExtractorMethod,
    #[serde(default = "ContentConfig::default_auto_mark_read")]
    pub auto_mark_read: bool,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            extractor: ExtractorMethod::default(),
            auto_mark_read: true,
        }
    }
}

impl ContentConfig {
    fn default_auto_mark_read() -> bool {
        true
    }
}

/// TUI-specific settings.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TuiConfig {
    /// Auto-refresh interval in seconds. 0 = disabled (default).
    #[serde(default)]
    pub auto_refresh_interval: u64,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config = serde_norway::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }
        let content = serde_norway::to_string(self).context("Failed to serialize config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    pub fn default_path() -> Result<PathBuf> {
        let strategy =
            etcetera::choose_base_strategy().context("Could not determine home directory")?;
        Ok(strategy.config_dir().join("feed").join("config.yaml"))
    }

    pub fn resolve_config_path() -> Result<PathBuf> {
        Self::default_path()
    }

    pub fn add_feed(&mut self, entry: FeedEntry) {
        self.feeds.retain(|f| f.url != entry.url);
        self.feeds.push(entry);
    }

    pub fn remove_feed(&mut self, target: &str) -> bool {
        let target_lower = target.to_lowercase();
        let before = self.feeds.len();
        self.feeds
            .retain(|f| f.name.to_lowercase() != target_lower && f.url != target);
        self.feeds.len() < before
    }

    pub fn find_feed(&self, target: &str) -> Option<&FeedEntry> {
        let target_lower = target.to_lowercase();
        self.feeds
            .iter()
            .find(|f| f.name.to_lowercase() == target_lower || f.url == target)
    }

    pub fn feeds_by_tag(&self, tag: &str) -> Vec<&FeedEntry> {
        let tag_lower = tag.to_lowercase();
        self.feeds
            .iter()
            .filter(|f| f.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .collect()
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.feeds.iter().flat_map(|f| f.tags.clone()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}
