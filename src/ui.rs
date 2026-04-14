/// Stub — Nikhil's module.
/// ratatui rendering logic will be implemented here.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Renders the full static UI skeleton, called once per `terminal.draw()` invocation.
pub fn render(f: &mut Frame) {
    let area = f.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Fill(1),
            Constraint::Min(3),
        ])
        .split(area);

    let timer_area  = outer[0];
    let middle_area = outer[1];
    let help_area   = outer[2];

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(middle_area);

    let task_area  = middle[0];
    let stats_area = middle[1];

    render_timer(f, timer_area);
    render_tasks(f, task_area);
    render_stats(f, stats_area);
    render_help(f, help_area);
}

fn render_timer(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled("Terminal Pomodoro ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let progress = Line::from(vec![
        Span::styled("🍅 ", Style::default().fg(Color::Red)),
        Span::styled("🍅 ", Style::default().fg(Color::Red)),
        Span::styled("⬜ ", Style::default().fg(Color::DarkGray)),
        Span::styled("⬜ ", Style::default().fg(Color::DarkGray)),
        Span::raw(" Session 2 / 4"),
    ]);

    let timer_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "25:00",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "● FOCUS",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        progress.alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(timer_text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_tasks(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " Tasks ",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled("[ ] ", Style::default().fg(Color::Yellow)),
            Span::raw("Write project report"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("[ ] ", Style::default().fg(Color::Yellow)),
            Span::raw("Review pull requests"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("[✓] ", Style::default().fg(Color::Green)),
            Span::styled("Set up repo", Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
        ])),
    ];

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_stats(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " Stats ",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let stats_text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Sessions today : ", Style::default().fg(Color::DarkGray)),
            Span::styled("2", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Focus time     : ", Style::default().fg(Color::DarkGray)),
            Span::styled("50m", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Tasks done     : ", Style::default().fg(Color::DarkGray)),
            Span::styled("1", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Streak         : ", Style::default().fg(Color::DarkGray)),
            Span::styled("3 days", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
    ]);

    let paragraph = Paragraph::new(stats_text).block(block);
    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let dim   = Style::default().fg(Color::DarkGray);
    let key   = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let help_line = Line::from(vec![
        Span::styled("<Space>", key),
        Span::styled(" Pause  ", dim),
        Span::styled("<s>", key),
        Span::styled(" Skip  ", dim),
        Span::styled("<a>", key),
        Span::styled(" Add Task  ", dim),
        Span::styled("<d>", key),
        Span::styled(" Delete  ", dim),
        Span::styled("<q>", key),
        Span::styled(" Quit", dim),
    ]);

    let paragraph = Paragraph::new(Text::from(help_line))
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
