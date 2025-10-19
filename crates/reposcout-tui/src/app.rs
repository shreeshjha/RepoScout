// TUI application state and event handling
use reposcout_core::models::Repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,      // Navigating results
    Searching,   // Typing in search box
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

    pub fn set_results(&mut self, results: Vec<Repository>) {
        self.results = results;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
