use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::article::Article;
use crate::article_store::{ArticleStore, FilterParams};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    ArticleList,
    ArticleView,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LayoutAreas {
    pub(crate) main_area: Rect,
    pub(crate) status_bar: Rect,
}

pub struct App {
    pub(crate) screen: Screen,
    pub store: ArticleStore,
    pub(crate) filter_params: FilterParams,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub(crate) scroll_offset: usize,
    pub article_content: Option<String>,
    pub(crate) article_title: Option<String>,
    pub article_url: Option<String>,
    pub loading: bool,
    pub(crate) should_quit: bool,
    pub(crate) status_message: Option<String>,
    pub(crate) layout_areas: LayoutAreas,
    pub(crate) list_state: ListState,
    pub(crate) last_refresh: Instant,
    pub auto_refresh_interval: Option<Duration>,
    pub(crate) content_cache: HashMap<String, String>,
}

impl App {
    pub fn new(store: ArticleStore, filter_params: FilterParams) -> Self {
        let filtered_indices = store.query(&filter_params);
        let auto_refresh_secs = store.config().tui.auto_refresh_interval;
        let auto_refresh_interval = if auto_refresh_secs > 0 {
            Some(Duration::from_secs(auto_refresh_secs))
        } else {
            None
        };
        Self {
            screen: Screen::ArticleList,
            store,
            filter_params,
            filtered_indices,
            selected: 0,
            scroll_offset: 0,
            article_content: None,
            article_title: None,
            article_url: None,
            loading: false,
            should_quit: false,
            status_message: None,
            layout_areas: LayoutAreas::default(),
            list_state: ListState::default(),
            last_refresh: Instant::now(),
            auto_refresh_interval,
            content_cache: HashMap::new(),
        }
    }

    pub fn current_article(&self) -> Option<&Article> {
        let &idx = self.filtered_indices.get(self.selected)?;
        self.store.get(idx)
    }

    pub fn filtered_len(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub(crate) fn select(&mut self, index: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = index.min(self.filtered_indices.len() - 1);
    }

    fn article_line_count(&self) -> usize {
        let content_lines = self
            .article_content
            .as_deref()
            .map(|c| c.lines().count())
            .unwrap_or(0);
        3 + content_lines
    }

    fn clamp_scroll(&mut self, visible_height: usize) {
        let max = self.article_line_count().saturating_sub(visible_height);
        self.scroll_offset = self.scroll_offset.min(max);
    }

    pub(crate) fn scroll_down(&mut self, visible_height: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.clamp_scroll(visible_height);
    }

    pub(crate) fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub(crate) fn scroll_page_down(&mut self, page_height: usize, visible_height: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(page_height);
        self.clamp_scroll(visible_height);
    }

    pub(crate) fn scroll_page_up(&mut self, page_height: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_height);
    }

    pub(crate) fn selected_url(&self) -> Option<&str> {
        self.current_article().map(|a| a.url.as_str())
    }

    pub fn show_article(&mut self, title: String, url: String, content: String) {
        self.article_title = Some(title);
        self.article_url = Some(url);
        self.article_content = Some(content);
        self.scroll_offset = 0;
        self.screen = Screen::ArticleView;
        self.loading = false;
    }

    pub(crate) fn has_prev_article(&self) -> bool {
        self.selected > 0
    }

    pub(crate) fn has_next_article(&self) -> bool {
        !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1
    }

    pub(crate) fn select_prev_article(&mut self) {
        if self.has_prev_article() {
            self.selected -= 1;
            self.scroll_offset = 0;
            self.loading = true;
        }
    }

    pub(crate) fn select_next_article(&mut self) {
        if self.has_next_article() {
            self.selected += 1;
            self.scroll_offset = 0;
            self.loading = true;
        }
    }

    pub fn close_article(&mut self) {
        self.screen = Screen::ArticleList;
        self.article_content = None;
        self.article_title = None;
        self.article_url = None;
        self.scroll_offset = 0;
    }

    pub fn toggle_read_filter(&mut self) {
        let selected_url = self.selected_url().map(|s| s.to_string());
        self.filter_params.show_read = !self.filter_params.show_read;
        self.filtered_indices = self.store.query(&self.filter_params);
        if let Some(url) = selected_url {
            if let Some(pos) = self
                .filtered_indices
                .iter()
                .position(|&i| self.store.get(i).is_some_and(|a| a.url == url))
            {
                self.selected = pos;
                return;
            }
        }
        self.selected = self
            .selected
            .min(self.filtered_indices.len().saturating_sub(1));
    }

    pub(crate) fn is_showing_read(&self) -> bool {
        self.filter_params.show_read
    }

    fn current_store_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
    }

    pub fn mark_current_read(&mut self) {
        if let Some(idx) = self.current_store_index() {
            self.store.mark_read(idx);
        }
    }

    pub fn toggle_current_read(&mut self) {
        if let Some(idx) = self.current_store_index() {
            self.store.toggle_read(idx);
        }
    }

    pub fn should_auto_refresh(&self) -> bool {
        matches!(self.auto_refresh_interval, Some(interval) if !self.loading && self.last_refresh.elapsed() >= interval)
    }

    pub fn reset_refresh_timer(&mut self) {
        self.last_refresh = Instant::now();
    }

    pub fn rebuild_filtered_list(&mut self) {
        let selected_url = self.selected_url().map(|s| s.to_string());
        self.filtered_indices = self.store.query(&self.filter_params);

        if let Some(url) = selected_url {
            if let Some(pos) = self
                .filtered_indices
                .iter()
                .position(|&i| self.store.get(i).is_some_and(|a| a.url == url))
            {
                self.selected = pos;
                self.status_message = None;
                return;
            }
        }

        self.selected = self
            .selected
            .min(self.filtered_indices.len().saturating_sub(1));
        self.status_message = None;
    }

    pub(crate) fn get_cached_content(&self, url: &str) -> Option<&String> {
        self.content_cache.get(url)
    }

    pub(crate) fn cache_content(&mut self, url: String, content: String) {
        self.content_cache.insert(url, content);
    }
}
