//! Timer & Session State Machine
//!
//! Owns the Pomodoro cycle logic:
//!   Focus (25 min) → ShortBreak (5 min), repeated 4 times,
//!   then Focus → LongBreak (15 min), then cycle repeats.
//!
//! A background thread runs the countdown and sends [`TimerEvent`]s to the UI
//! thread via an `mpsc` channel. The UI thread sends [`TimerCommand`]s back.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc;

// ---------------------------------------------------------------------------
// Configurable durations
// ---------------------------------------------------------------------------

pub const FOCUS_SECS: u64 = 25 * 60;
pub const SHORT_BREAK_SECS: u64 = 5 * 60;
pub const LONG_BREAK_SECS: u64 = 15 * 60;
/// Number of focus sessions before a long break.
pub const SESSIONS_BEFORE_LONG_BREAK: u32 = 4;

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

/// The phase the Pomodoro timer is currently in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Focus,
    ShortBreak,
    LongBreak,
    /// Timer is paused; inner value is the phase that was active when paused.
    Paused(Box<SessionState>),
}

impl SessionState {
    /// Returns the total duration (in seconds) for this state.
    /// Panics if called on `Paused` — query the inner state instead.
    pub fn duration_secs(&self) -> u64 {
        match self {
            SessionState::Focus => FOCUS_SECS,
            SessionState::ShortBreak => SHORT_BREAK_SECS,
            SessionState::LongBreak => LONG_BREAK_SECS,
            SessionState::Paused(inner) => inner.duration_secs(),
        }
    }

    /// Returns `true` if the timer is currently paused.
    pub fn is_paused(&self) -> bool {
        matches!(self, SessionState::Paused(_))
    }
}

// ---------------------------------------------------------------------------
// Shared timer state (timer thread + UI thread both read this)
// ---------------------------------------------------------------------------

/// All mutable timer data protected by a single `Mutex`.
#[derive(Debug)]
pub struct TimerState {
    pub session: SessionState,
    /// Seconds remaining in the current session.
    pub remaining_secs: u64,
    /// How many Focus sessions have been completed in the current cycle.
    pub focus_sessions_completed: u32,
    /// Total Focus sessions completed across all cycles (lifetime counter).
    pub total_focus_sessions: u32,
}

impl TimerState {
    fn new() -> Self {
        TimerState {
            session: SessionState::Focus,
            remaining_secs: FOCUS_SECS,
            focus_sessions_completed: 0,
            total_focus_sessions: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Transition helpers
    // -----------------------------------------------------------------------

    /// Advance to the next session after the current one expires.
    /// Updates counters and resets `remaining_secs`.
    pub fn advance(&mut self) {
        match &self.session {
            SessionState::Focus => {
                self.focus_sessions_completed += 1;
                self.total_focus_sessions += 1;
                if self.focus_sessions_completed >= SESSIONS_BEFORE_LONG_BREAK {
                    self.focus_sessions_completed = 0;
                    self.session = SessionState::LongBreak;
                    self.remaining_secs = LONG_BREAK_SECS;
                } else {
                    self.session = SessionState::ShortBreak;
                    self.remaining_secs = SHORT_BREAK_SECS;
                }
            }
            SessionState::ShortBreak | SessionState::LongBreak => {
                self.session = SessionState::Focus;
                self.remaining_secs = FOCUS_SECS;
            }
            SessionState::Paused(_) => {
                // Advance from inside a paused state: resume then advance.
                self.resume();
                self.advance();
            }
        }
    }

    /// Pause the timer, remembering which session was active.
    /// No-op if already paused.
    pub fn pause(&mut self) {
        if !self.session.is_paused() {
            let current = self.session.clone();
            self.session = SessionState::Paused(Box::new(current));
        }
    }

    /// Resume a paused timer. No-op if not paused.
    pub fn resume(&mut self) {
        if let SessionState::Paused(inner) = &self.session {
            self.session = *inner.clone();
        }
    }

    /// Skip the current session and move to the next one immediately.
    pub fn skip(&mut self) {
        // If paused, resume first so `advance` sees the real session type.
        if self.session.is_paused() {
            self.resume();
        }
        self.advance();
    }
}

// ---------------------------------------------------------------------------
// Channel message types
// ---------------------------------------------------------------------------

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
    /// Fired every second with the current snapshot.
    Tick {
        session: SessionState,
        remaining_secs: u64,
        focus_sessions_completed: u32,
        total_focus_sessions: u32,
    },
    /// The current session expired and we transitioned to a new one.
    SessionChanged(SessionState),
}

// ---------------------------------------------------------------------------
// Timer handle (returned to the caller / UI thread)
// ---------------------------------------------------------------------------

/// Returned by [`TimerHandle::start`]. The UI thread holds this to communicate
/// with the background timer thread.
pub struct TimerHandle {
    pub cmd_tx: mpsc::Sender<TimerCommand>,
    pub event_rx: mpsc::Receiver<TimerEvent>,
    pub state: Arc<Mutex<TimerState>>,
}

impl TimerHandle {
    /// Spawn the background timer thread and return a handle to it.
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TimerCommand>();
        let (event_tx, event_rx) = mpsc::channel::<TimerEvent>();

