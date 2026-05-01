use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use crate::app::InputMode;

use crate::app::AppState;
use crate::tasks::TaskStatus;
use crate::timer::{SessionState, SESSIONS_BEFORE_LONG_BREAK};

/// Renders the full UI. Called every draw cycle with the current app state.
pub fn render(f: &mut Frame, state: &AppState) {
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

    render_timer(f, timer_area, state);
    render_tasks(f, task_area, state);
    render_stats(f, stats_area, state);
    render_help(f, help_area, state);
}

fn render_timer(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    // Colour and label change depending on the current session type.
    let (session_label, session_color) = match &state.session {
        SessionState::Focus               => ("● FOCUS",       Color::Green),
        SessionState::ShortBreak          => ("● SHORT BREAK", Color::Yellow),
        SessionState::LongBreak           => ("● LONG BREAK",  Color::Cyan),
        SessionState::Paused(inner) => match inner.as_ref() {
            SessionState::Focus      => ("⏸ PAUSED (FOCUS)",       Color::DarkGray),
            SessionState::ShortBreak => ("⏸ PAUSED (SHORT BREAK)", Color::DarkGray),
            SessionState::LongBreak  => ("⏸ PAUSED (LONG BREAK)",  Color::DarkGray),
            _                        => ("⏸ PAUSED",               Color::DarkGray),
        },
    };

    let block = Block::default()
        .title(Line::from(Span::styled(
            "Terminal Pomodoro ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Build the 🍅 / ⬜ progress row.
    let total = SESSIONS_BEFORE_LONG_BREAK as usize;
    let filled = state.focus_sessions_completed as usize;
    let mut progress_spans: Vec<Span> = (0..total)
        .map(|i| {
            if i < filled {
                Span::styled("🍅 ", Style::default().fg(Color::Red))
            } else {
                Span::styled("⬜ ", Style::default().fg(Color::DarkGray))
            }
        })
        .collect();
    progress_spans.push(Span::raw(format!(
        " Session {} / {}",
        filled, total
    )));
    let progress = Line::from(progress_spans);

    let timer_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            state.time_display(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            session_label,
            Style::default().fg(session_color).add_modifier(Modifier::BOLD),
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

fn render_tasks(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    // Reserve 3 rows at the bottom for the input box when adding a task.
    let (list_area, input_area) = if state.is_adding() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let block = Block::default()
        .title(Line::from(Span::styled(
            " Tasks ",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = if state.tasks.tasks().is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No tasks yet — press <a> to add one",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        state.tasks.tasks().iter().enumerate().map(|(i, task)| {
            let selected = state.selected_task == Some(i);
            let row_style = if selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let cursor = if selected { "▶" } else { " " };
            match task.status {
                TaskStatus::Done => ListItem::new(Line::from(vec![
                    Span::styled(format!("{} [✓] ", cursor), row_style.fg(Color::Green)),
                    Span::styled(
                        task.title.clone(),
                        row_style.fg(Color::DarkGray).add_modifier(Modifier::DIM),
                    ),
                ])),
                _ => ListItem::new(Line::from(vec![
                    Span::styled(format!("{} [ ] ", cursor), row_style.fg(Color::Yellow)),
                    Span::styled(task.title.clone(), row_style),
                ])),
            }
        }).collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, list_area);

    // Input box shown only when the user is typing a new task name.
    if let Some(area) = input_area {
        let input_block = Block::default()
            .title(Line::from(Span::styled(
                " New Task (Enter to confirm, Esc to cancel) ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let display = format!("{}_", state.input_buf());
        let input_widget = Paragraph::new(display)
            .block(input_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(input_widget, area);
    }
}

fn render_stats(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " Stats ",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let focus_mins = state.total_focus_sessions * 25;
    let tasks_done = state.tasks.tasks().iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let completion_str = match state.completion_rate() {
        Some(rate) => format!("{}%", rate),
        None        => "—".to_string(),
    };

    let stats_text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Completed  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                state.total_focus_sessions.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Skipped    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                state.skipped_sessions.to_string(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Completion : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                completion_str,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Streak     : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} (best {})", state.current_streak, state.longest_streak),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Focus time : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}m", focus_mins),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tasks done : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                tasks_done.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
    ]);

    let paragraph = Paragraph::new(stats_text).block(block);
    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let dim = Style::default().fg(Color::DarkGray);
    let key = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    // Change the pause label dynamically.
    let pause_label = if state.session.is_paused() { " Resume  " } else { " Pause   " };

    let help_line = if matches!(state.input_mode, InputMode::AddingTask(_)) {
        Line::from(vec![
            Span::styled("<Enter>", key),
            Span::styled(" Confirm  ", dim),
            Span::styled("<Esc>", key),
            Span::styled(" Cancel", dim),
        ])
    } else {
        Line::from(vec![
            Span::styled("<Space>", key),
            Span::styled(pause_label, dim),
            Span::styled("<s>", key),
            Span::styled(" Skip  ", dim),
            Span::styled("<u>", key),
            Span::styled(" Undo  ", dim),
            Span::styled("↑↓", key),
            Span::styled(" Select  ", dim),
            Span::styled("<Enter>", key),
            Span::styled(" Complete  ", dim),
            Span::styled("<a>", key),
            Span::styled(" Add  ", dim),
            Span::styled("<d>", key),
            Span::styled(" Delete  ", dim),
            Span::styled("<q>", key),
            Span::styled(" Quit", dim),
        ])
    };

    let paragraph = Paragraph::new(Text::from(help_line))
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
