# feed

A simple RSS/Atom feed reader for the terminal.

## Install

```bash
cargo install --path .
```

## How to use

### Read articles (default: TUI mode)

Run `feed` without any subcommand. It shows articles from all your feeds.

| Option                | What it does                         |
| --------------------- | ------------------------------------ |
| `--config <PATH>`     | Set the config file path             |
| `--name <NAME>`       | Show only feeds that match this name |
| `--tag <TAG>`         | Show only feeds that have this tag   |
| `--from <YYYY-MM-DD>` | Show only articles after this date   |
| `--all`               | Include articles you already read    |
| `--cached`            | Show only cached articles            |
| `--limit <N>`         | Limit how many articles to show      |
| `--cli`               | Use CLI output instead of TUI        |

### Manage feeds

```bash
feed add <url>                         # Add a feed (title is fetched automatically)
feed add <url> --name "My Feed"        # Add a feed with a custom name
feed add <url> --tag tech --tag rust   # Add a feed with tags
feed remove <name or url>              # Remove a feed
feed list                              # List all feeds
feed tags                              # List all tags
```

### Fetch once

```bash
feed fetch-feed <url>                  # Show entries from a feed URL
feed fetch-article <url>               # Extract and show article content
```

## TUI key bindings

### Article list

| Key                               | Action                        |
| --------------------------------- | ----------------------------- |
| `j` / `Down`                      | Next article                  |
| `k` / `Up`                        | Previous article              |
| `Enter` / `Space`                 | Open article                  |
| `o`                               | Open in browser               |
| `m`                               | Toggle read / unread          |
| `a`                               | Toggle show all / unread only |
| `r`                               | Refresh feeds                 |
| `q` / `Esc` / `Ctrl-c` / `Ctrl-d` | Quit                          |

### Article view

| Key                 | Action               |
| ------------------- | -------------------- |
| `j` / `Down`        | Scroll down          |
| `k` / `Up`          | Scroll up            |
| `Space`             | Page down            |
| `h` / `Left`        | Previous article     |
| `l` / `Right`       | Next article         |
| `o`                 | Open in browser      |
| `m`                 | Toggle read / unread |
| `q` / `Esc`         | Back to list         |
| `Ctrl-c` / `Ctrl-d` | Quit                 |

## Config file

The program looks for a config file in this order:

1. The path you give with `--config <path>`
2. `$XDG_CONFIG_HOME/feed/config.yaml` (default: `~/.config/feed/config.yaml`)

```yaml
feeds:
  - name: "Rust Blog"
    url: "https://blog.rust-lang.org/feed.xml"
    tags: ["tech", "rust"]
    # extractor: rss_content  # You can set this per feed

cache:
  retention_days: 90   # 0: keep forever, -1: no cache
  # path: /path/to/cache
  # Default cache location: $XDG_DATA_HOME/feed/ (default: ~/.local/share/feed/)

content:
  extractor: readability  # "readability" (default) or "rss_content"
  auto_mark_read: true    # Mark articles as read when you open them (default: true)
  # If readability fails, it falls back to RSS content automatically

tui:
  auto_refresh_interval: 300  # Auto-refresh interval in seconds. 0: off (default)
```
