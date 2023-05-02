use lazy_static::lazy_static;
use tui::style::{Color, Modifier, Style};

lazy_static! {
    pub static ref BOLD: Style = Style::default().add_modifier(Modifier::BOLD);
    pub static ref ITALIC: Style = Style::default().add_modifier(Modifier::ITALIC);
    pub static ref UNDERLINE: Style = Style::default().add_modifier(Modifier::UNDERLINED);
    pub static ref RED: Style = Style::default().fg(Color::Red);
    pub static ref WHITE: Style = Style::default().fg(Color::White);
    pub static ref BLACK: Style = Style::default().fg(Color::Black);
    pub static ref GREEN: Style = Style::default().fg(Color::Green);
    pub static ref BLUE: Style = Style::default().fg(Color::Blue);
    pub static ref YELLOW: Style = Style::default().fg(Color::Yellow);
    pub static ref WHITE_BG: Style = Style::default().bg(Color::White);
    pub static ref ROW_BG: Style = Style::default().bg(Color::Rgb(165, 165, 165));
    // let row_style = Style::default().add_modifier(Modifier::DIM);
}
