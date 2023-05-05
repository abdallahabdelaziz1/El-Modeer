use super::{Render, MBackend};
use crate::metrics::*;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::Paragraph;
use tui::Frame;
use tui::widgets::{Block, Borders, Wrap};
use crate::util::to_gb;


pub fn render_system_info(
    app: &CPUTimeApp,
    area: Rect,
    f: &mut Frame<'_, MBackend>,
    border_style: Style,
    // zf: &u32,
    // offset: &usize,
) {
    let bold_style = Style::default().add_modifier(Modifier::BOLD); 

    let mut cpu_spans = vec![Span::styled("CPU: ", bold_style)];
    
    app.cpus.iter().for_each(|(i, cpu)| {
        cpu_spans.push(Span::raw(
            format!("{:>2} ", i),
        ));
        cpu_spans.push(Span::styled(
            format!("{:>4.1}% ", cpu),
            Style::default().fg(Color::Blue),
        ));
    });

    let text = vec![
        // Memory
        Spans::from(vec![
            Span::styled("Memory: ", bold_style),
            Span::raw(format!(
                "{:>7.2}/{:>7.2} GB",
                to_gb(app.mem_utilization),
                to_gb(app.mem_total),
            )),
        ]),
        Spans::from(vec![
            Span::styled("Swap:   ", bold_style),
            Span::raw(format!(
                "{:>7.2}/{:>7.2} GB",
                to_gb(app.swap_utilization),
                to_gb(app.swap_total),
            )),
        ]),
        // Tasks (Processes)
        Spans::from(vec![
            Span::styled("Tasks: ", bold_style),
            Span::styled(format!("{:>3} ", app.total_processes), Style::default().fg(Color::White)),
            Span::raw("total, "),
            Span::styled(format!("{:>3} ", app.running_processes), Style::default().fg(Color::Green)),
            Span::raw("running, "),
            Span::styled(format!("{:>3} ", app.sleeping_processes), Style::default().fg(Color::Yellow)),
            Span::raw("sleeping, "),
            Span::styled(format!("{:>3} ", app.stopped_processes), Style::default().fg(Color::Red)),
            Span::raw("stopped, "),
            Span::styled(format!(" {:>3} ", app.zombie_processes), Style::default().fg(Color::Blue)),
            Span::raw("zombie."),
        ]),
        // CPU
        Spans::from(cpu_spans),
    ];

    Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled("System Info", border_style)))
        .style(Style::default())
        .wrap(Wrap { trim: true }).render(f, area);
}