pub mod action;
pub mod app;
mod handlers;
pub mod keybindings;
pub(crate) mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use crate::article_store::{ArticleStore, FilterParams};
use app::App;

enum BgMessage {
    FetchComplete(Vec<crate::article::Article>),
    ArticleContent {
        url: String,
        title: String,
        content: String,
    },
}

pub(crate) async fn run(store: ArticleStore, filter_params: FilterParams) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;

    let supports_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    if supports_enhancement {
        stdout.queue(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
        ))?;
    }

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = App::new(store, filter_params);

    let (tx, rx) = mpsc::unbounded_channel();

    // Background refresh on startup
    spawn_background_fetch(&app, tx.clone());
    app.status_message = Some(" Updating...".to_string());
    app.reset_refresh_timer();

    let result = event_loop(&mut terminal, &mut app, tx, rx).await;

    if supports_enhancement {
        io::stdout().execute(PopKeyboardEnhancementFlags)?;
    }
    io::stdout().execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn spawn_background_fetch(app: &App, tx: mpsc::UnboundedSender<BgMessage>) {
    let client = app.store.client().clone();
    let feeds = app.store.feeds().to_vec();
    let config = app.store.config().clone();
    let data_dir = app.store.data_dir().to_path_buf();

    tokio::spawn(async move {
        let mut temp_store = ArticleStore::with_client(feeds, config, data_dir, client);
        temp_store.fetch(false).await;
        let articles = temp_store.take_articles();
        let _ = tx.send(BgMessage::FetchComplete(articles));
    });
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tx: mpsc::UnboundedSender<BgMessage>,
    mut rx: mpsc::UnboundedReceiver<BgMessage>,
) -> Result<()> {
    let mut last_click: Option<(std::time::Instant, usize)> = None;

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        let has_event = event::poll(Duration::from_millis(50))?;

        // Check background messages
        handlers::poll_bg_messages(app, &mut rx);

        // Auto-refresh check
        if app.should_auto_refresh() {
            app.loading = true;
            app.status_message = Some(" Auto-refreshing...".to_string());
            app.reset_refresh_timer();
            spawn_background_fetch(app, tx.clone());
        }

        if !has_event {
            continue;
        }

        let event = event::read()?;

        if let Event::Key(key) = &event {
            let size = terminal.size()?;
            let width = size.width as usize;
            let height = size.height as usize;
            if handlers::handle_key_event(app, key, width, height, &tx) {
                break;
            }
        }

        if let Event::Mouse(mouse) = &event {
            let size = terminal.size()?;
            let width = size.width as usize;
            let height = size.height as usize;
            handlers::handle_mouse_event(app, mouse, width, height, &mut last_click, &tx);
        }
    }
    Ok(())
}
