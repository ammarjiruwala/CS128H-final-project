use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{AppState, InputMode};
use crate::stats;
use crate::tasks::TaskStatus;
use crate::timer::{SessionState, SESSIONS_BEFORE_LONG_BREAK};

// ---------------------------------------------------------------------------
// Heatmap colour palette (GitHub-style green gradient)
// ---------------------------------------------------------------------------

fn heatmap_cell(focus_secs: u64) -> Span<'static> {
    // Thresholds: 0 | <15m | <30m | <50m | 50m+
    match focus_secs {
        0           => Span::styled("▪", Style::default().fg(Color::Rgb(30, 35, 40))),
        1..=899     => Span::styled("▪", Style::default().fg(Color::Rgb(14, 68, 41))),
        900..=1799  => Span::styled("▪", Style::default().fg(Color::Rgb(0,  109, 50))),
        1800..=2999 => Span::styled("▪", Style::default().fg(Color::Rgb(38, 166, 65))),
        _           => Span::styled("▪", Style::default().fg(Color::Rgb(57, 211, 83))),
    }
}

// ---------------------------------------------------------------------------
// Top-level render
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Timer panel
// ---------------------------------------------------------------------------

fn render_timer(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let (label, color) = match &state.session {
        SessionState::Focus               => ("● FOCUS",       Color::Green),
        SessionState::ShortBreak          => ("● SHORT BREAK", Color::Yellow),
        SessionState::LongBreak           => ("● LONG BREAK",  Color::Cyan),
        SessionState::Paused(inner) => match inner.as_ref() {
            SessionState::Focus      => ("⏸  PAUSED — add tasks then press Space to start", Color::DarkGray),
            SessionState::ShortBreak => ("⏸  PAUSED (SHORT BREAK)", Color::DarkGray),
            SessionState::LongBreak  => ("⏸  PAUSED (LONG BREAK)",  Color::DarkGray),
            _                        => ("⏸  PAUSED",               Color::DarkGray),
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

// ---------------------------------------------------------------------------
// Task list panel
// ---------------------------------------------------------------------------

fn render_tasks(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
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
            let selected  = state.selected_task == Some(i);
            let row_style = if selected { Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD) }
                            else        { Style::default() };
            let cursor    = if selected { "▶" } else { " " };
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

    let mut list_state = ListState::default();
    list_state.select(state.selected_task);
    f.render_stateful_widget(List::new(items).block(block), list_area, &mut list_state);

    if let Some(area) = input_area {
        let (title, hint) = match &state.input_mode {
            InputMode::AddingTask(_)      => (" New Task ", " Enter to confirm • Esc to cancel "),
            InputMode::EditingTask { .. } => (" Edit Task ", " Enter to save • Esc to cancel "),
            InputMode::Normal             => ("", ""),
        };
        let input_block = Block::default()
            .title(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            .title_bottom(Span::styled(hint, Style::default().fg(Color::DarkGray)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let display = format!("{}_", state.input_buf());
        f.render_widget(
            Paragraph::new(display).block(input_block).style(Style::default().fg(Color::White)),
            area,
        );
    }
}

// ---------------------------------------------------------------------------
// Stats panel
// ---------------------------------------------------------------------------

fn render_stats(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Stats ", Style::default().add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(
            " [ ↑  ] ↓ ",
            Style::default().fg(Color::DarkGray),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // How many heatmap weeks fit in the panel (each cell = 1 char + 1 space, plus "M " label).
    let inner_width = area.width.saturating_sub(4) as usize; // 2 borders + 2 label chars
    let weeks       = (inner_width / 2).clamp(6, 16);

    let all     = stats::compute_all_time_stats(&state.session_history);
    let heatmap = stats::build_heatmap(&state.session_history, weeks);

    let dim     = Style::default().fg(Color::DarkGray);
    let bold    = Style::default().add_modifier(Modifier::BOLD);
    let cyan    = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let red     = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    let yel     = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let grn     = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
    let section = Style::default().fg(Color::DarkGray);

    let day_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

    // ── Heatmap ──────────────────────────────────────────────────────────────
    let mut items: Vec<ListItem> = Vec::new();

    items.push(ListItem::new(Line::from(Span::styled(
        " Activity",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ))));

    // Legend row
    let legend = Line::from(vec![
        Span::styled("   ", dim),
        Span::styled("▪", Style::default().fg(Color::Rgb(30, 35, 40))),
        Span::styled(" none  ", dim),
        Span::styled("▪", Style::default().fg(Color::Rgb(14, 68, 41))),
        Span::styled("▪", Style::default().fg(Color::Rgb(0, 109, 50))),
        Span::styled("▪", Style::default().fg(Color::Rgb(38, 166, 65))),
        Span::styled("▪", Style::default().fg(Color::Rgb(57, 211, 83))),
        Span::styled(" more", dim),
    ]);
    items.push(ListItem::new(legend));

    for (day_idx, row) in heatmap.iter().enumerate() {
        let mut spans = vec![
            Span::styled(
                format!("{} ", day_labels[day_idx]),
                Style::default().fg(Color::DarkGray),
            ),
        ];
        for &secs in row {
            spans.push(heatmap_cell(secs));
            spans.push(Span::raw(" "));
        }
        items.push(ListItem::new(Line::from(spans)));
    }

    // ── Separator ─────────────────────────────────────────────────────────────
    items.push(ListItem::new(Line::from(Span::styled(
        "─".repeat(inner_width + 2),
        section,
    ))));

    // ── All-time summary ──────────────────────────────────────────────────────
    items.push(ListItem::new(Line::from(Span::styled(" All time", bold))));

    let at_mins = all.total_focus_secs / 60;
    let at_time = if at_mins >= 60 {
        format!("{}h {}m", at_mins / 60, at_mins % 60)
    } else {
        format!("{}m", at_mins)
    };
    let at_total = all.total_completed + all.total_skipped;
    let at_rate  = if at_total > 0 { all.total_completed * 100 / at_total } else { 0 };

    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Done : ", dim),
        Span::styled(all.total_completed.to_string(), cyan),
        Span::styled("  Skip : ", dim),
        Span::styled(all.total_skipped.to_string(), red),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Rate : ", dim),
        Span::styled(format!("{}%", at_rate), yel),
        Span::styled("  Best : ", dim),
        Span::styled(format!("{} 🔥", all.best_streak), grn),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Time : ", dim),
        Span::styled(at_time, cyan),
    ])));

    // ── Separator ─────────────────────────────────────────────────────────────
    items.push(ListItem::new(Line::from(Span::styled(
        "─".repeat(inner_width + 2),
        section,
    ))));

    // ── This session ──────────────────────────────────────────────────────────
    items.push(ListItem::new(Line::from(Span::styled(" This session", bold))));

    let sess_mins      = state.total_focus_secs / 60;
    let completion_str = match state.completion_rate() {
        Some(r) => format!("{}%", r),
        None    => "—".to_string(),
    };
    let tasks_done = state.tasks.tasks().iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();

    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Done : ", dim),
        Span::styled(state.total_focus_sessions.to_string(), cyan),
        Span::styled("  Skip : ", dim),
        Span::styled(state.skipped_sessions.to_string(), red),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Rate : ", dim),
        Span::styled(completion_str, yel),
        Span::styled("  Tasks: ", dim),
        Span::styled(tasks_done.to_string(), cyan),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Streak: ", dim),
        Span::styled(
            format!("{} (best {})", state.current_streak, state.longest_streak),
            grn,
        ),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::styled(" Time : ", dim),
        Span::styled(format!("{}m", sess_mins), cyan),
    ])));

    let mut list_state = ListState::default().with_offset(state.stats_scroll as usize);
    f.render_stateful_widget(List::new(items).block(block), area, &mut list_state);
}

// ---------------------------------------------------------------------------
// Help bar
// ---------------------------------------------------------------------------

fn render_help(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let dim         = Style::default().fg(Color::DarkGray);
    let key         = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
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

    f.render_widget(
        Paragraph::new(Text::from(line)).block(block).alignment(Alignment::Center),
        area,
    );
}
