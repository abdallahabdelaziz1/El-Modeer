use super::{percent_of, Render, MBackend};
use crate::float_to_byte_string;
use crate::metrics::mprocess::{ProcessStatusExt, MProcess};
use crate::metrics::{CPUTimeApp, ProcessTableSortOrder};
use crate::renderer::column::Column;
use byte_unit::{Byte, ByteUnit};
use chrono::prelude::DateTime;
use chrono::Local;
use num_traits::FromPrimitive;
use std::borrow::Cow;
use std::time::{Duration, UNIX_EPOCH};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
use tui::Frame;

pub fn render_process_table(
    app: &CPUTimeApp,
    process_table: &[i32],
    area: Rect,
    process_table_start: usize,
    f: &mut Frame<'_, MBackend>,
    proc_columns: &Vec<Column>,
    border_style: Style,
    show_paths: bool,
    show_find: bool,
    show_find_cat: bool,
    show_kill: bool,
    show_suspend: bool,
    show_resume: bool,
    show_nice: bool,
    show_rate: bool,
    filter: &str,
    action_pid: &str,
    action_input: &str,
    new_rate: &str,
    process_table_message: &String,
    highlighted_row: usize,
    freeze: bool,
    tick_rate: u64,
) -> Option<Box<MProcess>> {
    // 4 for the margins and table header
    let display_height = match area.height.saturating_sub(4) {
        0 => return None,
        v => v as usize,
    };

    let procs: Vec<&MProcess> = process_table
        .iter()
        .map(|pid| {
            app.process_map
                .get(pid)
                .expect("expected pid to be present")
        })
        .collect();
    let highlighted_process = if !procs.is_empty() {
        Some(Box::new(procs[highlighted_row].clone()))
    } else {
        None
    };
    if area.height < 5 {
        return highlighted_process; // not enough space to draw anything
    }

    // TODO: Make sure to update the header as well
    

    let rows: Vec<Row> = procs
        .iter()
        .enumerate()
        .skip(process_table_start)
        .take(display_height)
        .map(|(i, p)| {
            let cmd_string = if show_paths {
                if p.command.len() > 1 {
                    format!(" - {:}", p.command.join(" "))
                } else if !p.command.is_empty() {
                    format!(" - {:}", p.command[0])
                } else {
                    String::from("")
                }
            } else if p.command.len() > 1 {
                format!(" {:}", p.command[1..].join(" "))
            } else {
                String::from("")
            };

            // Loop over columns and add cells to the row
            let mut row = vec![];

            for column in proc_columns {
                match column {
                    Column::PID => row.push(Cell::from(format!("{: >width$}", p.pid, width = app.max_pid_len))),
                    Column::PPID => row.push(Cell::from(format!("{: >width$}", p.ppid, width = app.max_pid_len))),
                    Column::User => row.push(Cell::from(format!("{: <10}", p.user_name))),
                    Column::Priority => row.push(Cell::from(format!("{: <3}", p.priority))),
                    Column::Nice => row.push(Cell::from(format!("{: <3}", p.nice))),
                    Column::Status => row.push(Cell::from(format!("{:1}", p.status.to_single_char()))),
                    Column::TTY => row.push(Cell::from(format!("{: <10}", p.tty))),
                    Column::CPUPercentage => row.push(Cell::from(format!("{:>5.1}", p.cpu_usage))),
                    Column::MemoryPercentage => row.push(Cell::from(format!("{:>5.1}", percent_of(p.memory, app.mem_total)))),
                    Column::Memory => row.push(Cell::from(format!("{:>8}", float_to_byte_string!(p.memory as f64, ByteUnit::B).replace('B', "")))),
                    Column::VirtualMemory => row.push(Cell::from(format!("{:>8}", float_to_byte_string!(p.virtual_memory as f64, ByteUnit::KB).replace('B', "")))),
                    Column::CPUTime => row.push(Cell::from(format!("{:>5.1}", p.cpu_time))),
                    Column::StartTime => row.push(Cell::from(format!("{:>5.1}", p.start_time))),
                    Column::CMD => row.push(Cell::from(format!("{:}{:}", p.name, cmd_string))),
                }
            }
            
            let row = Row::new(row);

            if i == highlighted_row {
                row.style(
                    Style::default()
                        .bg(Color::Gray)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                row
            }
        })
        .collect();

    // let mut header = vec![
    //     format!("{:<width$}", "PID", width = app.max_pid_len + 1),
    //     format!("{:<width$}", "PPID", width = app.max_pid_len + 1),
    //     String::from("USER         "),
    //     String::from("P   "),
    //     String::from("N  "),
    //     String::from("S "),
    //     String::from("TTY   "),
    //     String::from("CPU%  "),
    //     String::from("MEM%  "),
    //     String::from("MEM     "),
    //     String::from("VIRT     "),
    //     String::from("CPUTIME "),
    //     String::from("STARTTIME   "),
    //     // String::from("READ/s   "),
    //     // String::from("WRITE/s  "),
    //     // String::from("IOWAIT% "),
    // ];


    // Loop over columns and add cells to the row
    let mut header = vec![];

    for column in proc_columns {
        match column {
            Column::PID => header.push(format!("{:<width$}", "PID", width = app.max_pid_len + 1)),
            Column::PPID => header.push(format!("{:<width$}", "PPID", width = app.max_pid_len + 1)),
            Column::User => header.push(String::from("USER         ")),
            Column::Priority => header.push(String::from("P   ")),
            Column::Nice => header.push(String::from("N  ")),
            Column::Status => header.push(String::from("S ")),
            Column::TTY => header.push(String::from("TTY   ")),
            Column::CPUPercentage => header.push(String::from("CPU%  ")),
            Column::MemoryPercentage => header.push(String::from("MEM%  ")),
            Column::Memory => header.push(String::from("MEM     ")),
            Column::VirtualMemory => header.push(String::from("VIRT     ")),
            Column::CPUTime => header.push(String::from("CPUTIME ")),
            Column::StartTime => header.push(String::from("STARTTIME   ")),
            _ => {}
        }
    }

    let mut widths = Vec::with_capacity(header.len() + 1);
    let mut used_width = 0;
    for item in &header {
        let len = item.len() as u16;
        widths.push(Constraint::Length(len));
        used_width += len;
    }
    let cmd_width = f.size().width.saturating_sub(used_width).saturating_sub(3);
    let cmd_header = format!("{:<width$}", "CMD", width = cmd_width as usize);
    
    if proc_columns.contains(&Column::CMD) {
        widths.push(Constraint::Min(cmd_width));
        header.push(cmd_header);
    }

    let mut sort_index = 0;
    let mut found = false;
     for i in 0..14 { // TODO: make 14 a constant
        let column: Column = FromPrimitive::from_u32(i as u32)
                .expect("Index not in range for Column enum");
        if proc_columns.contains(&column) {
            if column == app.psortby {
                found = true;
                break;
            }
            else {
                sort_index += 1;
            }
        }
    }
    if !found {
        sort_index = 0;
    }


    header[sort_index].pop();
    let sort_ind = match app.psortorder {
        ProcessTableSortOrder::Ascending => '↑',
        ProcessTableSortOrder::Descending => '↓',
    };
    header[sort_index].insert(0, sort_ind); //sort column indicator
    let header_row: Vec<Cell> = header
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if i == sort_index {
                Cell::from(c.as_str()).style(
                    Style::default()
                        .bg(Color::Gray)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Cell::from(c.as_str())
            }
        })
        .collect();
    let title = if show_find {
        format!("[ESC] Clear, Find: {:}", filter)
    } else if show_find_cat {
        format!("[ESC] Clear, Category: {:}", filter)
    }
    else if !filter.is_empty() {
        format!("Filtered Results: {:}, [/] to change/clear", filter)
    } else if show_kill {
        format!("[ESC] Clear, PID to kill: {:}{}", action_pid, process_table_message)
    } else if show_suspend {
        format!("[ESC] Clear, PID to suspend: {:}{}", action_pid, process_table_message)
    } else if show_resume {
        format!("[ESC] Clear, PID to resume: {:}{}", action_pid, process_table_message)
    } else if show_nice {
        format!("[ESC] Clear, PID to nice: {:}{}{}", action_pid, process_table_message, action_input)
    } else if show_rate {
        format!("[ESC] Clear, set refresh rate in millis: {:}{}", new_rate, process_table_message)
    }
     else {
        format!("Freeze [f] Sort Col [,/.] Asc/Dec [;] Filter [/] Category [c] Kill [k] Suspend [s] Resume [r] Nice [n]")
    };

    Table::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, border_style)),
        )
        .widths(widths.as_slice())
        .column_spacing(0)
        .header(
            Row::new(header_row)
                .style(Style::default().bg(Color::DarkGray))
                .bottom_margin(0),
        )
        .render(f, area);


    let frozen_text = vec![Spans::from(vec![
            Span::styled("  FROZEN  ", Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)),
        ])];
        
    if freeze{
        Paragraph::new(frozen_text)
        .block(Block::default())
        .render(f, Rect::new(1, f.size().height.saturating_sub(2), 12, 1));
    }
    else {
        let rate_text = vec![Spans::from(vec![
            Span::styled("  Refresh Rate:", Style::default().fg(Color::White).bg(Color::DarkGray)),
            Span::styled(format!(" {}  ", tick_rate), Style::default().fg(Color::White).bg(Color::DarkGray)),
        ])];

        let l : u16 = (tick_rate.to_string().len() + 18) as u16;

        Paragraph::new(rate_text)
            .block(Block::default())
            .render(f, Rect::new(f.size().right().saturating_sub(l+1), f.size().height.saturating_sub(2), l, 1));
    }

    highlighted_process
}

