mod system_info;
mod help;
pub mod macros;
mod process;
pub mod section;
pub mod column;
use crate::constants::NUMBER_OF_COLUMNS;
use crate::metrics::mprocess::*;
use crate::metrics::*;
use crate::renderer::section::{Section, SectionMGRList};
use crate::renderer::column::{Column, ColumnMGRList};
use crate::util::*;
use crate::{convert_result_to_string, convert_error_to_string};
use crossterm::{
    event::{KeyCode as Key, KeyEvent, KeyModifiers},
    execute,
    terminal::EnterAlternateScreen,
};
use num_traits::FromPrimitive;
use sysinfo::SystemExt;
use std::io;
use std::io::Stdout;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, Terminal};
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::Frame;
use heim::process as hproc;
use heim::process::ProcessError;


const PROCESS_SELECTION_GRACE: Duration = Duration::from_millis(2000); //TODO: check this

type MBackend = CrosstermBackend<Stdout>;

/// Compatibility trait, that preserves an older method from tui 0.6.5
/// Exists mostly to keep the caller code idiomatic for the use cases in this file
/// May be refactored out later if the widget usage patterns change
trait Render<B>
where
    B: Backend,
{
    fn render(self, f: &mut Frame<B>, area: Rect);
}

impl<T, B> Render<B> for T
where
    T: tui::widgets::Widget,
    B: Backend,
{
    fn render(self, f: &mut Frame<B>, area: Rect) {
        f.render_widget(self, area)
    }
}

/// current size of the terminal returned as (columns, rows)
fn terminal_size() -> (u16, u16) {
    crossterm::terminal::size().expect("Failed to get terminal size")
}

/// ceil to nearest upper even number
macro_rules! ceil_even {
    ($x:expr) => {
        ($x + 1) / 2 * 2
    };
}

/// Convert percentage heights to length constraints. This is done since sections other
/// than process have two sub-parts and should be of even height.
fn eval_constraints(
    section_geometry: &[(Section, f64)],
    height: u16,
    borrowed: &mut bool,
) -> Vec<Constraint> {
    let mut constraints = vec![Constraint::Length(1)];
    let avail_height = height as i32 - 1;
    let mut process_index = -1;
    let mut max_others = 0;
    let mut max_others_index = -1;
    let mut sum_others = 0;
    // each section should have a height of at least 2 rows
    let mut max_section_height = avail_height - section_geometry.len() as i32 * 2;
    // process section is at least 4 rows high
    if section_geometry.iter().any(|s| s.0 == Section::Process) {
        max_section_height -= 2;
    }
    // convert percentage heights to length constraints and apply additional
    // criteria that height should be even number for non-process sections
    for (section_index, section) in section_geometry.iter().enumerate() {
        let required_height = section.1 * avail_height as f64 / 100.0;
        // ensure max_section_height is at least 2 after every recalculation
        max_section_height = max_section_height.max(2);
        if section.0 == Section::Process {
            process_index = section_index as i32;
            constraints.push(Constraint::Min(4));
        } else {
            // round to nearest even size for the two sub-parts in each section display
            let section_height =
                max_section_height.min(ceil_even!(required_height.floor().max(1.0) as i32));
            sum_others += section_height;
            // adjust max_section_height for subsequent sections
            max_section_height -= section_height - 2;
            if section_height >= max_others {
                max_others = section_height;
                max_others_index = section_index as i32;
            }
            constraints.push(Constraint::Length(section_height as u16));
        }
    }
    // remaining is what will be actually used for process section but if its too small (due to
    // rounding to even heights for other sections), then borrow rows from the largest section
    if process_index != -1 {
        let process_height = avail_height - sum_others;
        if process_height < 4 && max_others > 4 {
            let borrow = ceil_even!(4 - process_height).min(max_others - 4);
            // (max_others - borrow) will be >= 4 due to the min() above so cast to u16 is safe
            constraints[max_others_index as usize + 1] =
                Constraint::Length((max_others - borrow) as u16);
            constraints[process_index as usize + 1] =
                Constraint::Min((process_height + borrow) as u16);
            *borrowed = true;
        } else {
            constraints[process_index as usize + 1] = Constraint::Min(process_height as u16);
        }
    }

    constraints
}

