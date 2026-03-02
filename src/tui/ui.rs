use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Screen};

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ArticleList => render_article_list(frame, app),
        Screen::ArticleView => render_article_view(frame, app),
    }
}

fn render_article_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .filter_map(|&idx| app.store.get(idx))
        .map(|entry| {
            let date = entry
                .published
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "          ".to_string());

            if entry.read {
                let line = Line::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled("  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(entry.title.clone(), Style::default().fg(Color::DarkGray)),
                ]);
                ListItem::new(line)
            } else {
                let line = Line::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::raw(&entry.title),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let title = if app.is_showing_read() {
        " Feed Reader (all) "
    } else {
        " Feed Reader (unread) "
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    app.list_state.select(Some(app.selected));
    app.layout_areas.main_area = chunks[0];
    app.layout_areas.status_bar = chunks[1];
    frame.render_stateful_widget(list, chunks[0], &mut app.list_state);

    let status = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else if app.loading {
        " Loading...".to_string()
    } else {
        " Enter: open  m: read/unread  o: browser  a: +read/-read  r: refresh  Esc: quit"
            .to_string()
    };
    let status_line = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status_line, chunks[1]);
}

fn render_article_view(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    app.layout_areas.main_area = chunks[0];
    app.layout_areas.status_bar = chunks[1];

    let title = app.article_title.as_deref().unwrap_or("Article");
    let url = app.article_url.as_deref().unwrap_or("");
    let content = app.article_content.as_deref().unwrap_or("");

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        url,
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(""));
    for line in content.lines() {
        lines.push(Line::from(line));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(app.scroll_offset).unwrap_or(u16::MAX), 0));

    frame.render_widget(paragraph, chunks[0]);

    let read_label = if app.current_article().is_some_and(|a| a.read) {
        "unread"
    } else {
        "read"
    };
    let status = format!(" m: {}  o: browser  Esc: back", read_label);
    let status_line = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status_line, chunks[1]);
}