        let state = Arc::new(Mutex::new(TimerState::new()));
        let state_clone = Arc::clone(&state);

        thread::spawn(move || {
            timer_loop(state_clone, cmd_rx, event_tx);
        });

        TimerHandle { cmd_tx, event_rx, state }
    }
}

// ---------------------------------------------------------------------------
// Background timer loop
// ---------------------------------------------------------------------------

/// Runs on its own thread. Ticks every second and processes commands.
fn timer_loop(
    state: Arc<Mutex<TimerState>>,
    cmd_rx: mpsc::Receiver<TimerCommand>,
    event_tx: mpsc::Sender<TimerEvent>,
) {
    let tick_interval = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        // Drain all pending commands before doing anything else.
        loop {
            match cmd_rx.try_recv() {
                Ok(TimerCommand::Pause) => {
                    state.lock().unwrap().pause();
                }
                Ok(TimerCommand::Resume) => {
                    state.lock().unwrap().resume();
                }
                Ok(TimerCommand::Skip) => {
                    let new_session = {
                        let mut s = state.lock().unwrap();
                        s.skip();
                        s.session.clone()
                    };
                    let _ = event_tx.send(TimerEvent::SessionChanged(new_session));
                }
                Ok(TimerCommand::Quit) | Err(mpsc::TryRecvError::Disconnected) => return,
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }

        // Sleep the remainder of the 1-second tick interval.
        let elapsed = last_tick.elapsed();
        if elapsed < tick_interval {
            thread::sleep(tick_interval - elapsed);
        }
        last_tick = Instant::now();

        // Tick: decrement remaining_secs (only if not paused).
        let session_expired = {
            let mut s = state.lock().unwrap();
            if !s.session.is_paused() {
                if s.remaining_secs > 0 {
                    s.remaining_secs -= 1;
                }
                s.remaining_secs == 0
            } else {
                false
            }
        };

        // If this tick caused the session to expire, advance to the next one.
        if session_expired {
            let new_session = {
                let mut s = state.lock().unwrap();
                s.advance();
                s.session.clone()
            };
            let _ = event_tx.send(TimerEvent::SessionChanged(new_session));
        }

        // Send a tick snapshot regardless.
        let snapshot = {
            let s = state.lock().unwrap();
            TimerEvent::Tick {
                session: s.session.clone(),
                remaining_secs: s.remaining_secs,
                focus_sessions_completed: s.focus_sessions_completed,
                total_focus_sessions: s.total_focus_sessions,
            }
        };
        if event_tx.send(snapshot).is_err() {
            // UI thread has dropped the receiver; exit cleanly.
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a fresh TimerState for testing without spawning threads.
    fn fresh() -> TimerState {
        TimerState::new()
    }

    // --- Initial state ---

    #[test]
    fn initial_state_is_focus() {
        let s = fresh();
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.remaining_secs, FOCUS_SECS);
        assert_eq!(s.focus_sessions_completed, 0);
    }

    // --- Focus → ShortBreak (first 3 times) ---

    #[test]
    fn focus_advances_to_short_break_for_first_three_sessions() {
        for i in 1..SESSIONS_BEFORE_LONG_BREAK {
            let mut s = fresh();
            // Simulate completing `i` focus sessions without hitting the long break.
            s.focus_sessions_completed = i - 1;
            s.advance(); // complete one more focus session
            assert_eq!(
                s.session,
                SessionState::ShortBreak,
                "session {i}: expected ShortBreak"
            );
            assert_eq!(s.remaining_secs, SHORT_BREAK_SECS);
        }
    }

    // --- Focus → LongBreak after 4th session ---

    #[test]
    fn fourth_focus_session_triggers_long_break() {
        let mut s = fresh();
        s.focus_sessions_completed = SESSIONS_BEFORE_LONG_BREAK - 1;
        s.advance();
        assert_eq!(s.session, SessionState::LongBreak);
        assert_eq!(s.remaining_secs, LONG_BREAK_SECS);
        assert_eq!(s.focus_sessions_completed, 0, "counter should reset after long break");
    }

    // --- Break → Focus ---

    #[test]
    fn short_break_advances_to_focus() {
        let mut s = fresh();
        s.session = SessionState::ShortBreak;
        s.remaining_secs = SHORT_BREAK_SECS;
        s.advance();
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.remaining_secs, FOCUS_SECS);
    }

    #[test]
    fn long_break_advances_to_focus() {
        let mut s = fresh();
        s.session = SessionState::LongBreak;
        s.remaining_secs = LONG_BREAK_SECS;
        s.advance();
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.remaining_secs, FOCUS_SECS);
    }

    // --- Full 4-session cycle ---

    #[test]
    fn full_four_session_cycle() {
        let mut s = fresh();
        // 3 focus → short break cycles
        for _ in 0..3 {
            assert_eq!(s.session, SessionState::Focus);
            s.advance(); // focus completes
            assert_eq!(s.session, SessionState::ShortBreak);
            s.advance(); // short break completes
        }
        // 4th focus → long break
        assert_eq!(s.session, SessionState::Focus);
        s.advance();
        assert_eq!(s.session, SessionState::LongBreak);
        s.advance(); // long break completes
        // Back to focus, cycle counter reset
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.focus_sessions_completed, 0);
    }

    // --- Pause / Resume ---

    #[test]
    fn pause_marks_state_as_paused() {
        let mut s = fresh();
        s.pause();
        assert!(s.session.is_paused());
    }

    #[test]
    fn pause_preserves_inner_session_type() {
        let mut s = fresh();
        s.pause();
        assert_eq!(s.session, SessionState::Paused(Box::new(SessionState::Focus)));
    }

    #[test]
    fn resume_restores_session_type() {
        let mut s = fresh();
        s.pause();
        s.resume();
        assert_eq!(s.session, SessionState::Focus);
    }

    #[test]
    fn double_pause_is_noop() {
        let mut s = fresh();
        s.pause();
        s.pause(); // second pause should not double-wrap
        assert_eq!(s.session, SessionState::Paused(Box::new(SessionState::Focus)));
    }

    #[test]
    fn resume_on_running_timer_is_noop() {
        let mut s = fresh();
        s.resume(); // should not panic or change state
        assert_eq!(s.session, SessionState::Focus);
    }

    // --- Skip ---

    #[test]
    fn skip_from_focus_goes_to_short_break() {
        let mut s = fresh();
        s.skip();
        assert_eq!(s.session, SessionState::ShortBreak);
    }

    #[test]
    fn skip_while_paused_advances_correctly() {
        let mut s = fresh();
        s.pause();
        s.skip(); // should unpause then advance
        assert_eq!(s.session, SessionState::ShortBreak);
    }

    // --- Total session counter ---

    #[test]
    fn total_focus_sessions_increments_correctly() {
        let mut s = fresh();
        // Complete a full 4-session cycle
        for _ in 0..SESSIONS_BEFORE_LONG_BREAK {
            s.advance(); // focus → break
            s.advance(); // break → focus (or break → focus on 4th)
        }
        assert_eq!(s.total_focus_sessions, SESSIONS_BEFORE_LONG_BREAK);
    }

    // --- Duration helpers ---

    #[test]
    fn duration_secs_returns_correct_values() {
        assert_eq!(SessionState::Focus.duration_secs(), FOCUS_SECS);
        assert_eq!(SessionState::ShortBreak.duration_secs(), SHORT_BREAK_SECS);
        assert_eq!(SessionState::LongBreak.duration_secs(), LONG_BREAK_SECS);
        // Paused delegates to inner
        assert_eq!(
            SessionState::Paused(Box::new(SessionState::Focus)).duration_secs(),
            FOCUS_SECS
        );
    }
}
