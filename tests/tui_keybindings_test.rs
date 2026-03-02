use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use feed::tui::action::Action;
use feed::tui::app::Screen;
use feed::tui::keybindings::resolve_action;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn test_article_list_quit() {
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('q'))),
        Action::Quit
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Esc)),
        Action::Quit
    );
}

#[test]
fn test_article_list_navigation() {
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('j'))),
        Action::MoveDown
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Down)),
        Action::MoveDown
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('k'))),
        Action::MoveUp
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Up)),
        Action::MoveUp
    );
}

#[test]
fn test_article_list_actions() {
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Enter)),
        Action::OpenArticle
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char(' '))),
        Action::OpenArticle
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('o'))),
        Action::OpenInBrowser
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('m'))),
        Action::ToggleRead
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('a'))),
        Action::ToggleReadFilter
    );
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('r'))),
        Action::Refresh
    );
}

#[test]
fn test_article_view_navigation() {
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('q'))),
        Action::BackToList
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Esc)),
        Action::BackToList
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('j'))),
        Action::ScrollDown
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('k'))),
        Action::ScrollUp
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('h'))),
        Action::PrevArticle
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('l'))),
        Action::NextArticle
    );
}

#[test]
fn test_article_view_actions() {
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char(' '))),
        Action::PageDown
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('o'))),
        Action::OpenInBrowser
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('m'))),
        Action::ToggleRead
    );
}

#[test]
fn test_unknown_key_returns_none() {
    assert_eq!(
        resolve_action(&Screen::ArticleList, &key(KeyCode::Char('z'))),
        Action::None
    );
    assert_eq!(
        resolve_action(&Screen::ArticleView, &key(KeyCode::Char('z'))),
        Action::None
    );
}
