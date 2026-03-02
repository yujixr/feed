use futures::future::join_all;

use crate::article::Article;
use crate::cache::CacheStore;
use crate::config::ExtractorMethod;
use crate::feed_source::{self, FetchResult};

use super::ArticleStore;

fn load_from_cache(
    cache: &CacheStore,
    feed_url: &str,
    feed_name: &str,
    extractor_method: &ExtractorMethod,
) -> Vec<Article> {
    cache
        .load_feed(feed_url)
        .map(|c| {
            c.articles
                .into_iter()
                .map(|info| Article {
                    title: info.title,
                    url: info.url,
                    published: info.published,
                    feed_url: feed_url.to_string(),
                    feed_name: feed_name.to_string(),
                    extractor: extractor_method.clone(),
                    read: info.read,
                    rss_content: info.rss_content,
                })
                .collect()
        })
        .unwrap_or_default()
}

impl ArticleStore {
    /// Fetch all feeds and update internal article list.
    /// If cached_only=true, only loads from cache (no network).
    pub async fn fetch(&mut self, cached_only: bool) {
        let cache_enabled = self.config().cache.retention_days >= 0;
        let default_extractor = self.config().content.extractor.clone();

        if cached_only {
            let futures: Vec<_> = self
                .feeds()
                .iter()
                .map(|feed_entry| {
                    let cache = self.cache().clone();
                    let feed_url = feed_entry.url.clone();
                    let feed_name = feed_entry.name.clone();
                    let extractor_method = feed_entry
                        .extractor
                        .as_ref()
                        .unwrap_or(&default_extractor)
                        .clone();
                    tokio::task::spawn_blocking(move || {
                        load_from_cache(&cache, &feed_url, &feed_name, &extractor_method)
                    })
                })
                .collect();

            let mut all_articles: Vec<Article> = Vec::new();
            for articles in join_all(futures).await.into_iter().flatten() {
                all_articles.extend(articles);
            }
            all_articles.sort_by(|a, b| b.published.cmp(&a.published));
            self.set_articles(all_articles);
            return;
        }

        let futures: Vec<_> = self
            .feeds()
            .iter()
            .map(|feed_entry| {
                let client = self.client().clone();
                let feed_url = feed_entry.url.clone();
                let cache = self.cache().clone();
                async move {
                    let metadata = cache.load_http_metadata(&feed_url);
                    feed_source::fetch(&client, &feed_url, &metadata).await
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut all_articles: Vec<Article> = Vec::new();

        for (feed_entry, result) in self.feeds().iter().zip(results) {
            let extractor_method = feed_entry
                .extractor
                .as_ref()
                .unwrap_or(&default_extractor)
                .clone();
            let feed_url = &feed_entry.url;
            let feed_name = &feed_entry.name;

            match result {
                Ok(FetchResult::Fetched(feed)) => {
                    if cache_enabled {
                        let save_cache = self.cache().clone();
                        let save_url = feed_url.to_string();
                        let save_feed = feed.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            save_cache.save_feed(
                                &save_url,
                                &save_feed,
                                save_feed.etag.as_deref(),
                                save_feed.last_modified.as_deref(),
                            )
                        })
                        .await;
                        all_articles.extend(load_from_cache(
                            self.cache(),
                            feed_url,
                            feed_name,
                            &extractor_method,
                        ));
                    } else {
                        all_articles.extend(feed.entries.into_iter().map(|e| Article {
                            title: e.title,
                            url: e.url,
                            published: e.published,
                            feed_url: feed_url.to_string(),
                            feed_name: feed_name.to_string(),
                            extractor: extractor_method.clone(),
                            read: false,
                            rss_content: e.rss_content,
                        }));
                    }
                }
                Ok(FetchResult::NotModified) => {
                    all_articles.extend(load_from_cache(
                        self.cache(),
                        feed_url,
                        feed_name,
                        &extractor_method,
                    ));
                }
                Err(e) => {
                    eprintln!("Error fetching {}: {}", feed_name, e);
                    all_articles.extend(load_from_cache(
                        self.cache(),
                        feed_url,
                        feed_name,
                        &extractor_method,
                    ));
                }
            }
        }

        all_articles.sort_by(|a, b| b.published.cmp(&a.published));
        self.set_articles(all_articles);
    }
}
