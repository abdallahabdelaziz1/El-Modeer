use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, thread::sleep, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};
use procfs::process;

struct App {
    state: TableState,
    items: Vec<Vec< String>>,
}

impl App {
    fn new(tablestate : Option<TableState>) -> App {
        let tps = procfs::ticks_per_second();
        let mut vec = Vec::new();
        for prc in process::all_processes().unwrap() {
            let prc = prc.unwrap();
            let stat = prc.stat().unwrap();
            // total_time is in seconds
            let total_time = (stat.utime + stat.stime) as f32 / (tps as f32);
            if stat.tty_nr().1 != 0{
                vec.push(vec![stat.pid.to_string(), stat.tty_nr().1.to_string(), total_time.to_string(), stat.comm])
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
    terminal.draw(|f| ui(f, &mut app))?;

    loop{
        if std::time::Instant::now() - last_draw_time >= draw_interval {
            terminal.clear()?;
            app = App::new(Some(app.state));
            terminal.draw(|f| ui(f, &mut app))?;
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(0)
        .split(f.size());

  //  let row_style = Style::default().add_modifier(Modifier::DIM);
    let row_style = Style::default().bg(Color::Rgb(165, 165, 165));
    let header_style = Style::default().bg(Color::White);
    let header_cells = ["PID", "TTY", "TIME", "CMD"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Black)));
    let header = Row::new(header_cells)
        .style(header_style)
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
    let n :u32 = app.items[0].len() as u32;
    let widths = [
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n),
        Constraint::Ratio(1, n)
    ];
    
    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(row_style)
        .widths(&widths);
    f.render_stateful_widget(t, rects[0], &mut app.state);
}