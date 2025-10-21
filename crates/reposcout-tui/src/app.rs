// TUI application state and event handling
use reposcout_core::models::Repository;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,      // Navigating results
    Searching,   // Typing in search box
    Filtering,   // Navigating filters
    EditingFilter, // Actively typing in a filter field
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    Stats,   // Show repository statistics
    Readme,  // Show README content
}

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub min_stars: Option<u32>,
    pub max_stars: Option<u32>,
    pub pushed: Option<String>,
    pub sort_by: String,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            language: None,
            min_stars: None,
            max_stars: None,
            pushed: None,
            sort_by: "stars".to_string(),
        }
    }
}

impl SearchFilters {
    pub fn build_query(&self, base_query: &str) -> String {
        let mut parts = vec![base_query.to_string()];

        if let Some(lang) = &self.language {
            if !lang.is_empty() {
                parts.push(format!("language:{}", lang));
            }
        }

        match (self.min_stars, self.max_stars) {
            (Some(min), Some(max)) => parts.push(format!("stars:{}..{}", min, max)),
            (Some(min), None) => parts.push(format!("stars:>={}", min)),
            (None, Some(max)) => parts.push(format!("stars:<={}", max)),
            (None, None) => {}
        }

        if let Some(pushed) = &self.pushed {
            if !pushed.is_empty() {
                parts.push(format!("pushed:{}", pushed));
            }
        }

        parts.join(" ")
    }

    pub fn sort_results(&self, results: &mut [Repository]) {
        match self.sort_by.as_str() {
            "stars" => results.sort_by(|a, b| b.stars.cmp(&a.stars)),
            "forks" => results.sort_by(|a, b| b.forks.cmp(&a.forks)),
            "updated" => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            _ => {}
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub search_input: String,
    pub results: Vec<Repository>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error_message: Option<String>,
    pub filters: SearchFilters,
    pub show_filters: bool,
    pub filter_cursor: usize,
    pub filter_edit_buffer: String,
    pub list_state: ListState,
    pub preview_mode: PreviewMode,
    pub readme_content: Option<String>,
    pub readme_loading: bool,
    // Cache README content per repository to avoid re-fetching
    pub readme_cache: std::collections::HashMap<String, String>,
    // Scroll position for README view
    pub readme_scroll: u16,
    // Track bookmarked repositories (platform + full_name)
    pub bookmarked: std::collections::HashSet<String>,
    // Show bookmarks only
    pub show_bookmarks_only: bool,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            should_quit: false,
            input_mode: InputMode::Searching,
            search_input: String::new(),
            results: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            loading: false,
            error_message: None,
            filters: SearchFilters::default(),
            show_filters: false,
            filter_cursor: 0,
            filter_edit_buffer: String::new(),
            list_state,
            preview_mode: PreviewMode::Stats,
            readme_content: None,
            readme_loading: false,
            readme_cache: std::collections::HashMap::new(),
            readme_scroll: 0,
            bookmarked: std::collections::HashSet::new(),
            show_bookmarks_only: false,
        }
    }

    /// Get bookmark key for a repository
    pub fn bookmark_key(platform: &str, full_name: &str) -> String {
        format!("{}:{}", platform, full_name)
    }

    /// Check if current repository is bookmarked
    pub fn is_current_bookmarked(&self) -> bool {
        if let Some(repo) = self.selected_repository() {
            let key = Self::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            self.bookmarked.contains(&key)
        } else {
            false
        }
    }

    /// Add/remove current repository from bookmarks
    pub fn toggle_current_bookmark(&mut self) {
        if let Some(repo) = self.selected_repository() {
            let key = Self::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            if self.bookmarked.contains(&key) {
                self.bookmarked.remove(&key);
            } else {
                self.bookmarked.insert(key);
            }
        }
    }

    /// Toggle showing bookmarks only
    pub fn toggle_bookmarks_view(&mut self) {
        self.show_bookmarks_only = !self.show_bookmarks_only;
    }

    pub fn toggle_preview_mode(&mut self) {
        self.preview_mode = match self.preview_mode {
            PreviewMode::Stats => PreviewMode::Readme,
            PreviewMode::Readme => PreviewMode::Stats,
        };
    }

    pub fn set_readme(&mut self, content: String) {
        self.readme_content = Some(content);
        self.readme_loading = false;
    }

    pub fn clear_readme(&mut self) {
        self.readme_content = None;
        self.readme_loading = false;
    }

    /// Check if README is cached for the currently selected repository
    pub fn get_cached_readme(&self) -> Option<&String> {
        if let Some(repo) = self.selected_repository() {
            self.readme_cache.get(&repo.full_name)
        } else {
            None
        }
    }

