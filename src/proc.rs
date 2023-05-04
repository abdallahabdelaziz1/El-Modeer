use crate::styles::{BOLD, GREEN, BLUE, RED, WHITE, YELLOW, WHITE_BG, ROW_BG, BLACK};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, thread, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};


use crossbeam_channel::{select, tick, unbounded, Receiver};
use num_rational::Ratio;
use sysinfo::{CpuExt, System, SystemExt, Process, ProcessExt, UserExt, ProcessStatus};
use users::{Users, UsersCache};
const TO_GB: f64 = 1024_f64 * 1024_f64 * 1024_f64;



fn status_to_single_char(status: ProcessStatus) -> String {
    let s = match status {
        ProcessStatus::Idle => "I",
        ProcessStatus::Run => "R",
        ProcessStatus::Sleep => "S",
        ProcessStatus::Stop => "T",
        ProcessStatus::Zombie => "Z",
        ProcessStatus::Tracing => "t",
        ProcessStatus::Dead => "x",
        ProcessStatus::Wakekill => "K",
        ProcessStatus::Waking => "W",
        ProcessStatus::Parked => "P",
        ProcessStatus::Unknown(_) => "U",
        _ => "U"
    };

    s.to_string()
}



struct App {
    state: TableState,
    items: Vec<Vec<String>>,
}

impl App {
    fn new(tablestate: Option<TableState>, sys: &mut System) -> App {
        let tps = procfs::ticks_per_second();
        let mut vec = Vec::new();

        let process_list = sys.processes();


        for (pid, process) in process_list {
                        
            //  User
            // PID
            // PPID (Parent PID)
            // TTY
            /// CPU utilization
            // CPU time
            // CMD (Command)
            // Start time => Neglect
            // Priority (nice value)
            // Background or foreground => Neglect
            // Category (sleeping, running, zombie, stopped) 

           let mut user_name = String::from("Not Known");
           if let Some(user_id) = process.user_id() {
                user_name = match sys.get_user_by_id(user_id) {
                           Some(s) => s.name().to_string(),
                           None => String::from("Not Known")
                }
        
            }
            //let ppid = process.parent().unwrap().to_string();

            let pid_i32 :i32 = pid.to_string().parse().unwrap();
            

            let fproc = match procfs::process::Process::new(pid_i32) {
                    Ok((p)) =>  p,
                    Err(error) => continue,
            };

           // let fproc = procfs::process::Process::new(pid_i32).unwrap_or_else(|| None);
            let stat = fproc.stat().unwrap();
            let tty = format!("pts/{}", stat.tty_nr().1).to_string(); //TODO(Adjust tty)

            let cpu_usage  = process.cpu_usage().to_string();

            let total_time = (stat.utime + stat.stime) as f32 / (tps as f32);

            //let cmd = (*process.cmd())[0].to_string();

            let cmd = process.cmd().join("");

            let priority = stat.nice.to_string();

            let status = status_to_single_char(process.status());
           
            let ppid = stat.ppid.to_string();



            vec.push(vec![pid.to_string(), user_name, ppid, tty, cpu_usage, total_time.to_string(),
                cmd, priority, status
            ])
                        


        }

        // for prc in procfs::process::all_processes().unwrap() {
        //     let prc = prc.unwrap();
        //     let stat = prc.stat().unwrap();
        //     // total_time is in seconds
        //     let pid = stat.pid;
        //     let total_time = (stat.utime + stat.stime) as f32 / (tps as f32);
        //     let tty = format!("pts/{}", stat.tty_nr().1); //TODO(Adjust tty)



        //     vec.push(vec![pid.to_string(), tty.to_string(), total_time.to_string(), stat.comm])
            
        // }
        App {
            state: tablestate.unwrap_or(TableState::default()),
            items: vec,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.items.len() - 1
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
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

fn setup_ui_events() -> Receiver<Event> {
	let (sender, receiver) = unbounded();
	thread::spawn(move || loop {
		sender.send(crossterm::event::read().unwrap()).unwrap();
	});

	receiver
}

fn setup_ctrl_c() -> Receiver<()> {
	let (sender, receiver) = unbounded();
	ctrlc::set_handler(move || {
		sender.send(()).unwrap();
	})
	.unwrap();

	receiver
}


pub fn proc() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    let draw_interval = Ratio::from_integer(1);
    let ticker = tick(Duration::from_secs_f64(
		*draw_interval.numer() as f64 / *draw_interval.denom() as f64,
	));

    

    // Create a sys (System object) to be accessible from everywhere
    let mut sys: System = System::new();
    sys.refresh_all(); // First we update all information of our `System` struct.
    let mut app = App::new(None, &mut sys);
    sys.processes();
  

    let ctrl_c_events = setup_ctrl_c();
    let ui_events_receiver = setup_ui_events();
    terminal.draw(|f| ui(f, &mut app, &mut sys))?;

    loop {
        select!{
            recv(ctrl_c_events) -> _ => {
				break;
			}
            recv(ticker) -> _ => {
                terminal.clear()?;
                app = App::new(Some(app.state), &mut sys);
                terminal.draw(|f| ui(f, &mut app, &mut sys))?;
            }
            recv(ui_events_receiver) -> message => {
                match message.unwrap(){
                    Event::Key(key_event) => {
						if key_event.modifiers.is_empty() {
                            match key_event.code{
                                KeyCode::Char('q') => break, 
                                KeyCode::Down => app.next(),
                                KeyCode::Up => app.previous(),
                                _ => {}
                            }
                        }else if key_event.modifiers == KeyModifiers::CONTROL {
							match key_event.code {
								KeyCode::Char('c') => {
									break
								},
                                _ => {}
                            }
                        }

                    },
                    _ => {}
                }
            }

        }
    }

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


// Function that return a widget that displays the system information
fn get_sys_widget<'a>(sys: &mut System) -> Paragraph<'a> {
    // Number of tasks (processes)
    let mut total = 0;
    let mut running = 0;
    let mut sleeping = 0;
    let mut stopped = 0;
    let mut zombie = 0;

    for proc in procfs::process::all_processes().unwrap() {
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
///   vec.push(vec![pid.to_string(), user_name, ppid, tty, cpu_usage, total_time.to_string(),
// cmd, priority, status
// ])


    let header_cells = ["PID", "USER", "PPID", "TTY", "CPU%", "TIME", "CMD", "Priority", "Status"]
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
    
    //[1u32; n];
    let widths = [Constraint::Ratio(1, n); 9];
    // let widths = [
    //     Constraint::Ratio(1, n),
    //     Constraint::Ratio(1, n),
    //     Constraint::Ratio(1, n),
    //     Constraint::Ratio(1, n),
    // ];

    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(*ROW_BG)
        .widths(&widths);

    f.render_widget(get_sys_widget(sys), rects[0]);
    f.render_stateful_widget(t, rects[1], &mut app.state);
}
