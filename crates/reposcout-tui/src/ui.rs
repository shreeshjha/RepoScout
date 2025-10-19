// UI rendering logic
// Currently a stub - will hold all the ratatui magic later

use crate::App;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render<B: Backend>(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.area());

    let block = Block::default()
        .title("RepoScout")
        .borders(Borders::ALL);

    let paragraph = Paragraph::new("TUI coming soon!").block(block);
    frame.render_widget(paragraph, chunks[0]);
}
