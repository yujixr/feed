#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    OpenArticle,
    BackToList,
    Refresh,
    ToggleRead,
    ToggleReadFilter,
    OpenInBrowser,
    NextArticle,
    PrevArticle,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    None,
}
