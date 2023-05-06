use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use tui::Frame;

use super::{FromPrimitive, Render, MBackend};
use std::fmt;

#[derive(FromPrimitive, PartialEq, Copy, Clone, Debug, Ord, PartialOrd, Eq)]
pub enum Column {
    PID = 0,
    PPID = 1,
    User = 2,
    Priority = 3,
    Nice = 4,
    Status = 5,
    TTY = 6,
    CPUPercentage = 7,
    MemoryPercentage = 8,
    Memory = 9,
    VirtualMemory = 10,
    CPUTime = 11,
    StartTime = 12,
    CMD = 13,
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Column::PID => " PID",
            Column::PPID => " PPID",   
            Column::User => " User",
            Column::Priority => " Priority",
            Column::Nice => " Nice",
            Column::Status => " Status",
            Column::TTY => " TTY",
            Column::CPUPercentage => " CPU Percentage",
            Column::MemoryPercentage => " Memory Percentage",
            Column::Memory => " Memory",
            Column::VirtualMemory => " Virtual Memory",
            Column::CPUTime => " CPU Time",
            Column::StartTime => " Start Time",
            Column::CMD => " CMD / Command Name",
        };
        write!(f, "{}", name)
    }
}

pub struct ColumnMGRList<'a> {
    pub items: Vec<(Column, ListItem<'a>)>,
    pub state: ListState,
}

impl<'a> ColumnMGRList<'a> {
    pub fn with_cols(cols: Vec<Column>) -> ColumnMGRList<'a> {
        let mut state = ListState::default();
        let items: Vec<(Column, ListItem)> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
            .iter()
            .map(|i| {
                let column: Column = FromPrimitive::from_u32(*i as u32)
                    .expect("Index not in range for Column enum");
                let c: String = format!("{}", column);
                // default is first 6
                if cols.contains(&column) {
                    (
                        column,
                        Span::styled(
                            format!("*{}", c),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    )
                } else {
                    (column, Span::styled(format!(" {}", c), Style::default()))
                }      
            })
            .map(|(c, span)| (c, ListItem::new(span)))
            .collect();
        state.select(Some(0));
        ColumnMGRList { items, state }
    }

    pub fn selected(&self) -> Option<Column> {
        self.state.selected().map(|s| self.items[s].0)
    }
}

pub fn render_column_mgr(list: &mut ColumnMGRList<'_>, area: Rect, f: &mut Frame<'_, MBackend>) {
    let layout = Layout::default()
        .margin(5)
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Percentage(80),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(area);
    let header_style = Style::default().fg(Color::Green);
    let t = vec![Span::styled("Options", header_style)];
    let help = vec![Span::styled(
        "Navigate [↑/↓] Toggle [Space] Return [o]",
        header_style,
    )];
    Paragraph::new(Spans::from(t))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center)
        .render(f, layout[0]);
    Paragraph::new(Spans::from(help))
        .wrap(Wrap { trim: false })
        .alignment(Alignment:: Center)
        .render(f, layout[2]);
    let list_items: Vec<ListItem> = list.items.iter().map(|i| i.1.clone()).collect();
    let list_widget = List::new(list_items)
        .block(Block::default().title("Columns").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Green))
        .highlight_symbol("➡ ");
    f.render_stateful_widget(list_widget, layout[1], &mut list.state);
}