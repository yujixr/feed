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

// 'q' and Esc quit from the article list screen.
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

// j/Down move down and k/Up move up in the article list.
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

// Enter/Space open an article; o opens in browser; m toggles read; a toggles filter; r refreshes.
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

// q/Esc go back to list; j/k scroll; h/l switch articles.
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

// Space pages down; o opens in browser; m toggles read in the article view.
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

// Unbound keys return Action::None on both screens.
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
