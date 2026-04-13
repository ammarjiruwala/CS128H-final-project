//! Timer & Session State Machine — stub (Ammar's module)
//!
//! Full implementation lives on the `ammar/timer` branch.
//! Type definitions are here so the rest of the project compiles.

use std::sync::{Arc, Mutex};
use std::sync::mpsc;

/// The phase the Pomodoro timer is currently in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Focus,
    ShortBreak,
    LongBreak,
    /// Timer is paused; inner value is the phase that was active when paused.
    Paused(Box<SessionState>),
}

/// Commands sent from the UI thread → timer thread.
#[derive(Debug)]
pub enum TimerCommand {
    Pause,
    Resume,
    Skip,
    Quit,
}

/// Events sent from the timer thread → UI thread.
#[derive(Debug, Clone)]
pub enum TimerEvent {
    Tick {
        session: SessionState,
        remaining_secs: u64,
        focus_sessions_completed: u32,
        total_focus_sessions: u32,
    },
    SessionChanged(SessionState),
}

/// All mutable timer data protected by a single `Mutex`.
#[derive(Debug)]
pub struct TimerState {
    pub session: SessionState,
    pub remaining_secs: u64,
    pub focus_sessions_completed: u32,
    pub total_focus_sessions: u32,
}

/// Returned by [`TimerHandle::start`]. The UI thread holds this to communicate
/// with the background timer thread.
pub struct TimerHandle {
    pub cmd_tx: mpsc::Sender<TimerCommand>,
    pub event_rx: mpsc::Receiver<TimerEvent>,
    pub state: Arc<Mutex<TimerState>>,
}

impl TimerHandle {
    pub fn start() -> Self {
        todo!("Ammar: implement timer thread")
    }
}
