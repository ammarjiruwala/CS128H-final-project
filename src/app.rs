use crate::persistence;
use crate::stats::SessionRecord;
use crate::tasks::TaskQueue;
use crate::timer::{TimerCommand, TimerEvent, TimerHandle, SessionState, FOCUS_SECS};

pub enum InputMode {
    Normal,
    AddingTask(String),
    /// Editing the task at `idx`; `buffer` holds the current typed text.
    EditingTask { idx: usize, buffer: String },
}

/// Top-level application state shared between the event loop and the UI renderer.
pub struct AppState {
    pub timer: TimerHandle,

    // Mirrored from the latest TimerEvent::Tick.
    pub session: SessionState,
    pub remaining_secs: u64,
    pub focus_sessions_completed: u32,
    pub total_focus_sessions: u32,
    pub skipped_sessions: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    /// Actual seconds spent in Focus across all sessions (including partial skips).
    pub total_focus_secs: u64,

    pub tasks: TaskQueue,
    pub selected_task: Option<usize>,
    pub input_mode: InputMode,

    /// How many rows the stats panel has been scrolled down.
    pub stats_scroll: u16,

    pub session_history: Vec<SessionRecord>,
    skip_pending: bool,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            timer: TimerHandle::start(),
            // TimerHandle::start() sends an immediate Pause, so we mirror that here.
            session: SessionState::Paused(Box::new(SessionState::Focus)),
            remaining_secs: FOCUS_SECS,
            focus_sessions_completed: 0,
            total_focus_sessions: 0,
            skipped_sessions: 0,
            current_streak: 0,
            longest_streak: 0,
            total_focus_secs: 0,
            tasks: TaskQueue::new(),
            selected_task: None,
            input_mode: InputMode::Normal,
            stats_scroll: 0,
            session_history: Vec::new(),
            skip_pending: false,
        }
    }

    // -----------------------------------------------------------------------
    // Input mode helpers
    // -----------------------------------------------------------------------

    pub fn is_adding(&self) -> bool {
        matches!(self.input_mode, InputMode::AddingTask(_))
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.input_mode, InputMode::EditingTask { .. })
    }

    pub fn is_typing(&self) -> bool {
        self.is_adding() || self.is_editing()
    }

    pub fn input_buf(&self) -> &str {
        match &self.input_mode {
            InputMode::AddingTask(s)              => s,
            InputMode::EditingTask { buffer, .. } => buffer,
            InputMode::Normal                     => "",
        }
    }

    pub fn input_push(&mut self, c: char) {
        match &mut self.input_mode {
            InputMode::AddingTask(s)              => s.push(c),
            InputMode::EditingTask { buffer, .. } => buffer.push(c),
            InputMode::Normal                     => {}
        }
    }

    pub fn input_pop(&mut self) {
        match &mut self.input_mode {
            InputMode::AddingTask(s)              => { s.pop(); }
            InputMode::EditingTask { buffer, .. } => { buffer.pop(); }
            InputMode::Normal                     => {}
        }
    }

    // -----------------------------------------------------------------------
    // Task navigation
    // -----------------------------------------------------------------------

    pub fn select_next(&mut self) {
        let len = self.tasks.tasks().len();
        if len == 0 { self.selected_task = None; return; }
        self.selected_task = Some(match self.selected_task {
            None    => 0,
            Some(i) => (i + 1).min(len - 1),
        });
    }

    pub fn select_prev(&mut self) {
        let len = self.tasks.tasks().len();
        if len == 0 { self.selected_task = None; return; }
        self.selected_task = Some(match self.selected_task {
            None    => 0,
            Some(i) => i.saturating_sub(1),
        });
    }

    // -----------------------------------------------------------------------
    // Task actions
    // -----------------------------------------------------------------------

    /// Toggle the selected task between Todo and Done (pressing Enter twice undoes a completion).
    pub fn toggle_selected(&mut self) {
        if let Some(idx) = self.selected_task {
            if let Some(task) = self.tasks.tasks().get(idx) {
                let id = task.id;
                self.tasks.toggle_task(id);
            }
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_task {
            if let Some(task) = self.tasks.tasks().get(idx) {
                let id = task.id;
                self.tasks.delete_task(id);
                let len = self.tasks.tasks().len();
                self.selected_task = if len == 0 { None } else { Some(idx.min(len - 1)) };
            }
        }
    }

    pub fn start_add_task(&mut self) {
        self.input_mode = InputMode::AddingTask(String::new());
    }

    pub fn confirm_add_task(&mut self) {
        let title = match &self.input_mode {
            InputMode::AddingTask(s) => s.trim().to_string(),
            _                        => String::new(),
        };
        if !title.is_empty() {
            self.tasks.add_task(&title);
            self.selected_task = Some(self.tasks.tasks().len() - 1);
        }
        self.input_mode = InputMode::Normal;
    }

    /// Begin editing the currently selected task name, pre-filling the buffer.
    pub fn start_edit_task(&mut self) {
        if let Some(idx) = self.selected_task {
            if let Some(task) = self.tasks.tasks().get(idx) {
                let current = task.title.clone();
                self.input_mode = InputMode::EditingTask { idx, buffer: current };
            }
        }
    }

    pub fn confirm_edit_task(&mut self) {
        if let InputMode::EditingTask { idx, buffer } = &self.input_mode {
            let idx = *idx;
            let new_title = buffer.trim().to_string();
            if !new_title.is_empty() {
                if let Some(task) = self.tasks.tasks().get(idx) {
                    let id = task.id;
                    self.tasks.rename_task(id, &new_title);
                }
            }
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    // -----------------------------------------------------------------------
    // Stats scroll
    // -----------------------------------------------------------------------

    pub fn scroll_stats_down(&mut self) {
        self.stats_scroll = self.stats_scroll.saturating_add(1);
    }

    pub fn scroll_stats_up(&mut self) {
        self.stats_scroll = self.stats_scroll.saturating_sub(1);
    }

    // -----------------------------------------------------------------------
    // Timer events
    // -----------------------------------------------------------------------

    /// Drain all pending timer events, update the snapshot, and record sessions.
    pub fn process_timer_events(&mut self) {
        while let Ok(event) = self.timer.event_rx.try_recv() {
            match event {
                TimerEvent::Tick {
                    session, remaining_secs, focus_sessions_completed,
                    total_focus_sessions, skipped_sessions, current_streak,
                    longest_streak, total_focus_secs,
                } => {
                    self.session = session;
                    self.remaining_secs = remaining_secs;
                    self.focus_sessions_completed = focus_sessions_completed;
                    self.total_focus_sessions = total_focus_sessions;
                    self.skipped_sessions = skipped_sessions;
                    self.current_streak = current_streak;
                    self.longest_streak = longest_streak;
                    self.total_focus_secs = total_focus_secs;
                }
                TimerEvent::SessionChanged(new_session) => {
                    let was_skip = self.skip_pending;
                    self.skip_pending = false;

                    // Record the session that just ended.
                    let (session_type, elapsed_secs) = match &self.session {
                        SessionState::Focus => {
                            let elapsed = if was_skip {
                                FOCUS_SECS.saturating_sub(self.remaining_secs)
                            } else {
                                FOCUS_SECS
                            };
                            (Some("Focus"), elapsed)
                        }
                        SessionState::ShortBreak => (Some("ShortBreak"), 0),
                        SessionState::LongBreak  => (Some("LongBreak"), 0),
                        SessionState::Paused(_)  => (None, 0),
                    };
                    if let Some(st) = session_type {
                        self.session_history.push(
                            SessionRecord::new(st, !was_skip, elapsed_secs)
                        );
                    }
                    self.session = new_session;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Timer commands
    // -----------------------------------------------------------------------

    pub fn toggle_pause(&self) {
        let cmd = if self.session.is_paused() { TimerCommand::Resume } else { TimerCommand::Pause };
        let _ = self.timer.cmd_tx.send(cmd);
    }

    pub fn skip(&mut self) {
        self.skip_pending = true;
        let _ = self.timer.cmd_tx.send(TimerCommand::Skip);
    }

    pub fn undo_last(&self) {
        let _ = self.timer.cmd_tx.send(TimerCommand::UndoLast);
    }

    pub fn quit(&self) {
        let _ = self.timer.cmd_tx.send(TimerCommand::Quit);
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    pub fn save(&self) -> Result<(), String> {
        persistence::save_tasks(self.tasks.tasks())?;
        persistence::save_history(&self.session_history)?;
        Ok(())
    }

    pub fn time_display(&self) -> String {
        format!("{:02}:{:02}", self.remaining_secs / 60, self.remaining_secs % 60)
    }

    pub fn completion_rate(&self) -> Option<u32> {
        let total = self.total_focus_sessions + self.skipped_sessions;
        if total == 0 { None } else { Some(self.total_focus_sessions * 100 / total) }
    }
}
