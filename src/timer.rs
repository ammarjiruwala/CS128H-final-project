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
// Shared timer state
// ---------------------------------------------------------------------------

/// All mutable timer data protected by a single `Mutex`.
#[derive(Debug)]
pub struct TimerState {
    pub session: SessionState,
    /// Seconds remaining in the current session.
    pub remaining_secs: u64,
    /// How many Focus sessions completed in the current cycle (resets at long break).
    pub focus_sessions_completed: u32,
    /// Focus sessions that ran to zero naturally (excludes skipped).
    pub total_focus_sessions: u32,
    /// Focus sessions the user skipped before they expired.
    pub skipped_sessions: u32,
    /// Consecutive naturally-completed Focus sessions without a skip.
    pub current_streak: u32,
    /// Highest streak reached this run.
    pub longest_streak: u32,
}

impl TimerState {
    fn new() -> Self {
        TimerState {
            session: SessionState::Focus,
            remaining_secs: FOCUS_SECS,
            focus_sessions_completed: 0,
            total_focus_sessions: 0,
            skipped_sessions: 0,
            current_streak: 0,
            longest_streak: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Transition helpers
    // -----------------------------------------------------------------------

    /// Advance to the next session.
    ///
    /// `completed` should be `true` when the timer hit zero naturally, and
    /// `false` when the user pressed skip. Only natural completions count
    /// toward `total_focus_sessions` and the streak.
    pub fn advance(&mut self, completed: bool) {
        match &self.session {
            SessionState::Focus => {
                // Always advance the cycle counter (needed to trigger long break).
                self.focus_sessions_completed += 1;

                if completed {
                    self.total_focus_sessions += 1;
                    self.current_streak += 1;
                    if self.current_streak > self.longest_streak {
                        self.longest_streak = self.current_streak;
                    }
                } else {
                    self.skipped_sessions += 1;
                    self.current_streak = 0;
                }

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
                self.resume();
                self.advance(completed);
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

    /// Skip the current session — does not count as completed.
    pub fn skip(&mut self) {
        if self.session.is_paused() {
            self.resume();
        }
        self.advance(false);
    }

    /// Remove the last naturally-completed Focus session from stats.
    /// Does not change which session is currently active.
    pub fn undo_last_session(&mut self) {
        if self.total_focus_sessions > 0 {
            self.total_focus_sessions -= 1;
        }
        if self.current_streak > 0 {
            self.current_streak -= 1;
        }
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
    /// Remove the last completed session from stats without changing session state.
    UndoLast,
    Quit,
}

/// Events sent from the timer thread → UI thread.
#[derive(Debug, Clone)]
pub enum TimerEvent {
    /// Fired every second with a full state snapshot.
    Tick {
        session: SessionState,
        remaining_secs: u64,
        focus_sessions_completed: u32,
        total_focus_sessions: u32,
        skipped_sessions: u32,
        current_streak: u32,
        longest_streak: u32,
    },
    /// The active session changed (natural expiry or skip).
    SessionChanged(SessionState),
}

// ---------------------------------------------------------------------------
// Timer handle
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

fn timer_loop(
    state: Arc<Mutex<TimerState>>,
    cmd_rx: mpsc::Receiver<TimerCommand>,
    event_tx: mpsc::Sender<TimerEvent>,
) {
    let tick_interval = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        // Drain all pending commands before ticking.
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
                Ok(TimerCommand::UndoLast) => {
                    state.lock().unwrap().undo_last_session();
                }
                Ok(TimerCommand::Quit) | Err(mpsc::TryRecvError::Disconnected) => return,
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }

        let elapsed = last_tick.elapsed();
        if elapsed < tick_interval {
            thread::sleep(tick_interval - elapsed);
        }
        last_tick = Instant::now();

        // Decrement the clock (only if running).
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

        // Natural expiry — advance with completed = true.
        if session_expired {
            let new_session = {
                let mut s = state.lock().unwrap();
                s.advance(true);
                s.session.clone()
            };
            let _ = event_tx.send(TimerEvent::SessionChanged(new_session));
        }

        // Always send a tick snapshot.
        let snapshot = {
            let s = state.lock().unwrap();
            TimerEvent::Tick {
                session: s.session.clone(),
                remaining_secs: s.remaining_secs,
                focus_sessions_completed: s.focus_sessions_completed,
                total_focus_sessions: s.total_focus_sessions,
                skipped_sessions: s.skipped_sessions,
                current_streak: s.current_streak,
                longest_streak: s.longest_streak,
            }
        };
        if event_tx.send(snapshot).is_err() {
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

    // --- Transitions ---

    #[test]
    fn focus_advances_to_short_break_for_first_three_sessions() {
        for i in 1..SESSIONS_BEFORE_LONG_BREAK {
            let mut s = fresh();
            s.focus_sessions_completed = i - 1;
            s.advance(true);
            assert_eq!(s.session, SessionState::ShortBreak, "session {i}");
            assert_eq!(s.remaining_secs, SHORT_BREAK_SECS);
        }
    }

    #[test]
    fn fourth_focus_session_triggers_long_break() {
        let mut s = fresh();
        s.focus_sessions_completed = SESSIONS_BEFORE_LONG_BREAK - 1;
        s.advance(true);
        assert_eq!(s.session, SessionState::LongBreak);
        assert_eq!(s.remaining_secs, LONG_BREAK_SECS);
        assert_eq!(s.focus_sessions_completed, 0);
    }

    #[test]
    fn short_break_advances_to_focus() {
        let mut s = fresh();
        s.session = SessionState::ShortBreak;
        s.remaining_secs = SHORT_BREAK_SECS;
        s.advance(true);
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.remaining_secs, FOCUS_SECS);
    }

    #[test]
    fn long_break_advances_to_focus() {
        let mut s = fresh();
        s.session = SessionState::LongBreak;
        s.remaining_secs = LONG_BREAK_SECS;
        s.advance(true);
        assert_eq!(s.session, SessionState::Focus);
        assert_eq!(s.remaining_secs, FOCUS_SECS);
    }

    #[test]
    fn full_four_session_cycle() {
        let mut s = fresh();
        for _ in 0..3 {
            assert_eq!(s.session, SessionState::Focus);
            s.advance(true);
            assert_eq!(s.session, SessionState::ShortBreak);
            s.advance(true);
        }
        assert_eq!(s.session, SessionState::Focus);
        s.advance(true);
        assert_eq!(s.session, SessionState::LongBreak);
        s.advance(true);
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
        s.pause();
        assert_eq!(s.session, SessionState::Paused(Box::new(SessionState::Focus)));
    }

    #[test]
    fn resume_on_running_timer_is_noop() {
        let mut s = fresh();
        s.resume();
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
        s.skip();
        assert_eq!(s.session, SessionState::ShortBreak);
    }

    // --- Skip does NOT count as a completed session ---

    #[test]
    fn skip_does_not_increment_total_focus_sessions() {
        let mut s = fresh();
        s.skip();
        assert_eq!(s.total_focus_sessions, 0);
    }

    #[test]
    fn skip_increments_skipped_sessions() {
        let mut s = fresh();
        s.skip();
        assert_eq!(s.skipped_sessions, 1);
    }

    #[test]
    fn natural_completion_does_not_increment_skipped_sessions() {
        let mut s = fresh();
        s.advance(true);
        assert_eq!(s.skipped_sessions, 0);
    }

    // --- Streak ---

    #[test]
    fn streak_increments_on_natural_completion() {
        let mut s = fresh();
        s.advance(true);
        assert_eq!(s.current_streak, 1);
        s.advance(true); // break → focus
        s.advance(true); // second focus
        assert_eq!(s.current_streak, 2);
    }

    #[test]
    fn streak_resets_to_zero_on_skip() {
        let mut s = fresh();
        s.advance(true); // focus 1 done
        s.advance(true); // break done
        assert_eq!(s.current_streak, 1);
        s.skip(); // skip focus 2
        assert_eq!(s.current_streak, 0);
    }

    #[test]
    fn longest_streak_tracks_best_run() {
        let mut s = fresh();
        // Complete 2 sessions then skip — longest should be 2.
        s.advance(true); s.advance(true); // focus 1, break
        s.advance(true); s.advance(true); // focus 2, break
        assert_eq!(s.longest_streak, 2);
        s.skip();
        assert_eq!(s.current_streak, 0);
        assert_eq!(s.longest_streak, 2); // best still held
    }

    // --- Undo ---

    #[test]
    fn undo_decrements_total_focus_sessions() {
        let mut s = fresh();
        s.advance(true);
        assert_eq!(s.total_focus_sessions, 1);
        s.undo_last_session();
        assert_eq!(s.total_focus_sessions, 0);
    }

    #[test]
    fn undo_decrements_current_streak() {
        let mut s = fresh();
        s.advance(true);
        assert_eq!(s.current_streak, 1);
        s.undo_last_session();
        assert_eq!(s.current_streak, 0);
    }

    #[test]
    fn undo_on_zero_is_noop() {
        let mut s = fresh();
        s.undo_last_session(); // should not underflow
        assert_eq!(s.total_focus_sessions, 0);
        assert_eq!(s.current_streak, 0);
    }

    #[test]
    fn undo_does_not_change_session_state() {
        let mut s = fresh();
        s.advance(true); // now in ShortBreak
        s.undo_last_session();
        assert_eq!(s.session, SessionState::ShortBreak); // still in break
    }

    // --- Counters ---

    #[test]
    fn total_focus_sessions_increments_correctly() {
        let mut s = fresh();
        for _ in 0..SESSIONS_BEFORE_LONG_BREAK {
            s.advance(true);
            s.advance(true);
        }
        assert_eq!(s.total_focus_sessions, SESSIONS_BEFORE_LONG_BREAK);
    }

    // --- Duration helpers ---

    #[test]
    fn duration_secs_returns_correct_values() {
        assert_eq!(SessionState::Focus.duration_secs(), FOCUS_SECS);
        assert_eq!(SessionState::ShortBreak.duration_secs(), SHORT_BREAK_SECS);
        assert_eq!(SessionState::LongBreak.duration_secs(), LONG_BREAK_SECS);
        assert_eq!(
            SessionState::Paused(Box::new(SessionState::Focus)).duration_secs(),
            FOCUS_SECS
        );
    }
}
