use chrono::{DateTime, Utc};
use terminal_size::{terminal_size, Width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::Config;

pub struct DisplayItem<'a> {
    pub title: &'a str,
    pub url: &'a str,
    pub published: Option<DateTime<Utc>>,
}

pub(crate) fn term_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

/// Wraps displayed text in an OSC 8 hyperlink escape sequence.
/// The terminal shows `text` but clicking it opens `url`.
pub(crate) fn osc8_hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}

pub fn render_article_list(title: &str, entries: &[DisplayItem]) -> String {
    let mut lines = Vec::new();

    lines.push(format!("\n {}\n", title));

    if entries.is_empty() {
        lines.push(" (no entries)".to_string());
        return lines.join("\n");
    }

    let width = term_width();
    let fixed = 15;
    let available = width.saturating_sub(fixed);

    let actual_max_title = entries
        .iter()
        .map(|e| display_width(e.title))
        .max()
        .unwrap_or(0);
    let max_title_width = actual_max_title.min(available / 2);

    for entry in entries {
        let date = entry
            .published
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "          ".to_string());
        let title_w = display_width(entry.title);
        let title_text = if title_w > max_title_width {
            truncate(entry.title, max_title_width)
        } else {
            entry.title.to_string()
        };
        let actual_title_w = display_width(&title_text);
        let title = osc8_hyperlink(entry.url, &title_text);
        let padding: String = " ".repeat(max_title_width.saturating_sub(actual_title_w));
        let url_budget = width.saturating_sub(fixed + max_title_width);
        let url_text = if display_width(entry.url) > url_budget && url_budget > 0 {
            truncate(entry.url, url_budget)
        } else {
            entry.url.to_string()
        };
        let url = osc8_hyperlink(entry.url, &url_text);
        lines.push(format!(" {}  {}{}  {}", date, title, padding, url));
    }

    lines.join("\n")
}

pub fn render_feed_list(config: &Config) -> String {
    let feeds: Vec<(String, String, Vec<String>)> = config
        .feeds
        .iter()
        .map(|f| (f.name.clone(), f.url.clone(), f.tags.clone()))
        .collect();

    if feeds.is_empty() {
        return " No feeds registered. Use `feed add <url>` to add one.".to_string();
    }

    let width = term_width();
    let available = width.saturating_sub(3);

    let actual_max_name = feeds
        .iter()
        .map(|(n, _, _)| display_width(n))
        .max()
        .unwrap_or(0);
    let max_name = actual_max_name.min(available * 3 / 10);
    let actual_max_url = feeds
        .iter()
        .map(|(_, u, _)| display_width(u))
        .max()
        .unwrap_or(0);
    let max_url = actual_max_url.min(available * 6 / 10);

    feeds
        .iter()
        .map(|(name, url, tags)| {
            let padded_name = pad_or_truncate(name, max_name);
            let url_truncated = truncate(url, max_url);
            let url_w = display_width(&url_truncated);
            let linked_url = osc8_hyperlink(url, &url_truncated);
            let url_padding = " ".repeat(max_url.saturating_sub(url_w));
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", tags.join(", "))
            };
            format!(" {}  {}{}{}", padded_name, linked_url, url_padding, tag_str)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_tag_list(config: &Config) -> String {
    let tags = config.all_tags();
    if tags.is_empty() {
        return " No tags found.".to_string();
    }
    tags.iter()
        .map(|t| format!(" {}", t))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub fn pad_or_truncate(s: &str, width: usize) -> String {
    let current_width = display_width(s);
    if current_width > width {
        let ellipsis_width = UnicodeWidthChar::width('…').unwrap_or(1);
        let mut result = String::new();
        let mut w = 0;
        for c in s.chars() {
            let cw = UnicodeWidthChar::width(c).unwrap_or(0);
            if w + cw + ellipsis_width > width {
                break;
            }
            result.push(c);
            w += cw;
        }
        result.push('…');
        w += ellipsis_width;
        for _ in w..width {
            result.push(' ');
        }
        result
    } else {
        let mut result = s.to_string();
        for _ in current_width..width {
            result.push(' ');
        }
        result
    }
}

/// Truncate string to fit within width, adding ellipsis if needed. No padding.
pub fn truncate(s: &str, width: usize) -> String {
    let current_width = display_width(s);
    if current_width <= width {
        return s.to_string();
    }
    let ellipsis_width = UnicodeWidthChar::width('…').unwrap_or(1);
    let mut result = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(0);
        if w + cw + ellipsis_width > width {
            break;
        }
        result.push(c);
        w += cw;
    }
    result.push('…');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc8_hyperlink_format() {
        let result = osc8_hyperlink("https://example.com", "Click here");
        assert!(result.contains("https://example.com"));
        assert!(result.contains("Click here"));
        assert!(result.starts_with("\x1b]8;;"));
        assert!(result.ends_with("\x1b]8;;\x1b\\"));
    }

    #[test]
    fn test_osc8_hyperlink_empty_text() {
        let result = osc8_hyperlink("https://example.com", "");
        assert_eq!(result, "\x1b]8;;https://example.com\x1b\\\x1b]8;;\x1b\\");
    }
}
