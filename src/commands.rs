use anyhow::{bail, Context, Result};
use chrono::{TimeZone, Utc};
use reqwest::Client;
use std::time::Duration;

use crate::article_store::{ArticleStore, FilterParams};
use crate::cache::CacheStore;
use crate::cache::HttpMetadata;
use crate::cli::Cli;
use crate::config::{Config, FeedEntry};
use crate::display;
use crate::feed_source::{self, FetchResult};

pub(crate) fn build_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap_or_else(|_| Client::new())
}

pub async fn cmd_default(cli: &Cli, config: &Config, data_dir: &std::path::Path) -> Result<()> {
    let feeds: Vec<&FeedEntry> = match (cli.tag.as_deref(), cli.name.as_deref()) {
        (Some(t), _) => config.feeds_by_tag(t),
        (_, Some(n)) => config.find_feed(n).into_iter().collect(),
        _ => config.feeds.iter().collect(),
    };

    if feeds.is_empty() {
        if config.feeds.is_empty() {
            eprintln!("No feeds registered. Use `feed add <url>` to add one.");
        } else {
            eprintln!("No matching feeds found.");
        }
        return Ok(());
    }

    let owned_feeds: Vec<FeedEntry> = feeds.into_iter().cloned().collect();

    let from = match &cli.from {
        Some(from_str) => {
            let from_date = from_str.parse::<chrono::NaiveDate>().map_err(|_| {
                anyhow::anyhow!("Invalid date format: {}. Use YYYY-MM-DD", from_str)
            })?;
            Some(Utc.from_utc_datetime(&from_date.and_hms_opt(0, 0, 0).context("Invalid time")?))
        }
        None => None,
    };

    let filter_params = FilterParams {
        show_read: cli.all,
        from,
        limit: cli.limit,
    };

    let mut store = ArticleStore::new(owned_feeds, config.clone(), data_dir.to_path_buf());

    if cli.cli {
        store.fetch(cli.cached).await;
        let articles = store.query_articles(&filter_params);
        if articles.is_empty() {
            eprintln!("No articles found.");
            return Ok(());
        }
        let items: Vec<display::DisplayItem> = articles
            .iter()
            .map(|a| display::DisplayItem {
                title: &a.title,
                url: &a.url,
                published: a.published,
            })
            .collect();
        let title = format!("{} feeds", store.feeds().len());
        println!("{}", display::render_article_list(&title, &items));
    } else {
        store.fetch(true).await; // Cache first for instant TUI
        crate::tui::run(store, filter_params).await?;
    }

    Ok(())
}

pub async fn cmd_fetch_article(url: &str) -> Result<()> {
    let client = build_client();
    let width = terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80);
    let (title, html) = crate::article::extract_readable_html(&client, url).await?;
    let text = crate::article::html_to_text(&html, width.saturating_sub(2));
    if !title.is_empty() {
        println!("\n {}", title);
    }
    println!(" {}\n", url);
    for line in text.lines() {
        println!(" {}", line);
    }
    Ok(())
}

pub async fn cmd_fetch_feed(
    url: &str,
    config: &Config,
    data_dir: &std::path::Path,
    limit: Option<usize>,
) -> Result<()> {
    let client = build_client();
    eprintln!("Resolving feed URL...");
    let feed_url = feed_source::resolve_feed_url(&client, url).await?;
    if feed_url != url {
        eprintln!("Discovered feed: {}", feed_url);
    }

    let metadata = HttpMetadata {
        etag: None,
        last_modified: None,
    };
    let result = feed_source::fetch(&client, &feed_url, &metadata).await?;
    let feed = match result {
        FetchResult::Fetched(feed) => feed,
        FetchResult::NotModified => {
            eprintln!("Feed not modified");
            return Ok(());
        }
    };

    if config.cache.retention_days >= 0 {
        CacheStore::new(data_dir).save_feed(
            &feed_url,
            &feed,
            feed.etag.as_deref(),
            feed.last_modified.as_deref(),
        )?;
    }

    let entries: Vec<_> = feed
        .entries
        .iter()
        .take(limit.unwrap_or(usize::MAX))
        .collect();
    let items: Vec<display::DisplayItem> = entries
        .iter()
        .map(|e| display::DisplayItem {
            title: &e.title,
            url: &e.url,
            published: e.published,
        })
        .collect();
    println!("{}", display::render_article_list(&feed.title, &items));
    Ok(())
}

pub async fn cmd_add(
    url: &str,
    name: Option<&str>,
    tags: &[String],
    config_path: &std::path::Path,
) -> Result<()> {
    let client = build_client();
    eprintln!("Resolving feed URL...");
    let feed_url = feed_source::resolve_feed_url(&client, url).await?;
    if feed_url != url {
        eprintln!("Discovered feed: {}", feed_url);
    }

    let feed_name = match name {
        Some(n) => n.to_string(),
        None => {
            eprintln!("Fetching feed title...");
            let metadata = HttpMetadata {
                etag: None,
                last_modified: None,
            };
            let result = feed_source::fetch(&client, &feed_url, &metadata).await?;
            match result {
                FetchResult::Fetched(feed) => feed.title,
                FetchResult::NotModified => "(unknown)".to_string(),
            }
        }
    };

    let mut config = Config::load(config_path)?;
    config.add_feed(FeedEntry {
        name: feed_name.clone(),
        url: feed_url.to_string(),
        tags: tags.to_vec(),
        extractor: None,
    });
    config.save(config_path)?;

    eprintln!("Added: {} ({})", feed_name, feed_url);
    Ok(())
}

pub fn cmd_remove(target: &str, config_path: &std::path::Path) -> Result<()> {
    let mut config = Config::load(config_path)?;
    if config.remove_feed(target) {
        config.save(config_path)?;
        eprintln!("Removed: {}", target);
    } else {
        bail!("Feed not found: {}", target);
    }
    Ok(())
}

pub fn cmd_list(config: &Config) -> Result<()> {
    println!("{}", display::render_feed_list(config));
    Ok(())
}

pub fn cmd_tags(config: &Config) -> Result<()> {
    println!("{}", display::render_tag_list(config));
    Ok(())
}
