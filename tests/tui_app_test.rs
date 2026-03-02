use anyhow::Context;
use chrono::Utc;
use feed::article::Article;
use feed::article_store::{ArticleStore, FilterParams};
use feed::config::Config;
use feed::tui::app::App;
use tempfile::tempdir;

fn make_article(title: &str, read: bool, days_ago: i64) -> Article {
    Article {
        title: title.to_string(),
        url: format!("https://example.com/{}", title.replace(' ', "-")),
        published: Some(Utc::now() - chrono::Duration::days(days_ago)),
        feed_url: "https://example.com/feed".to_string(),
        feed_name: "Test Blog".to_string(),
        extractor: feed::config::ExtractorMethod::default(),
        read,
        rss_content: None,
    }
}

fn make_app(articles: Vec<Article>, filter_params: FilterParams) -> anyhow::Result<App> {
    let dir = tempdir()?;
    let mut store = ArticleStore::new(vec![], Config::default(), dir.keep());
    store.set_articles(articles);
    Ok(App::new(store, filter_params))
}

#[test]
fn test_toggle_read_filter_preserves_selection() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![
            make_article("unread1", false, 1),
            make_article("read1", true, 2),
            make_article("unread2", false, 3),
        ],
        FilterParams::default(),
    )?;
    // Default: unread only -> 2 articles (unread1, unread2)
    assert_eq!(app.filtered_len(), 2);

    // Select second article (unread2)
    app.selected = 1;

    // Toggle to show all
    app.toggle_read_filter();
    assert_eq!(app.filtered_len(), 3);
    // unread2 should still be selected
    assert_eq!(
        app.current_article()
            .context("expected current article")?
            .title,
        "unread2"
    );
    Ok(())
}

#[test]
fn test_toggle_read_filter_clamps_when_selected_removed() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![
            make_article("read1", true, 1),
            make_article("read2", true, 2),
            make_article("unread1", false, 3),
        ],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;
    // All 3 visible, select read1
    assert_eq!(app.filtered_len(), 3);
    app.selected = 0;

    // Toggle to unread only -> read1 disappears
    app.toggle_read_filter();
    assert_eq!(app.filtered_len(), 1);
    // Selection should clamp to valid range
    assert!(app.selected < app.filtered_len());
    Ok(())
}

#[test]
fn test_toggle_read_filter_toggles_filter_params() -> anyhow::Result<()> {
    let mut app = make_app(vec![], FilterParams::default())?;
    assert!(!app.is_showing_read());
    app.toggle_read_filter();
    assert!(app.is_showing_read());
    app.toggle_read_filter();
    assert!(!app.is_showing_read());
    Ok(())
}

#[tokio::test]
async fn test_mark_current_read_updates_articles() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![make_article("a", false, 1), make_article("b", false, 2)],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;
    app.selected = 0;
    app.mark_current_read();

    assert!(
        app.store
            .get(app.filtered_indices[0])
            .context("expected article at index 0")?
            .read
    );
    assert!(
        !app.store
            .get(app.filtered_indices[1])
            .context("expected article at index 1")?
            .read
    );
    Ok(())
}

#[tokio::test]
async fn test_toggle_current_read() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![make_article("a", false, 1)],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;
    app.toggle_current_read();
    assert!(
        app.current_article()
            .context("expected current article after first toggle")?
            .read
    );

    app.toggle_current_read();
    assert!(
        !app.current_article()
            .context("expected current article after second toggle")?
            .read
    );
    Ok(())
}

#[test]
fn test_move_down_up() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![
            make_article("a", false, 1),
            make_article("b", false, 2),
            make_article("c", false, 3),
        ],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;
    assert_eq!(app.selected, 0);
    app.move_down();
    assert_eq!(app.selected, 1);
    app.move_down();
    assert_eq!(app.selected, 2);
    app.move_down(); // at end, should not go further
    assert_eq!(app.selected, 2);
    app.move_up();
    assert_eq!(app.selected, 1);
    app.move_up();
    assert_eq!(app.selected, 0);
    app.move_up(); // at start, should not go further
    assert_eq!(app.selected, 0);
    Ok(())
}

#[test]
fn test_select() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![
            make_article("a", false, 1),
            make_article("b", false, 2),
            make_article("c", false, 3),
        ],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;
    assert_eq!(app.selected, 0);
    app.select(2);
    assert_eq!(app.selected, 2);
    app.select(1);
    assert_eq!(app.selected, 1);
    // Out of range should clamp
    app.select(100);
    assert_eq!(app.selected, 2);
    // Empty list
    let mut empty_app = make_app(vec![], FilterParams::default())?;
    empty_app.select(0);
    assert_eq!(empty_app.selected, 0);
    Ok(())
}

#[test]
fn test_set_articles_during_article_view_preserves_content() -> anyhow::Result<()> {
    let mut app = make_app(
        vec![
            make_article("a", false, 1),
            make_article("b", false, 2),
        ],
        FilterParams {
            show_read: true,
            ..Default::default()
        },
    )?;

    // Open article "a"
    app.selected = 0;
    app.show_article(
        "a".to_string(),
        "https://example.com/a".to_string(),
        "Article content here".to_string(),
    );

    // Simulate FetchComplete arriving while viewing article:
    // replace articles (as auto-refresh would)
    app.store.set_articles(vec![
        make_article("a", false, 1),
        make_article("b", false, 2),
        make_article("c", false, 0), // new article from refresh
    ]);
    app.rebuild_filtered_list();

    // Article content should be unaffected
    assert_eq!(app.article_content.as_deref(), Some("Article content here"));
    assert_eq!(app.article_url.as_deref(), Some("https://example.com/a"));

    // Return to list
    app.close_article();
    assert_eq!(app.filtered_len(), 3);
    // Selection should be preserved on article "a"
    assert_eq!(
        app.current_article()
            .context("expected current article")?
            .title,
        "a"
    );
    Ok(())
}

#[test]
fn test_auto_refresh_disabled_by_default() -> anyhow::Result<()> {
    let app = make_app(vec![], FilterParams::default())?;
    assert!(app.auto_refresh_interval.is_none());
    assert!(!app.should_auto_refresh());
    Ok(())
}

#[test]
fn test_auto_refresh_not_triggered_while_loading() -> anyhow::Result<()> {
    let mut app = make_app(vec![], FilterParams::default())?;
    app.auto_refresh_interval = Some(std::time::Duration::from_secs(0));
    app.loading = true;
    assert!(!app.should_auto_refresh());
    Ok(())
}

#[test]
fn test_auto_refresh_triggered_after_interval() -> anyhow::Result<()> {
    let mut app = make_app(vec![], FilterParams::default())?;
    app.auto_refresh_interval = Some(std::time::Duration::from_secs(0));
    // Interval is 0 so it should fire immediately
    assert!(app.should_auto_refresh());
    Ok(())
}

#[test]
fn test_reset_refresh_timer() -> anyhow::Result<()> {
    let mut app = make_app(vec![], FilterParams::default())?;
    app.auto_refresh_interval = Some(std::time::Duration::from_secs(9999));
    app.reset_refresh_timer();
    // Just reset, so not enough time has elapsed
    assert!(!app.should_auto_refresh());
    Ok(())
}