fn get_constraints(section_geometry: &[(Section, f64)], height: u16) -> Vec<Constraint> {
    let mut borrowed = false;
    eval_constraints(section_geometry, height, &mut borrowed)
}

pub struct TerminalRenderer<'a> {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: CPUTimeApp,
    events: Events,
    process_table_row_start: usize,

    /// Index in the vector below is "order" on the screen starting from the top
    /// (usually CPU) while value is the section it belongs to and its current height (as %).
    /// Currently all sections are stacked on top of one another horizontally and
    /// occupy entire width of the screen but this may change going forward. For the case
    /// where there are multiple sections stacked vertically, the "order" can have the
    /// convention of top-bottom and left-right in each horizontal layer and the width of
    /// each section be tracked below. For more generic positioning (e.g. sections cutting
    /// across others vertically), this mapping needs to also include the position of
    /// top-left corner of the section. In that case the only significance that the
    /// "order" will have is the sequence in which the TAB key will shift focus
    /// among the sections.
    section_geometry: Vec<(Section, f64)>,
    proc_columns: Vec<Column>,
    zoom_factor: u32,
    update_number: u32,
    selected_section_index: usize,
    constraints: Vec<Constraint>,
    process_message: Option<String>,
    process_table_message: String,
    show_help: bool,
    show_paths: bool,
    show_find: bool,
    show_find_cat: bool,
    show_kill: bool,
    show_suspend: bool,
    show_resume: bool,
    show_nice: bool,
    show_rate: bool,
    show_section_mgr: bool,
    show_column_mgr: bool,
    freeze: bool,
    filter: String,
    action_pid: String,
    action_input: String,
    new_rate: String,
    highlighted_row: usize,
    selection_grace_start: Option<Instant>,
    section_manager_options: SectionMGRList<'a>,
    column_manager_options: ColumnMGRList<'a>,
    recompute_constraints_on_start_up: bool,
    tick_rate: u64,
}

impl<'a> TerminalRenderer<'_> {
    pub fn new(
        tick_rate: u64,
        section_geometry: &[(Section, f64)],
    ) -> TerminalRenderer {
        let app = CPUTimeApp::new(Duration::from_millis(tick_rate));
        let events = Events::new(Duration::from_millis(tick_rate));

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).expect("Unable to enter alternate screen");
        let backend = CrosstermBackend::new(stdout);
        let mut terminal =
            Terminal::new(backend).expect("Couldn't create new terminal with backend");
        terminal.hide_cursor().ok();

        let constraints = get_constraints(section_geometry, terminal_size().1);
        let section_geometry = section_geometry.to_vec();
        let recompute_constraints_on_start_up = false;
        let mut default_cols = vec![
            Column::PID, 
            Column::PPID, 
            Column::TTY, 
            Column::Nice,
            Column::Status, 
            Column::User, 
            Column::CPUPercentage, 
            Column::CMD
        ];
        default_cols.sort();

