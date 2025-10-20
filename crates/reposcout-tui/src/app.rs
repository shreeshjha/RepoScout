// TUI application state and event handling
use reposcout_core::models::Repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,      // Navigating results
    Searching,   // Typing in search box
    Filtering,   // Editing filters
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
}

impl App {
    pub fn new() -> Self {
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
        }
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
        }
    }

    pub fn previous_result(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
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
