use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "feed",
    about = "A simple RSS/Atom feed reader for the terminal"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Filter by feed name
    #[arg(long)]
    pub name: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Show cached articles only (no network access)
    #[arg(long)]
    pub cached: bool,

    /// Show all articles (including read)
    #[arg(long)]
    pub all: bool,

    /// Show articles from this date onwards (YYYY-MM-DD)
    #[arg(long)]
    pub from: Option<String>,

    /// Use CLI output instead of TUI
    #[arg(long)]
    pub cli: bool,

    /// Limit number of entries to display
    #[arg(long, global = true)]
    pub limit: Option<usize>,

    /// Path to config file
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Fetch and display a feed by URL
    FetchFeed {
        /// Feed URL
        url: String,
    },
    /// Fetch and display an article's text content
    FetchArticle {
        /// Article URL
        url: String,
    },
    /// Register a new feed
    Add {
        /// Feed URL
        url: String,

        /// Display name for the feed (auto-detected if omitted)
        #[arg(long)]
        name: Option<String>,

        /// Tag for the feed (can be specified multiple times)
        #[arg(long, short)]
        tag: Vec<String>,
    },
    /// Remove a registered feed
    Remove {
        /// Feed name or URL
        target: String,
    },
    /// List registered feeds
    List,
    /// List all tags
    Tags,
}