pub fn render_process(
    app: &CPUTimeApp,
    layout: Rect,
    f: &mut Frame<'_, MBackend>,
    border_style: Style,
    process_message: &Option<String>,
    p: &MProcess,
    freeze: bool,
    tick_rate: u64,
) {
    Block::default()
        .title(Span::styled(format!("Process: {0}", p.name), border_style))
        .borders(Borders::ALL)
        .border_style(border_style)
        .render(f, layout);
    let v_sections = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(2), Constraint::Min(1)].as_ref())
        .split(layout);

    let title = format!("(b)ack (n)ice (p)riority 0 (s)uspend (r)esume (k)ill [SIGKILL] (t)erminate [SIGTERM] {:} {: >width$}", 
                        process_message.as_ref().unwrap_or(&String::from("")), "", width = layout.width as usize);

    Block::default()
        .title(Span::styled(
            title,
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ))
        .render(f, v_sections[0]);

    //Block::default().borders(Borders::LEFT).render(f, h_sections[1]);

    let alive = if p.end_time.is_some() {
        format!(
            "dead since {:}",
            DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(p.end_time.unwrap()))
        )
    } else {
        "alive".to_string()
    };
    let run_duration = p.get_run_duration();
    let d = format!(
        "{:0>2}:{:0>2}:{:0>2}",
        run_duration.num_hours(),
        run_duration.num_minutes() % 60,
        run_duration.num_seconds() % 60
    );

    let rhs_style = Style::default().fg(Color::Green);
    let mut text = vec![
        Spans::from(vec![
            Span::raw("Name:                  "),
            Span::styled(format!("{:} ({:})", &p.name, alive), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("PID:                   "),
            Span::styled(
                format!("{:>width$}", &p.pid, width = app.max_pid_len),
                rhs_style,
            ),
        ]),
        Spans::from(vec![
            Span::raw("Command:               "),
            Span::styled(p.command.join(" "), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("User:                  "),
            Span::styled(&p.user_name, rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("Start Time:            "),
            Span::styled(
                format!(
                    "{:}",
                    DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(p.start_time))
                ),
                rhs_style,
            ),
        ]),
        Spans::from(vec![
            Span::raw("Total Run Time:        "),
            Span::styled(d, rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("CPU Usage:             "),
            Span::styled(format!("{:>7.2} %", &p.cpu_usage), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("Threads:               "),
            Span::styled(format!("{:>7}", &p.threads_total), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("Status:                "),
            Span::styled(format!("{:}", p.status), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("Priority:              "),
            Span::styled(format!("{:>7}", p.priority), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("Nice:                  "),
            Span::styled(format!("{:>7}", p.nice), rhs_style),
        ]),
        Spans::from(vec![
            Span::raw("MEM Usage:             "),
            Span::styled(
                format!("{:>7.2} %", percent_of(p.memory, app.mem_total)),
                rhs_style,
            ),
        ]),
        Spans::from(vec![
            Span::raw("Total Memory:          "),
            Span::styled(
                format!(
                    "{:>10}",
                    float_to_byte_string!(p.memory as f64, ByteUnit::KB)
                ),
                rhs_style,
            ),
        ]),
        Spans::from(vec![
            Span::raw("Disk Read:             "),
            Span::styled(
                format!(
                    "{:>10} {:}/s",
                    float_to_byte_string!(p.read_bytes as f64, ByteUnit::B),
                    float_to_byte_string!(
                        p.get_read_bytes_sec(&Duration::from_millis(tick_rate)), // TODO: make this a setting
                        ByteUnit::B
                    )
                ),
                rhs_style,
            ),
        ]),
        Spans::from(vec![
            Span::raw("Disk Write:            "),
            Span::styled(
                format!(
                    "{:>10} {:}/s",
                    float_to_byte_string!(p.write_bytes as f64, ByteUnit::B),
                    float_to_byte_string!(
                        p.get_write_bytes_sec(&Duration::from_millis(tick_rate)), // TODO: make this a setting
                        ByteUnit::B
                    )
                ),
                rhs_style,
            ),
        ]),
    ];

    let frozen_text = vec![Spans::from(vec![
        Span::styled("  FROZEN  ", Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)),
    ])];

    // if !app.gfx_devices.is_empty() {
    //     text.push(Spans::from(vec![
    //         Span::raw("SM Util:            "),
    //         Span::styled(format!("{:7.2} %", p.sm_utilization as f64), rhs_style),
    //     ]));
    //     text.push(Spans::from(vec![
    //         Span::raw("Frame Buffer:       "),
    //         Span::styled(format!("{:7.2} %", p.fb_utilization as f64), rhs_style),
    //     ]));
    //     text.push(Spans::from(vec![
    //         Span::raw("Encoder Util:       "),
    //         Span::styled(format!("{:7.2} %", p.enc_utilization as f64), rhs_style),
    //     ]));
    //     text.push(Spans::from(vec![
    //         Span::raw("Decoder Util:       "),
    //         Span::styled(format!("{:7.2} %", p.dec_utilization as f64), rhs_style),
    //     ]));
    // }

    #[cfg(target_os = "linux")]
    text.push(Spans::from(vec![
        Span::raw("IO Wait:               "),
        Span::styled(
            format!(
                "{:>7.2} % ({:>7.2} %)",
                p.get_io_wait(&Duration::from_millis(tick_rate)), // TODO: make this a setting
                p.get_total_io_wait()
            ),
            rhs_style,
        ),
    ]));
    #[cfg(target_os = "linux")]
    text.push(Spans::from(vec![
        Span::raw("Swap Wait:             "),
        Span::styled(
            format!(
                "{:>7.2} % ({:>7.2} %)",
                p.get_swap_wait(&Duration::from_millis(tick_rate)), // TODO: make this a setting
                p.get_total_swap_wait()
            ),
            rhs_style,
        ),
    ]));

    if text.len() > v_sections[1].height as usize * 3 {
        let h_sections = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints(
                [
                    Constraint::Percentage(50),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ]
                .as_ref(),
            )
            .split(v_sections[1]);

        let second_part = text.split_off(h_sections[0].height as usize * 3);
        Paragraph::new(text)
            .block(Block::default())
            .wrap(Wrap { trim: false })
            .render(f, h_sections[0]);

        Paragraph::new(second_part)
            .block(Block::default())
            .wrap(Wrap { trim: false })
            .render(f, h_sections[2]);

    } else {
        Paragraph::new(text)
        .block(Block::default())
        .wrap(Wrap { trim: true })
        .render(f, v_sections[1]);
    }
    
    if freeze{
        Paragraph::new(frozen_text)
        .block(Block::default())
        .render(f, Rect::new(1, f.size().height.saturating_sub(2), 12, 1));
    }
    else {
        let rate_text = vec![Spans::from(vec![
            Span::styled("  Refresh Rate:", Style::default().fg(Color::White).bg(Color::DarkGray)),
            Span::styled(format!(" {}  ", tick_rate), Style::default().fg(Color::White).bg(Color::DarkGray)),
        ])];

        let l : u16 = (tick_rate.to_string().len() + 18) as u16;

        Paragraph::new(rate_text)
            .block(Block::default())
            .render(f, Rect::new(f.size().right().saturating_sub(l+1), f.size().height.saturating_sub(2), l, 1));
    }
}

pub fn filter_process_table<'a>(app: &'a CPUTimeApp, filter: &str, show_find_cat: bool) -> Cow<'a, [i32]> {
    if filter.is_empty() {
        return Cow::Borrowed(&app.processes);
    }

    let filter_lc = filter.to_lowercase();
    #[allow(unused_assignments)]
    let mut results : Vec<i32> = Vec::new();
    if !show_find_cat {
        results = app
        .processes
        .iter()
        .filter(|pid| {
            let p = app
                .process_map
                .get(pid)
                .expect("Pid present in processes but not in map.");
            p.name.to_lowercase().contains(&filter_lc)
                || p.exe.to_lowercase().contains(&filter_lc)
                || p.command.join(" ").to_lowercase().contains(&filter_lc)
                || format!("{:}", p.pid).contains(&filter_lc)
                || p.status.to_string().to_lowercase().contains(&filter_lc)
                || p.user_name.to_lowercase().contains(&filter_lc)
        })
        .copied()
        .collect();
    }
    else{
        results = app
        .processes
        .iter()
        .filter(|pid| {
            let p = app
                .process_map
                .get(pid)
                .expect("Pid present in processes but not in map.");
            p.status.to_string().to_lowercase().contains(&filter_lc)
        })
        .copied()
        .collect();

    }
   
    results.into()
}

// fn set_process_row_style<'a>(
//     current_pid: i32,
//     test_pid: Option<i32>,
//     row_content: String,
// ) -> Cell<'a> {
//     match test_pid {
//         Some(p) => {
//             if p == current_pid {
//                 Cell::from(row_content).style(Style::default().fg(Color::Red))
//             } else {
//                 Cell::from(row_content)
//             }
//         }
//         None => Cell::from(row_content),
//     }
// }
