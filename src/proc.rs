use crate::styles::{BOLD, GREEN, BLUE, RED, WHITE, YELLOW, WHITE_BG, ROW_BG, BLACK};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};

use procfs::process;
use sysinfo::{CpuExt, System, SystemExt};

const TO_GB: f64 = 1024_f64 * 1024_f64 * 1024_f64;

struct App {
    state: TableState,
    items: Vec<Vec<String>>,
}

impl App {
    fn new(tablestate: Option<TableState>) -> App {
        let tps = procfs::ticks_per_second();
        let mut vec = Vec::new();
        for prc in process::all_processes().unwrap() {
            let prc = prc.unwrap();
            let stat = prc.stat().unwrap();
            // total_time is in seconds
            let total_time = (stat.utime + stat.stime) as f32 / (tps as f32);
            if stat.tty_nr().1 != 0 {
                vec.push(vec![
                    stat.pid.to_string(),
                    stat.tty_nr().1.to_string(),
                    total_time.to_string(),
                    stat.comm,
                ])
            }

            //    let in_vec = [stat.pid, stat.tty_nr().1, total_time, stat.comm].iter().map(|item| item.to_string()).collect::<Vec<_>>();
            //     vec.push(in_vec);
        }
        App {
            state: tablestate.unwrap_or(TableState::default()),
            items: vec,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub fn proc() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let draw_interval = Duration::from_millis(100);
    let mut last_draw_time = std::time::Instant::now();
    let mut app = App::new(None);

    // Create a sys (System object) to be accessible from everywhere
    let mut sys: System = System::new();
    sys.refresh_all(); // First we update all information of our `System` struct.

    terminal.draw(|f| ui(f, &mut app, &mut sys))?;

    loop {
        if std::time::Instant::now() - last_draw_time >= draw_interval {
            terminal.clear()?;
            app = App::new(Some(app.state));
            terminal.draw(|f| ui(f, &mut app, &mut sys))?;
            last_draw_time = std::time::Instant::now();
        }

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
    // create app and run it
    // let app = App::new();
    // let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // if let Err(err) = res {
    //     println!("{:?}", err)
    // }

    Ok(())
}

// fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
//     loop {
//         terminal.draw(|f| ui(f, &mut app))?;

//         if let Event::Key(key) = event::read()? {
//             match key.code {
//                 KeyCode::Char('q') => return Ok(()),
//                 KeyCode::Down => app.next(),
//                 KeyCode::Up => app.previous(),
//                 _ => {}
//             }
//         }
//     }
// }

// Function that return a widget that displays the system information
fn get_sys_widget<'a>(sys: &mut System) -> Paragraph<'a> {
    // Number of tasks (processes)
    let mut total = 0;
    let mut running = 0;
    let mut sleeping = 0;
    let mut stopped = 0;
    let mut zombie = 0;

    for proc in process::all_processes().unwrap() {
        total += 1;
        let proc = proc.unwrap();
        match proc.stat().unwrap().state {
            'R' => running += 1,
            'S' => sleeping += 1,
            'T' => stopped += 1,
            'Z' => zombie += 1,
            _ => (),
        }
    }

    let mut cpu_spans = vec![Span::styled("CPU: ", *BOLD)];

    sys.refresh_cpu();
    sys.cpus().iter().enumerate().for_each(|(i, cpu)| {
        cpu_spans.push(Span::raw(
            format!("{:>2} ", i + 1),
        ));
        cpu_spans.push(Span::styled(
            format!("{:>5.2}% ", cpu.cpu_usage()),
            *BLUE,
        ));
    });

    let text = vec![
        // Memory
        Spans::from(vec![
            Span::styled("Memory: ", *BOLD),
            Span::raw(format!(
                "{:>7.2}/{:>7.2} GB",
                sys.used_memory() as f64 / TO_GB,
                sys.total_memory() as f64 / TO_GB,
            )),
        ]),
        Spans::from(vec![
            Span::styled("Swap:   ", *BOLD),
            Span::raw(format!(
                "{:>7.2}/{:>7.2} GB",
                sys.used_swap() as f64 / TO_GB,
                sys.total_swap() as f64 / TO_GB,
            )),
        ]),
        // Tasks (Processes)
        Spans::from(vec![
            Span::styled("Tasks: ", *BOLD),
            Span::styled(format!("{:>3} ", total), *WHITE),
            Span::raw("total, "),
            Span::styled(format!("{:>3} ", running), *GREEN),
            Span::raw("running, "),
            Span::styled(format!("{:>3} ", sleeping), *YELLOW),
            Span::raw("sleeping, "),
            Span::styled(format!("{:>3} ", stopped), *RED),
            Span::raw("stopped, "),
            Span::styled(format!(" {:>3} ", zombie), *BLUE),
            Span::raw("zombie."),
        ]),
        // CPU
        Spans::from(cpu_spans),
    ];

    let top_data = Paragraph::new(text)
        .block(Block::default().title("System Info").borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .wrap(Wrap { trim: true });

    top_data
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App, sys: &mut System) {
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Percentage(100)].as_ref())
        .margin(0)
        .split(f.size());

    let header_cells = ["PID", "TTY", "TIME", "CMD"]
        .iter()
        .map(|h| Cell::from(*h).style(*BLACK));
    let header = Row::new(header_cells)
        .style(*WHITE_BG)
        .height(1)
        .bottom_margin(0);
    let rows = app.items.iter().map(|item| {
        let height = item
            .iter()
            .map(|content| content.chars().filter(|c| *c == '\n').count())
            .max()
            .unwrap_or(0)
            + 1;
        let cells = item.iter().map(|c| Cell::from(c.clone()));
        Row::new(cells).height(height as u16).bottom_margin(0)
    });
    let n: u32 = app.items[0].len() as u32;
    let widths = [
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n),
    ];

    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(*ROW_BG)
        .widths(&widths);

    f.render_widget(get_sys_widget(sys), rects[0]);
    f.render_stateful_widget(t, rects[1], &mut app.state);
}
