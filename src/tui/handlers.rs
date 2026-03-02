use crossterm::event::{KeyEventKind, MouseButton, MouseEventKind};
use tokio::sync::mpsc;

use super::action::Action;
use super::app::{App, Screen};
use super::keybindings;
use super::BgMessage;
use crate::config::ExtractorMethod;

/// Open the current article, using content cache if available, otherwise spawning a background fetch.
/// For Readability mode with RSS content available, shows the RSS content as an immediate preview
/// while fetching the full Readability-extracted content in the background.
pub(super) fn open_current_article(
    app: &mut App,
    terminal_width: usize,
    tx: &mpsc::UnboundedSender<BgMessage>,
) {
    let article = match app.current_article() {
        Some(a) => a,
        None => return,
    };
    let url = article.url.clone();
    let title = article.title.clone();
    let rss_content = article.rss_content.clone();
    let method = article.extractor.clone();

    if let Some(cached) = app.get_cached_content(&url).cloned() {
        app.show_article(title, url, cached);
    } else {
        // Show RSS content as immediate preview for Readability mode
        let rss_preview = crate::article::extract_rss_html(rss_content.as_deref())
            .filter(|_| method == ExtractorMethod::Readability)
            .map(|html| {
                let content_width = terminal_width.saturating_sub(4);
                crate::article::html_to_text(&html, content_width)
            });

        if let Some(preview) = rss_preview {
            app.show_article(title.clone(), url.clone(), preview);
        } else {
            app.loading = true;
        }
        // Fetch full content in background
        spawn_content_fetch(
            app,
            url,
            title,
            method,
            rss_content,
            terminal_width,
            tx.clone(),
        );
    }
}

pub(super) fn spawn_content_fetch(
    app: &App,
    url: String,
    title: String,
    method: ExtractorMethod,
    rss_content: Option<String>,
    width: usize,
    tx: mpsc::UnboundedSender<BgMessage>,
) {
    let client = app.store.client().clone();
    tokio::spawn(async move {
        let content = crate::article::extract_html(&client, &url, &method, rss_content.as_deref())
            .await
            .map(|html| crate::article::html_to_text(&html, width.saturating_sub(4)))
            .unwrap_or_else(|e| format!("Error: {}", e));

        let _ = tx.send(BgMessage::ArticleContent {
            url,
            title,
            content,
        });
    });
}

/// Handle a key event. Returns true if the app should quit.
pub(super) fn handle_key_event(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    terminal_width: usize,
    terminal_height: usize,
    tx: &mpsc::UnboundedSender<BgMessage>,
) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }

    let action = keybindings::resolve_action(&app.screen, key);

    match action {
        Action::Quit => {
            app.should_quit = true;
            return true;
        }
        Action::MoveDown => app.move_down(),
        Action::MoveUp => app.move_up(),
        Action::OpenArticle => {
            if app.current_article().is_none() {
                return false;
            }
            if app.store.config().content.auto_mark_read {
                app.mark_current_read();
            }
            open_current_article(app, terminal_width, tx);
        }
        Action::BackToList => {
            app.close_article();
        }
        Action::Refresh => {
            app.loading = true;
            app.status_message = Some(" Refreshing...".to_string());
            app.reset_refresh_timer();
            super::spawn_background_fetch(app, tx.clone());
        }
        Action::ToggleRead => {
            app.toggle_current_read();
        }
        Action::ToggleReadFilter => {
            app.toggle_read_filter();
        }
        Action::OpenInBrowser => {
            if let Some(url) = app.selected_url() {
                let _ = open::that(url);
            }
        }
        Action::NextArticle => {
            if app.screen == Screen::ArticleView && app.has_next_article() {
                app.select_next_article();
                if app.store.config().content.auto_mark_read {
                    app.mark_current_read();
                }
                open_current_article(app, terminal_width, tx);
            }
        }
        Action::PrevArticle => {
            if app.screen == Screen::ArticleView && app.has_prev_article() {
                app.select_prev_article();
                if app.store.config().content.auto_mark_read {
                    app.mark_current_read();
                }
                open_current_article(app, terminal_width, tx);
            }
        }
        Action::ScrollDown => {
            app.scroll_down(terminal_height.saturating_sub(3));
        }
        Action::ScrollUp => app.scroll_up(),
        Action::PageDown => {
            app.scroll_page_down(
                terminal_height.saturating_sub(4),
                terminal_height.saturating_sub(3),
            );
        }
        Action::PageUp => {
            app.scroll_page_up(terminal_height.saturating_sub(4));
        }
        Action::None => {}
    }

    false
}

pub(super) fn handle_mouse_event(
    app: &mut App,
    mouse: &crossterm::event::MouseEvent,
    terminal_width: usize,
    terminal_height: usize,
    last_click: &mut Option<(std::time::Instant, usize)>,
    tx: &mpsc::UnboundedSender<BgMessage>,
) {
    match mouse.kind {
        MouseEventKind::ScrollUp => match app.screen {
            Screen::ArticleList => app.move_up(),
            Screen::ArticleView => app.scroll_up(),
        },
        MouseEventKind::ScrollDown => match app.screen {
            Screen::ArticleList => app.move_down(),
            Screen::ArticleView => {
                app.scroll_down(terminal_height.saturating_sub(3));
            }
        },
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;
            let main = app.layout_areas.main_area;

            if app.screen == Screen::ArticleList
                && row > main.y
                && row < main.y + main.height - 1
                && col >= main.x
                && col < main.x + main.width
            {
                let list_offset = app.list_state.offset();
                let clicked_row = (row - main.y - 1) as usize;
                let clicked_index = list_offset + clicked_row;
                if clicked_index < app.filtered_len() {
                    app.select(clicked_index);

                    // Double-click detection
                    let now = std::time::Instant::now();
                    let is_double = last_click
                        .map(|(t, idx)| {
                            now.duration_since(t).as_millis() < 300 && idx == clicked_index
                        })
                        .unwrap_or(false);

                    if is_double {
                        *last_click = None;
                        if app.current_article().is_some() {
                            if app.store.config().content.auto_mark_read {
                                app.mark_current_read();
                            }
                            open_current_article(app, terminal_width, tx);
                        }
                    } else {
                        *last_click = Some((now, clicked_index));
                    }
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if app.screen == Screen::ArticleView {
                app.close_article();
            }
        }
        _ => {}
    }
}

pub(super) fn poll_bg_messages(app: &mut App, rx: &mut mpsc::UnboundedReceiver<BgMessage>) {
    while let Ok(msg) = rx.try_recv() {
        match msg {
            BgMessage::FetchComplete(articles) => {
                app.reset_refresh_timer();
                app.store.set_articles(articles);
                app.rebuild_filtered_list();
                app.loading = false;
            }
            BgMessage::ArticleContent {
                url,
                title,
                content,
            } => {
                app.cache_content(url.clone(), content.clone());
                if app.screen == Screen::ArticleView && app.article_url.as_deref() == Some(&url) {
                    // Already viewing this article (RSS preview or loading) — update in place
                    app.article_content = Some(content);
                    app.loading = false;
                } else if app.screen != Screen::ArticleView {
                    // User is in article list waiting for content
                    app.show_article(title, url, content);
                }
                // If viewing a different article: content is cached for later
            }
        }
    }
}
