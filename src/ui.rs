use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{AppState, InputMode};
use crate::tasks::TaskStatus;
use crate::timer::{SessionState, SESSIONS_BEFORE_LONG_BREAK};

pub fn render(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Fill(1), Constraint::Min(3)])
        .split(area);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[1]);

    render_timer(f, outer[0], state);
    render_tasks(f, middle[0], state);
    render_stats(f, middle[1], state);
    render_help(f, outer[2], state);
}

fn render_timer(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let (label, color) = match &state.session {
        SessionState::Focus               => ("● FOCUS",       Color::Green),
        SessionState::ShortBreak          => ("● SHORT BREAK", Color::Yellow),
        SessionState::LongBreak           => ("● LONG BREAK",  Color::Cyan),
        SessionState::Paused(inner) => match inner.as_ref() {
            SessionState::Focus      => ("⏸ PAUSED — add tasks then press Space to start", Color::DarkGray),
            SessionState::ShortBreak => ("⏸ PAUSED (SHORT BREAK)", Color::DarkGray),
            SessionState::LongBreak  => ("⏸ PAUSED (LONG BREAK)",  Color::DarkGray),
            _                        => ("⏸ PAUSED",               Color::DarkGray),
        },
    };

    let block = Block::default()
        .title(Span::styled("Terminal Pomodoro ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let total  = SESSIONS_BEFORE_LONG_BREAK as usize;
    let filled = state.focus_sessions_completed as usize;
    let mut progress_spans: Vec<Span> = (0..total).map(|i| {
        if i < filled { Span::styled("🍅 ", Style::default().fg(Color::Red)) }
        else          { Span::styled("⬜ ", Style::default().fg(Color::DarkGray)) }
    }).collect();
    progress_spans.push(Span::raw(format!(" Session {} / {}", filled, total)));

    let text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            state.time_display(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )).alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(label, Style::default().fg(color).add_modifier(Modifier::BOLD)))
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from(progress_spans).alignment(Alignment::Center),
    ]);

    f.render_widget(Paragraph::new(text).block(block).alignment(Alignment::Center), area);
}

fn render_tasks(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    // Split off an input box at the bottom when typing.
    let (list_area, input_area) = if state.is_typing() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let block = Block::default()
        .title(Span::styled(" Tasks ", Style::default().add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = if state.tasks.tasks().is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No tasks yet — press <a> to add one",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        state.tasks.tasks().iter().enumerate().map(|(i, task)| {
            let selected   = state.selected_task == Some(i);
            let row_style  = if selected { Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD) }
                             else        { Style::default() };
            let cursor     = if selected { "▶" } else { " " };
            match task.status {
                TaskStatus::Done => ListItem::new(Line::from(vec![
                    Span::styled(format!("{} [✓] ", cursor), row_style.fg(Color::Green)),
                    Span::styled(task.title.clone(), row_style.fg(Color::DarkGray).add_modifier(Modifier::DIM)),
                ])),
                _ => ListItem::new(Line::from(vec![
                    Span::styled(format!("{} [ ] ", cursor), row_style.fg(Color::Yellow)),
                    Span::styled(task.title.clone(), row_style),
                ])),
            }
        }).collect()
    };

    // ListState makes ratatui auto-scroll to keep the selected item visible.
    let mut list_state = ListState::default();
    list_state.select(state.selected_task);
    f.render_stateful_widget(List::new(items).block(block), list_area, &mut list_state);

    // Input box for add or edit.
    if let Some(area) = input_area {
        let (title, hint) = match &state.input_mode {
            InputMode::AddingTask(_)       => (" New Task ", " Enter to confirm • Esc to cancel "),
            InputMode::EditingTask { .. }  => (" Edit Task ", " Enter to save • Esc to cancel "),
            InputMode::Normal              => ("", ""),
        };
        let input_block = Block::default()
            .title(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            .title_bottom(Span::styled(hint, Style::default().fg(Color::DarkGray)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let display = format!("{}_", state.input_buf());
        f.render_widget(Paragraph::new(display).block(input_block).style(Style::default().fg(Color::White)), area);
    }
}

fn render_stats(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Stats ", Style::default().add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(" [ ↑  ] ↓ ", Style::default().fg(Color::DarkGray)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let focus_mins = state.total_focus_secs / 60;
    let tasks_done = state.tasks.tasks().iter().filter(|t| t.status == TaskStatus::Done).count();
    let completion_str = match state.completion_rate() {
        Some(r) => format!("{}%", r),
        None    => "—".to_string(),
    };

    let dim  = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let red  = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let yel  = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let grn  = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);

    // Using List + ListState offset so scrolling works like the task list.
    let items = vec![
        ListItem::new(Line::from(vec![Span::styled("Completed  : ", dim), Span::styled(state.total_focus_sessions.to_string(), cyan)])),
        ListItem::new(Line::from(vec![Span::styled("Skipped    : ", dim), Span::styled(state.skipped_sessions.to_string(), red)])),
        ListItem::new(Line::from(vec![Span::styled("Completion : ", dim), Span::styled(completion_str, yel)])),
        ListItem::new(Line::from(vec![
            Span::styled("Streak     : ", dim),
            Span::styled(format!("{} (best {})", state.current_streak, state.longest_streak), grn),
        ])),
        ListItem::new(Line::from(vec![Span::styled("Focus time : ", dim), Span::styled(format!("{}m", focus_mins), cyan)])),
        ListItem::new(Line::from(vec![Span::styled("Tasks done : ", dim), Span::styled(tasks_done.to_string(), cyan)])),
    ];

    let mut list_state = ListState::default().with_offset(state.stats_scroll as usize);
    f.render_stateful_widget(List::new(items).block(block), area, &mut list_state);
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let dim = Style::default().fg(Color::DarkGray);
    let key = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let pause_label = if state.session.is_paused() { " Resume  " } else { " Pause   " };

    let line = Line::from(vec![
        Span::styled("<Space>", key), Span::styled(pause_label, dim),
        Span::styled("<s>", key),     Span::styled(" Skip  ", dim),
        Span::styled("<u>", key),     Span::styled(" Undo  ", dim),
        Span::styled("<a>", key),     Span::styled(" Add  ", dim),
        Span::styled("<e>", key),     Span::styled(" Edit  ", dim),
        Span::styled("<d>", key),     Span::styled(" Delete  ", dim),
        Span::styled("<Enter>", key), Span::styled(" Toggle done  ", dim),
        Span::styled("<q>", key),     Span::styled(" Quit", dim),
    ]);

    f.render_widget(Paragraph::new(Text::from(line)).block(block).alignment(Alignment::Center), area);
}
