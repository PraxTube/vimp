use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, LineGauge, Paragraph},
    Frame,
};

use crate::ui::terminal::App;
use std::collections::VecDeque;

pub struct Debug {
    pub messages: VecDeque<String>,
}

impl Debug {
    pub fn new() -> Debug {
        Debug {
            messages: VecDeque::new(),
        }
    }

    pub fn push_message(&mut self, message: String) {
        self.messages.push_back(message);
        if self.messages.len() > 10 {
            self.messages.pop_front();
        }
    }
}

pub fn render_active_song_info<B: Backend>(f: &mut Frame<B>, app: &mut App, chunk: Rect) {
    let block = Block::default().title("Debug").borders(Borders::ALL);
    f.render_widget(block, chunk);

    let chunk = Layout::default()
        .margin(1)
        .constraints([Constraint::Percentage(100)])
        .split(chunk);
    let mut debug_string = String::new();

    for msg in &app.debugger.messages {
        debug_string += &msg;
        debug_string += "\n";
    }
    let paragraph_info = Paragraph::new(debug_string);
    f.render_widget(paragraph_info, chunk[0]);
}