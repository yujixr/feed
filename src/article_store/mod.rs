mod fetch;

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;

use crate::article::Article;
use crate::cache::CacheStore;
use crate::config::{Config, FeedEntry};

pub struct ArticleStore {
    articles: Vec<Article>,
    feeds: Vec<FeedEntry>,
    config: Config,
    cache: CacheStore,
    client: Client,
}

#[derive(Debug, Clone, Default)]
pub struct FilterParams {
    pub show_read: bool,
    pub from: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl ArticleStore {
    pub fn new(feeds: Vec<FeedEntry>, config: Config, data_dir: PathBuf) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self::with_client(feeds, config, data_dir, client)
    }

    pub(crate) fn with_client(
        feeds: Vec<FeedEntry>,
        config: Config,
        data_dir: PathBuf,
        client: Client,
    ) -> Self {
        Self {
            articles: Vec::new(),
            feeds,
            config,
            cache: CacheStore::new(data_dir),
            client,
        }
    }

    pub(crate) fn feeds(&self) -> &[FeedEntry] {
        &self.feeds
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn cache(&self) -> &CacheStore {
        &self.cache
    }

    pub(crate) fn data_dir(&self) -> &Path {
        self.cache.data_dir()
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// Replace internal articles (used in tests and for direct manipulation).
    pub fn set_articles(&mut self, articles: Vec<Article>) {
        self.articles = articles;
    }

    pub(crate) fn take_articles(&mut self) -> Vec<Article> {
        std::mem::take(&mut self.articles)
    }

    /// Get article by index.
    pub fn get(&self, index: usize) -> Option<&Article> {
        self.articles.get(index)
    }

    /// Number of articles.
    pub fn len(&self) -> usize {
        self.articles.len()
    }

    /// Whether the store has no articles.
    pub fn is_empty(&self) -> bool {
        self.articles.is_empty()
    }

    /// Mark an article as read by index (internal state + cache).
    pub fn mark_read(&mut self, index: usize) {
        if let Some(a) = self.articles.get_mut(index) {
            a.read = true;
            let cache = self.cache.clone();
            let feed_url = a.feed_url.clone();
            let url = a.url.clone();
            tokio::task::spawn_blocking(move || {
                let _ = cache.set_read_status(&feed_url, &url, true);
            });
        }
    }

    /// Toggle read status of an article by index. Returns the new read state.
    pub(crate) fn toggle_read(&mut self, index: usize) -> bool {
        if let Some(a) = self.articles.get_mut(index) {
            a.read = !a.read;
            let new_read = a.read;
            let cache = self.cache.clone();
            let feed_url = a.feed_url.clone();
            let url = a.url.clone();
            tokio::task::spawn_blocking(move || {
                let _ = cache.set_read_status(&feed_url, &url, new_read);
            });
            new_read
        } else {
            false
        }
    }

    /// Return indices of filtered articles. Does not modify internal state.
    pub fn query(&self, params: &FilterParams) -> Vec<usize> {
        let mut result: Vec<usize> = self
            .articles
            .iter()
            .enumerate()
            .filter(|(_, a)| params.show_read || !a.read)
            .filter(|(_, a)| match params.from {
                Some(from) => a.published.is_none_or(|dt| dt >= from),
                None => true,
            })
            .map(|(i, _)| i)
            .collect();

        if let Some(limit) = params.limit {
            result.truncate(limit);
        }

        result
    }

    /// Return cloned articles matching the filter (for CLI display compatibility).
    pub(crate) fn query_articles(&self, params: &FilterParams) -> Vec<Article> {
        self.query(params)
            .into_iter()
            .filter_map(|i| self.articles.get(i).cloned())
            .collect()
    }
}
