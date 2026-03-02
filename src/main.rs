use anyhow::Result;
use clap::Parser;
use feed::cache;
use feed::cli::{Cli, Commands};
use feed::commands;
use feed::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = match &cli.config {
        Some(p) => p.clone(),
        None => Config::resolve_config_path()?,
    };

    let config = Config::load(&config_path)?;
    let data_dir = cache::data_dir(config.cache.path.as_deref())?;

    // Purge old articles on startup
    if config.cache.retention_days > 0 {
        let cache_store = cache::CacheStore::new(&data_dir);
        let retention = config.cache.retention_days;
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let _ = cache_store.purge_old_entries(retention);
            })
            .await
        });
    }

    match &cli.command {
        Some(Commands::FetchFeed { url }) => {
            commands::cmd_fetch_feed(url, &config, &data_dir, cli.limit).await?;
        }
        Some(Commands::FetchArticle { url }) => {
            commands::cmd_fetch_article(url).await?;
        }
        Some(Commands::Add { url, name, tag }) => {
            commands::cmd_add(url, name.as_deref(), tag, &config_path).await?;
        }
        Some(Commands::Remove { target }) => {
            commands::cmd_remove(target, &config_path)?;
        }
        Some(Commands::List) => {
            commands::cmd_list(&config)?;
        }
        Some(Commands::Tags) => {
            commands::cmd_tags(&config)?;
        }
        None => {
            commands::cmd_default(&cli, &config, &data_dir).await?;
        }
    }

    Ok(())
}