        TerminalRenderer {
            terminal,
            app,
            events,
            process_table_row_start: 0,
            section_geometry: section_geometry.clone(),
            proc_columns: default_cols.clone(),
            zoom_factor: 1,
            update_number: 0,
            // select the last section by default (normally should be Process)
            selected_section_index: section_geometry.len() - 1,
            constraints,
            process_message: None,
            process_table_message: String::from(""),
            show_help: false,
            show_paths: false,
            show_find: false,
            show_find_cat: false,
            show_kill: false,
            show_suspend: false,
            show_resume: false,
            show_nice: false,
            show_section_mgr: false,
            show_column_mgr: false,
            show_rate: false,
            freeze: false,
            filter: String::from(""),
            action_pid: String::from(""),
            action_input: String::from(""),
            new_rate: String::from(""),
            highlighted_row: 0,
            selection_grace_start: None,
            section_manager_options: SectionMGRList::with_geometry(section_geometry),
            column_manager_options: ColumnMGRList::with_cols(default_cols),
            recompute_constraints_on_start_up,
            tick_rate,
        }
    }

    fn selected_section(&self) -> Section {
        self.section_geometry[self.selected_section_index].0
    }

    pub async fn start(&mut self) {
        if self.recompute_constraints_on_start_up {
            self.recompute_constraints();
            self.recompute_constraints_on_start_up = false;
        }
        loop {
            let app = &self.app;
            let pst = &self.process_table_row_start;
            let mut width: u16 = 0;
            let mut process_table_height: u16 = 0;
            let proc_columns = &self.proc_columns;
            let constraints = &self.constraints;
            let geometry = &self.section_geometry.to_vec();
            let section_manager_options = &mut self.section_manager_options;
            let column_manager_options = &mut self.column_manager_options;
            // let selected = self.section_geometry[self.selected_section_index].0;
            let process_message = &self.process_message;
            let process_table_message = &self.process_table_message;
            let show_help = self.show_help;
            let show_section_mgr = self.show_section_mgr;
            let show_column_mgr = self.show_column_mgr;
            let show_paths = self.show_paths;
            let freeze = self.freeze;
            let filter = &self.filter;
            let show_find = self.show_find;
            let show_find_cat = self.show_find_cat;
            let show_kill = self.show_kill;
            let show_suspend = self.show_suspend;
            let show_resume = self.show_resume;
            let show_rate = self.show_rate;
            let show_nice = self.show_nice;
            let action_pid = &self.action_pid;
            let action_input = &self.action_input;
            let new_rate = &self.new_rate;
            let mut highlighted_process: Option<Box<MProcess>> = None;
            let process_table = process::filter_process_table(app, &self.filter, self.show_find_cat);

            if !process_table.is_empty() && self.highlighted_row >= process_table.len() {
                self.highlighted_row = process_table.len() - 1;
            }
            let highlighted_row = self.highlighted_row;

            let tick_rate = self.tick_rate;
            self.terminal
                .draw(|f| {
                    width = f.size().width;
                    if show_help {
                        let v_sections = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(0)
                            .constraints([Constraint::Length(1), Constraint::Length(40)].as_ref())
                            .split(f.size());

                        help::render_help(app, v_sections[1], f);
                    } else if show_section_mgr {
                        let v_sections = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(0)
                            .constraints([Constraint::Length(1), Constraint::Length(40)].as_ref())
                            .split(f.size());
                        section::render_section_mgr(section_manager_options, v_sections[1], f);
                    } else if show_column_mgr {
                        let v_columns = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints([Constraint::Length(1), Constraint::Length(40)].as_ref())
                            .split(f.size());
                        column::render_column_mgr(column_manager_options, v_columns[1], f);
                    } else {
                        // create layouts (primary vertical)
                        let v_sections = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(0)
                            .constraints(constraints.as_slice())
                            .split(f.size());
                        
                        for section_index in 0..geometry.len() {
                            let v_section = v_sections[section_index + 1];
                            let current_section = geometry[section_index].0;
                            
                            match current_section {
                                Section::SystemInfo => {
                                    system_info::render_system_info(
                                        app,
                                        v_section,
                                        f,
                                    );
                                }
                                Section::Process => {
                                    if let Some(p) = app.selected_process.as_ref() {
                                        process::render_process(
                                            app,
                                            v_section,
                                            f,
                                            process_message,
                                            p,
                                            freeze, 
                                            tick_rate
                                        );

                                    } else {
                                        highlighted_process = process::render_process_table(
                                            app,
                                            &process_table,
                                            v_section,
                                            *pst,
                                            f,
                                            proc_columns,
                                            show_paths,
                                            show_find,
                                            show_find_cat,
                                            show_kill,
                                            show_suspend,
                                            show_resume,
                                            show_nice,
                                            show_rate,
                                            filter,
                                            action_pid,
                                            action_input,
                                            new_rate,
                                            process_table_message,
                                            highlighted_row,
                                            freeze,
                                            tick_rate,
                                        );
                                        if v_section.height > 4 {
                                            // account for table border & margins.
                                            process_table_height = v_section.height - 5;
                                        }
                                    }
                                }
                            }
                        }
                    }
                })
                .expect("Could not draw frame.");

            let event = self.events.next().expect("No new event.");
            let action = match event {
                Event::Input(input) => {
                    let process_table = process_table.into_owned();
                    self.process_key_event(
                        input,
                        &process_table,
                        process_table_height,
                        highlighted_process,
                    )
                    .await
                }
                Event::Resize(_, height) => {
                    self.constraints = get_constraints(&self.section_geometry, height);
                    Action::Continue
                }
                Event::Tick => {
                    self.process_tick().await;
                    Action::Continue
                }
                Event::Terminate => {
                    Action::Quit
                }
                _ => Action::Continue,
            };
            match action {
                Action::Quit => break,
                Action::Continue => {}
            }
        }
    }

    async fn process_tick(&mut self) {
        if self.app.selected_process.is_none() {
            if let Some(start) = self.selection_grace_start {
                if start.elapsed() > PROCESS_SELECTION_GRACE {
                    self.selection_grace_start = None;
                }
            }
        }

        let keep_order =
            self.app.selected_process.is_some() || self.selection_grace_start.is_some();

        if !self.freeze {
            self.app.update(keep_order).await;
            self.update_number += 1;
            if self.update_number == self.zoom_factor {
                self.update_number = 0;
            }
        }
       
    }

    async fn process_key_event(
        &mut self,
        input: KeyEvent,
        process_table: &[i32],
        process_table_height: u16,
        highlighted_process: Option<Box<MProcess>>,
    ) -> Action {
        match input.code {
            Key::Up => self.view_up(process_table, 1),
            Key::PageUp => self.view_up(process_table, process_table_height.into()),
            Key::Down => self.view_down(process_table, process_table_height.into(), 1),
            Key::PageDown => self.view_down(
                process_table,
                process_table_height.into(),
                process_table_height.into(),
            ),
            Key::Home => self.view_up(process_table, process_table.len()),
            Key::End => self.view_down(
                process_table,
                process_table_height.into(),
                process_table.len(),
            ),
            Key::Enter => {
                if self.show_kill {
                    if self.action_pid.chars().all(|c| c.is_digit(10)) && !self.action_pid.is_empty() {
                        self.process_table_message = match hproc::get(self.action_pid.parse().unwrap()).await {
                            Ok(p) => convert_result_to_string!(p.kill().await),
                            Err(e) => convert_error_to_string!(e),
                        };
                    } else {
                        self.process_table_message = "Invalid PID".to_string();
                    }
                    self.action_pid = String::new();
                } else if self.show_suspend {
                    if self.action_pid.chars().all(|c| c.is_digit(10)) && !self.action_pid.is_empty() {
                        self.process_table_message = match hproc::get(self.action_pid.parse().unwrap()).await {
                            Ok(p) => convert_result_to_string!(p.suspend().await),
                            Err(e) => convert_error_to_string!(e),
                        };
                    } else {
                        self.process_table_message = "Invalid PID".to_string();
                    }
                    self.action_pid = String::new();

                } else if self.show_resume {
                    if self.action_pid.chars().all(|c| c.is_digit(10)) && !self.action_pid.is_empty() {
                        self.process_table_message = match hproc::get(self.action_pid.parse().unwrap()).await {
                            Ok(p) => convert_result_to_string!(p.resume().await),
                            Err(e) => convert_error_to_string!(e),
                        };
                    } else {
                        self.process_table_message = "Invalid PID".to_string();
                    }
                    self.action_pid = String::new();

                } else if self.show_nice && self.process_table_message == " Choose a nice value (-20 to 19): ".to_string() {
                    // Set the priority of the process to the specified value
                    let nice_value: i32 = self.action_input.parse().unwrap();
                    if nice_value < -20 || nice_value > 19 {
                        self.process_table_message = "Invalid nice value".to_string();  
                    } else {
                        let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, self.action_pid.parse().unwrap(), self.action_input.parse().unwrap()) };
                        if result == -1 {
                            self.process_table_message = "Failed to set process priority".to_string();
                        } else {
                            self.process_table_message = "Process priority set successfully".to_string();
                        }  
                    }

                    self.action_pid = String::new();
                    self.action_input = String::new();                  
                } else if self.show_nice {
                    if self.action_pid.is_empty() || !self.action_pid.chars().all(|c| c.is_digit(10)) {
                        self.process_table_message = "Invalid PID".to_string();
                        self.action_pid = String::new();
                    } else {
                        // Check if the process exists
                        let process = self.app.system.get_process(self.action_pid.parse().unwrap()).is_some(); 
                        if process {
                            self.process_table_message = " Choose a nice value (-20 to 19): ".to_string();
                        } else {
                            self.process_table_message = "No Such Process".to_string();
                            self.action_pid = String::new();
                        }
                    }
                } else if self.show_rate {
                    if self.new_rate.chars().all(|c| c.is_digit(10)) && !self.new_rate.is_empty(){
                        let r: u64 = self.new_rate.parse().unwrap();
                        if r < 1000{
                            self.process_table_message = "The rate must be at least 1000 millis".to_string();
                        }
                        else{
                            self.tick_rate = r;
                            self.app.change_tick(Duration::from_millis(self.tick_rate));
                            self.process_table_message = "The rate has been set".to_string();
                        }
                    }
                    else{
                        self.process_table_message = "Invalid rate".to_string();
                    }
                    self.new_rate = String::new();
                } 
                else {
                    self.select(highlighted_process);
                }
            }
            Key::Char('c') => {
                if input.modifiers.contains(KeyModifiers::CONTROL) {
                    return Action::Quit;
                } else if self.show_find || self.show_find_cat{
                    self.process_find_input(input);
                } else if self.show_kill {
                    self.process_kill_input(input);
                } else if self.show_suspend {
                    self.process_suspend_input(input);
                } else if self.show_resume {
                    self.process_resume_input(input);
                } else if self.show_nice {
                    self.process_nice_input(input);
                } else if self.show_rate{
                    self.process_rate_input(input);
                }
                else{
                    self.show_find_cat = true;
                    self.highlighted_row = 0;
                    self.process_table_row_start = 0;
                }
            }
            _other => {
                if self.show_find || self.show_find_cat{
                    self.process_find_input(input);
                } else if self.show_kill {
                    self.process_kill_input(input);
                } else if self.show_suspend {
                    self.process_suspend_input(input);
                } else if self.show_resume {
                    self.process_resume_input(input);
                } else if self.show_nice && self.process_table_message == " Choose a nice value (-20 to 19): ".to_string() {
                    self.process_nice_value_input(input);
                } else if self.show_nice {
                    self.process_nice_input(input);
                } else if self.show_rate{
                    self.process_rate_input(input);
                } else {
                    return self.process_toplevel_input(input).await;
                }
            }
        };
        Action::Continue
    }

    fn select(&mut self, highlighted_process: Option<Box<MProcess>>) {
        let selected = self.selected_section();
        if selected == Section::Process {
            self.app.select_process(highlighted_process);
            self.process_message = None;
            self.process_table_message = String::from("");
            self.show_find = false;
            self.show_find_cat = false;
            self.show_kill = false;
            self.show_suspend = false;
            self.show_resume = false;
            self.show_nice = false;
            self.show_rate = false;
            self.process_table_row_start = 0;
        }
    }

    fn view_up(&mut self, process_table: &[i32], delta: usize) {
        let selected = self.selected_section();
        if self.show_section_mgr {
            match self.section_manager_options.state.selected() {
                Some(i) => {
                    let mut idx = 0;
                    if (i as i32 - delta as i32) > 0 {
                        idx = i - delta;
                    }
                    self.section_manager_options.state.select(Some(idx));
                }
                None => self.section_manager_options.state.select(Some(0)),
            }
        } else if self.show_column_mgr {
            match self.column_manager_options.state.selected() {
                Some(i) => {
                    let mut idx = 0;
                    if (i as i32 - delta as i32) > 0 {
                        idx = i - delta;
                    }
                    self.column_manager_options.state.select(Some(idx));
                }
                None => self.section_manager_options.state.select(Some(0)),
            }
        } else if selected == Section::Process {
            if self.app.selected_process.is_some() || process_table.is_empty() {
                return;
            }

            self.selection_grace_start = Some(Instant::now());
            if self.highlighted_row != 0 {
                self.highlighted_row = self.highlighted_row.saturating_sub(delta);
            }
            if self.process_table_row_start > 0
                && self.highlighted_row < self.process_table_row_start
            {
                self.process_table_row_start = self.process_table_row_start.saturating_sub(delta);
            }
        }
    }

    fn view_down(&mut self, process_table: &[i32], process_table_height: usize, delta: usize) {
        use std::cmp::min;
        let selected = self.selected_section();
        if self.show_section_mgr {
            match self.section_manager_options.state.selected() {
                Some(i) => {
                    let mut idx = self.section_manager_options.items.len() - 1;
                    if i + delta < idx {
                        idx = i + delta;
                    }
                    self.section_manager_options.state.select(Some(idx));
                }
                None => self.section_manager_options.state.select(Some(0)),
            }
        } else if self.show_column_mgr {
            match self.column_manager_options.state.selected() {
                Some(i) => {
                    let mut idx = self.column_manager_options.items.len() -1;
                    if i + delta < idx {
                        idx = i + delta;
                    }
                    self.column_manager_options.state.select(Some(idx));
                }
                None => self.column_manager_options.state.select(Some(0)),
            }
        } else if selected == Section::Process {
            if self.app.selected_process.is_some() || process_table.is_empty() {
                return;
            }

            self.selection_grace_start = Some(Instant::now());
            if self.highlighted_row < process_table.len() - 1 {
                self.highlighted_row = min(self.highlighted_row + delta, process_table.len() - 1);
            }
            if self.process_table_row_start < process_table.len()
                && self.highlighted_row > (self.process_table_row_start + process_table_height)
            {
                self.process_table_row_start = min(
                    self.process_table_row_start + delta,
                    process_table.len() - process_table_height - 1,
                );
            }
        }
    }

    fn process_find_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_find = false;
                self.show_find_cat = false;
                self.filter = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.filter.push(c)
            }
            Key::Delete => match self.filter.pop() {
                Some(_c) => {}
                None => {self.show_find = false; self.show_find_cat = false;},
            },
            Key::Backspace => match self.filter.pop() {
                Some(_c) => {}
                None => {self.show_find = false; self.show_find_cat = false;},
            },
            _ => {}
        }
    }

    fn process_kill_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_kill = false;
                self.action_pid = String::from("");
                self.process_table_message = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.process_table_message = String::from("");
                self.action_pid.push(c)
            }
            Key::Delete => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_kill = false,
            },
            Key::Backspace => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_kill = false,
            },
            _ => {}
        }
    }

    fn process_suspend_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_suspend = false;
                self.action_pid = String::from("");
                self.process_table_message = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.process_table_message = String::from("");
                self.action_pid.push(c)
            }
            Key::Delete => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_suspend = false,
            },
            Key::Backspace => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_suspend = false,
            },
            _ => {}
        }
    }

    fn process_resume_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_resume = false;
                self.action_pid = String::from("");
                self.process_table_message = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.process_table_message = String::from("");
                self.action_pid.push(c)
            }
            Key::Delete => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_resume = false,
            },
            Key::Backspace => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_resume = false,
            },
            _ => {}
        }
    }

    fn process_nice_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_nice = false;
                self.action_pid = String::from("");
                self.process_table_message = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.process_table_message = String::from("");
                self.action_pid.push(c)
            }
            Key::Delete => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_nice = false,
            },
            Key::Backspace => match self.action_pid.pop() {
                Some(_c) => {}
                None => self.show_nice = false,
            },
            _ => {}
        }
    }

    fn process_nice_value_input(&mut self, input: KeyEvent) {
        match input.code {
            Key::Esc => {
                self.show_nice = false;
                self.action_pid = String::from("");
                self.action_input = String::from("");
            }
            Key::Char(c) if c != '\n' => {
                self.selection_grace_start = Some(Instant::now());
                self.action_input.push(c)
            }
            Key::Delete => match self.action_input.pop() {
                Some(_c) => {}
                None => self.show_nice = false,
            },
            Key::Backspace => match self.action_input.pop() {
                Some(_c) => {}
                None => self.show_nice = false,
            },
            _ => {}
        }
    }

    fn process_rate_input(&mut self, input: KeyEvent) {
        match input.code {
           Key::Esc => {
               self.show_rate = false;
               self.process_table_message = String::from("");
               self.new_rate = String::from("");
           }
           Key::Char(c) if c != '\n' => {
               self.selection_grace_start = Some(Instant::now());
               self.process_table_message = String::from("");
               self.new_rate.push(c)
           }
           Key::Delete => match self.new_rate.pop() {
               Some(_c) => {}
               None => self.show_rate = false,
           },
           Key::Backspace => match self.new_rate.pop() {
               Some(_c) => {}
               None => self.show_rate = false,
           },
           _ => {}
       }
   }

    fn recompute_constraints(&mut self) {
        self.selected_section_index = self.section_geometry.len()-1;
        if self.section_geometry.len() == 1 {
            self.section_geometry[0].1 = 100.0;
        } else {
            self.section_geometry[0].1 = 18.0;
            self.section_geometry[1].1 = 82.0;
        }
        let new_geometry = self.section_geometry.clone();
        let selected = self.section_manager_options.state.selected();
        self.section_manager_options = SectionMGRList::with_geometry(new_geometry);
        self.section_manager_options.state.select(selected);
        self.constraints = get_constraints(self.section_geometry.as_slice(), terminal_size().1);
    }

    fn update_columns(&mut self) {
        let new_proc_cols = self.proc_columns.clone();
        let selected = self.column_manager_options.state.selected();
        self.column_manager_options = ColumnMGRList::with_cols(new_proc_cols);
        self.column_manager_options.state.select(selected);
    }

    fn toggle_section(&mut self) {
        if self.show_section_mgr {
            if let Some(s) = self.section_manager_options.selected() {
                // The section is there and needs to be removed but at least one section should remain
                if self.section_geometry.len() > 1
                    && self.section_geometry.iter().any(|(gs, _)| *gs == s)
                {
                    self.section_geometry.retain(|(section, _)| *section != s);
                    self.recompute_constraints();
                } 
                // The section is not there and needs to be added
                else if !self.section_geometry.iter().any(|(gs, _)| *gs == s) {
                    let idx = 0;
                    self.section_geometry.insert(idx, (s, 1.0));
                    self.section_geometry
                        .sort_by(|(a_section, _), (b_section, _)| {
                            a_section
                                .partial_cmp(b_section)
                                .expect("Can't compare sections. Shouldn't happen.")
                        });
                    self.recompute_constraints();
                }
            }
        }

        if self.show_column_mgr {
            if let Some(c) = self.column_manager_options.selected() {
                if self.proc_columns.len() > 1
                        && self.proc_columns.iter().any(|gc| *gc == c)
                {
                    self.proc_columns.retain(|section| *section != c);
                    self.update_columns();
                } else if !self.proc_columns.iter().any(|gc| *gc == c) {
                    let idx = 0;
                    self.proc_columns.insert(idx, c);
                    self.proc_columns.sort();
                    self.update_columns();
                }
            }
        }
    }
    

    fn toggle_section_mgr(&mut self) {
        self.show_section_mgr = !self.show_section_mgr;
    }

    fn toggle_column_mgr(&mut self) {
        self.show_column_mgr = !self.show_column_mgr;
    }

    fn sort_by_next_column(&mut self) {
        if self.proc_columns.len() == 1 {
            return;
        }
        
        let mut next_column = self.app.psortby;
        let mut found = false;
        while !found {
            next_column = FromPrimitive::from_u32((next_column as u32 + 1) % NUMBER_OF_COLUMNS) // TODO: Add num of cols
                .expect("invalid value to set psortby");
            if self.proc_columns.contains(&next_column) {
                found = true;
            }
        }
        self.app.psortby = next_column;
        self.app.sort_process_table();
    }

    fn sort_by_prev_column(&mut self) {
        if self.proc_columns.len() == 1 {
            return;
        }
        
        let mut prev_column = self.app.psortby;
        let mut found = false;
        while !found {
            prev_column = FromPrimitive::from_u32((prev_column as i32 - 1 + NUMBER_OF_COLUMNS as i32) as u32 % NUMBER_OF_COLUMNS) 
                .expect("invalid value to set psortby");
            if self.proc_columns.contains(&prev_column) {
                found = true;
            }
        }
        self.app.psortby = prev_column;
        self.app.sort_process_table();
    }
  
    async fn process_toplevel_input(&mut self, input: KeyEvent) -> Action {
        match input.code {
            Key::Char('q') => {
                return Action::Quit;
            }
            Key::Char('.') | Key::Char('>') => {
                self.sort_by_next_column();
            }
            Key::Char(',') | Key::Char('<') => {
                self.sort_by_prev_column();
            }
            Key::Char(';') => {
                match self.app.psortorder {
                    ProcessTableSortOrder::Ascending => {
                        self.app.psortorder = ProcessTableSortOrder::Descending
                    }
                    ProcessTableSortOrder::Descending => {
                        self.app.psortorder = ProcessTableSortOrder::Ascending
                    }
                }
                self.app.sort_process_table();
            }
            Key::Esc | Key::Char('b') => {
                self.app.selected_process = None;
                self.process_message = None;
            }
            Key::Char('s') => {
                if self.app.selected_process.is_none() {
                    self.show_suspend = true;
                }
                self.process_message = match &self.app.selected_process {
                    Some(p) => Some(p.suspend().await),
                    None => None,
                };
            }
            Key::Char('r') => {
                if self.app.selected_process.is_none() {
                    self.show_resume = true;
                }
                self.process_message = match &self.app.selected_process {
                    Some(p) => Some(p.resume().await),
                    None => None,
                };
            }
            Key::Char('k') => {
                if self.app.selected_process.is_none() {
                    self.show_kill = true;
                }
                self.process_message = match &self.app.selected_process {
                    Some(p) => Some(p.kill().await),
                    None => None,
                };
            }
            Key::Char('t') => {
                self.process_message = match &self.app.selected_process {
                    Some(p) => Some(p.terminate().await),
                    None => None,
                };
            }
            Key::Char('n') => {
                if self.app.selected_process.is_none() {
                    self.show_nice = true;
                }
                self.process_message = self.app.selected_process.as_mut().map(|p| p.nice());
            }
            Key::Char('p') if self.app.selected_process.is_some() => {
                self.process_message = self
                    .app
                    .selected_process
                    .as_mut()
                    .map(|p| p.set_priority(0));
            }
            Key::Char(' ') => {
                self.toggle_section();
            }
            Key::Char('o') => {
                self.toggle_column_mgr();
            }
            Key::Char('i') => {
                self.toggle_section_mgr();
            }
            Key::Char('h') => {
                self.show_help = !self.show_help;
            }
            Key::Char('f') => {
                self.freeze = !self.freeze;
            }
            Key::Char('p') => {
                self.show_paths = !self.show_paths;
            }
            Key::Char('/') => {
                self.show_find = true;
                self.highlighted_row = 0;
                self.process_table_row_start = 0;
            }
            _ => {}
        }

        Action::Continue
    }
}

#[must_use]
enum Action {
    Continue,
    Quit,
}