use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::app::Screen;

pub fn resolve_action(screen: &Screen, key: &KeyEvent) -> Action {
    // Ctrl-C / Ctrl-D always quits regardless of screen
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
    {
        return Action::Quit;
    }

    match screen {
        Screen::ArticleList => resolve_list_action(key),
        Screen::ArticleView => resolve_view_action(key),
    }
}

fn resolve_list_action(key: &KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter | KeyCode::Char(' ') => Action::OpenArticle,
        KeyCode::Char('o') => Action::OpenInBrowser,
        KeyCode::Char('m') => Action::ToggleRead,
        KeyCode::Char('a') => Action::ToggleReadFilter,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}

fn resolve_view_action(key: &KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::BackToList,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('h') | KeyCode::Left => Action::PrevArticle,
        KeyCode::Char('l') | KeyCode::Right => Action::NextArticle,
        KeyCode::Char(' ') => Action::PageDown,
        KeyCode::Char('o') => Action::OpenInBrowser,
        KeyCode::Char('m') => Action::ToggleRead,
        _ => Action::None,
    }
}
