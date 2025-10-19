// TUI application state and event handling

pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { should_quit: false }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
