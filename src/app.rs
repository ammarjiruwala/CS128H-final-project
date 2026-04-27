use crate::tasks::TaskQueue;
use crate::timer::{TimerCommand, TimerEvent, TimerHandle, SessionState, FOCUS_SECS};

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

    pub tasks: TaskQueue,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            timer: TimerHandle::start(),
            session: SessionState::Focus,
            remaining_secs: FOCUS_SECS,
            focus_sessions_completed: 0,
            total_focus_sessions: 0,
            skipped_sessions: 0,
            current_streak: 0,
            longest_streak: 0,
            tasks: TaskQueue::new(),
        }
    }

    /// Drain all pending timer events and update the local snapshot.
    pub fn process_timer_events(&mut self) {
        while let Ok(event) = self.timer.event_rx.try_recv() {
            match event {
                TimerEvent::Tick {
                    session,
                    remaining_secs,
                    focus_sessions_completed,
                    total_focus_sessions,
                    skipped_sessions,
                    current_streak,
                    longest_streak,
                } => {
                    self.session = session;
                    self.remaining_secs = remaining_secs;
                    self.focus_sessions_completed = focus_sessions_completed;
                    self.total_focus_sessions = total_focus_sessions;
                    self.skipped_sessions = skipped_sessions;
                    self.current_streak = current_streak;
                    self.longest_streak = longest_streak;
                }
                TimerEvent::SessionChanged(session) => {
                    self.session = session;
                }
            }
        }
    }

    /// Toggle between paused and running.
    pub fn toggle_pause(&self) {
        let cmd = if self.session.is_paused() {
            TimerCommand::Resume
        } else {
            TimerCommand::Pause
        };
        let _ = self.timer.cmd_tx.send(cmd);
    }

    /// Skip the current session (does not count toward completed stats).
    pub fn skip(&self) {
        let _ = self.timer.cmd_tx.send(TimerCommand::Skip);
    }

    /// Remove the last completed session from stats without changing the current session.
    pub fn undo_last(&self) {
        let _ = self.timer.cmd_tx.send(TimerCommand::UndoLast);
    }

    /// Signal the timer thread to exit.
    pub fn quit(&self) {
        let _ = self.timer.cmd_tx.send(TimerCommand::Quit);
    }

    /// Format remaining_secs as "MM:SS".
    pub fn time_display(&self) -> String {
        let mins = self.remaining_secs / 60;
        let secs = self.remaining_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Completion rate as a rounded percentage, or None if no sessions attempted yet.
    pub fn completion_rate(&self) -> Option<u32> {
        let total = self.total_focus_sessions + self.skipped_sessions;
        if total == 0 {
            None
        } else {
            Some(self.total_focus_sessions * 100 / total)
        }
    }
}