    /// Cache README content for a repository
    pub fn cache_readme(&mut self, repo_name: String, content: String) {
        self.readme_cache.insert(repo_name, content);
    }

    /// Start README loading for current repository
    pub fn start_readme_loading(&mut self) {
        self.readme_loading = true;
        self.readme_content = None;
    }

    /// Set README from cache or fetched content
    pub fn load_readme_for_current(&mut self) {
        if let Some(repo) = self.selected_repository() {
            if let Some(cached) = self.readme_cache.get(&repo.full_name) {
                self.readme_content = Some(cached.clone());
                self.readme_loading = false;
            } else {
                // Mark as loading - will be fetched async
                self.start_readme_loading();
            }
        }
    }

    /// Scroll README down
    pub fn scroll_readme_down(&mut self) {
        self.readme_scroll = self.readme_scroll.saturating_add(1);
    }

    /// Scroll README up
    pub fn scroll_readme_up(&mut self) {
        self.readme_scroll = self.readme_scroll.saturating_sub(1);
    }

    /// Reset README scroll position
    pub fn reset_readme_scroll(&mut self) {
        self.readme_scroll = 0;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Searching;
    }

    pub fn enter_normal_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn enter_filter_mode(&mut self) {
        self.input_mode = InputMode::Filtering;
    }

    pub fn enter_editing_filter_mode(&mut self) {
        self.input_mode = InputMode::EditingFilter;
        // Load current filter value into edit buffer
        self.filter_edit_buffer = match self.filter_cursor {
            0 => self.filters.language.clone().unwrap_or_default(),
            1 => self.filters.min_stars.map(|s| s.to_string()).unwrap_or_default(),
            2 => self.filters.max_stars.map(|s| s.to_string()).unwrap_or_default(),
            3 => self.filters.pushed.clone().unwrap_or_default(),
            4 => self.filters.sort_by.clone(),
            _ => String::new(),
        };
    }

    pub fn save_filter_edit(&mut self) {
        // Save the edit buffer to the actual filter
        match self.filter_cursor {
            0 => {
                self.filters.language = if self.filter_edit_buffer.is_empty() {
                    None
                } else {
                    Some(self.filter_edit_buffer.clone())
                };
            }
            1 => {
                self.filters.min_stars = self.filter_edit_buffer.parse().ok();
            }
            2 => {
                self.filters.max_stars = self.filter_edit_buffer.parse().ok();
            }
            3 => {
                self.filters.pushed = if self.filter_edit_buffer.is_empty() {
                    None
                } else {
                    Some(self.filter_edit_buffer.clone())
                };
            }
            4 => {
                if !self.filter_edit_buffer.is_empty() {
                    self.filters.sort_by = self.filter_edit_buffer.clone();
                }
            }
            _ => {}
        }
        self.filter_edit_buffer.clear();
        self.input_mode = InputMode::Filtering;
    }

    pub fn cancel_filter_edit(&mut self) {
        self.filter_edit_buffer.clear();
        self.input_mode = InputMode::Filtering;
    }

    pub fn toggle_filters(&mut self) {
        self.show_filters = !self.show_filters;
    }

    pub fn next_filter(&mut self) {
        self.filter_cursor = (self.filter_cursor + 1).min(4); // 5 filter fields
    }

    pub fn previous_filter(&mut self) {
        if self.filter_cursor > 0 {
            self.filter_cursor -= 1;
        }
    }

    pub fn cycle_sort(&mut self) {
        self.filters.sort_by = match self.filters.sort_by.as_str() {
            "stars" => "forks".to_string(),
            "forks" => "updated".to_string(),
            _ => "stars".to_string(),
        };
    }

    pub fn clear_current_filter(&mut self) {
        match self.filter_cursor {
            0 => self.filters.language = None,
            1 => self.filters.min_stars = None,
            2 => self.filters.max_stars = None,
            3 => self.filters.pushed = None,
            4 => self.filters.sort_by = "stars".to_string(),
            _ => {}
        }
    }

    pub fn next_result(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.results.len() - 1);
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn previous_result(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn selected_repository(&self) -> Option<&Repository> {
        self.results.get(self.selected_index)
    }

    pub fn set_results(&mut self, mut results: Vec<Repository>) {
        // Apply sorting based on filters
        self.filters.sort_results(&mut results);
        self.results = results;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_state.select(Some(0));
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn get_search_query(&self) -> String {
        self.filters.build_query(&self.search_input)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
