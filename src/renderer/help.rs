use crate::metrics::*;
use crate::renderer::{Render, MBackend};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::Frame;

pub fn render_help(
    _app: &CPUTimeApp,
    area: Rect,
    f: &mut Frame<'_, MBackend>,
) {
    let header_style = Style::default().fg(Color::Green);
    let main_style = Style::default();
    let key_style = main_style.fg(Color::Cyan);

    static GLOBAL_KEYS: &[[&str; 2]] = &[
        ["h    ", "    Toggle this help screen\n"],
        ["q    ", "    Quit and exit El-Modeer\n"],
        ["f    ", "    Freeze refreshing\n"],
        ["i    ", "    Show Section Selection Menu\n"],
        ["o    ", "    Show Column Selection Menu of the Process Table\n"],
        ["←    ", "    Change the Sorting Column to the Next one\n"],
        ["→    ", "    Change the Sorting Column to the Previous one\n"],
        ["`    ", "    Reset charts to current\n"],
    ];

    static PROCESS_TABLE_KEYS: &[[&str; 2]] = &[
        ["<RET> ", "    Focus current process\n"],
        ["↓     ", "    Move one line down\n"],
        ["↑     ", "    Move one line up\n"],
        ["PgDown", "    Move view one screen down\n"],
        ["PgUp  ", "    Move view one screen up\n"],
        ["Home  ", "    Move to top\n"],
        ["End   ", "    Move to bottom\n"],
        [";     ", "    Change sort between ascending/descending\n"],
        [",     ", "    Cycle columns left\n"],
        [".     ", "    Cycle columns right\n"],
        ["p     ", "    Toggle paths on/off\n"],
        ["/     ", "    Enter filter mode\n"],
        ["c     ", "    Enter filter by Category mode\n"],
        ["k     ", "    Kill a process using its PID\n"],
        ["s     ", "    Suspend (stop) a process using its PID\n"],
        ["r     ", "    Resume a (stopped) process using its PID\n"],
        ["n     ", "    Nice a process (change its priority) using its PID and the new nice value\n"],
        ["<ESC> ", "    Leave any action mode\n"],
    ];

    let mut t = vec![Spans::from(vec![Span::styled(
        "Primary Interface",
        header_style,
    )])];

    for [key, text] in GLOBAL_KEYS {
        t.push(Spans::from(vec![
            Span::styled(*key, key_style),
            Span::styled(*text, main_style),
        ]));
    }

    t.push(Spans::from(vec![Span::styled("", header_style)]));
    t.push(Spans::from(vec![Span::styled(
        "Process Table\n",
        header_style,
    )]));

    for [key, text] in PROCESS_TABLE_KEYS {
        t.push(Spans::from(vec![
            Span::styled(*key, key_style),
            Span::styled(*text, main_style),
        ]));
    }

    let help_height = t.len() as u16;

    let help_layout = Layout::default()
        .horizontal_margin(5)
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(help_height),
                Constraint::Max(5),
            ]
            .as_ref(),
        )
        .split(area);
    let (title_area, help_area) = (help_layout[0], help_layout[1]);

    let b = Block::default().borders(Borders::ALL);
    Paragraph::new(t)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left)
        .block(b)
        .render(f, help_area);

    let t = vec![Span::styled(
        concat!("El-modeer v", env!("CARGO_PKG_VERSION")),
        header_style,
    )];
    Paragraph::new(Spans::from(t))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .render(f, title_area);
}
