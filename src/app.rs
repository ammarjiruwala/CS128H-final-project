use crate::timer::{TimerCommand, TimerEvent, TimerHandle};

/// Top-level application state. Owns the timer handle and reacts to timer events.
pub struct App {
    pub timer: TimerHandle,
}

impl App {
    pub fn new() -> Self {
        App {
            timer: TimerHandle::start(),
        }
    }

    /// Send a command to the timer thread (pause, resume, skip, quit).
    pub fn send(&self, cmd: TimerCommand) {
        let _ = self.timer.cmd_tx.send(cmd);
    }

    /// Poll for the latest timer event (non-blocking).
    pub fn poll_event(&self) -> Option<TimerEvent> {
        self.timer.event_rx.try_recv().ok()
    }
}
