# Terminal Pomodoro 🍅

## Group Name
Terminal Pomodoro

## Group Members
| Name | NetID |
|---|---|
| Nikhil Kulkarni | nhk5 |
| Ammar Jiruwala | ammarj2 |
| Tanay Desai | tndesai2 |

---

## Project Introduction

Terminal Pomodoro is a feature-rich, interactive command-line productivity tool built in Rust. It implements the [Pomodoro Technique](https://en.wikipedia.org/wiki/Pomodoro_Technique) — a time management method where work is broken into focused intervals (typically 25 minutes) separated by short breaks (5 minutes), with a longer break after every 4 completed sessions.

Beyond a simple countdown timer, Terminal Pomodoro layers on full **task management**: users can maintain a queue of tasks, associate completed pomodoro sessions with specific tasks, and review historical productivity stats across sessions. All data persists locally between runs, making it a genuinely useful daily tool.

**Goals and Objectives:**
- Build a polished, responsive terminal UI using `ratatui` with live-updating timer display, task list, and session progress
- Implement a robust session state machine managing the full work → short break → long break cycle with pause/resume support
- Provide persistent storage of tasks and session history, with summary statistics (total focus time, tasks completed, daily streaks)
- Demonstrate clean Rust design through well-defined structs, enums, and separation of concerns across modules

We chose this project because it is immediately useful to all three of us as students, it maps naturally to a clean three-way division of work, and it touches a wide range of Rust concepts covered in class (and beyond).

---

## Technical Overview

The project is structured around three major components, each owned primarily by one team member but integrated throughout.

### 1. Terminal UI (`ratatui` + `crossterm`)
The UI layer renders the full interactive interface in the terminal: a live countdown clock, the current session type (Focus / Short Break / Long Break), a scrollable task queue with status indicators, a session progress tracker (e.g. 🍅🍅⬜⬜ out of 4), and a stats summary panel. Keyboard bindings allow the user to add/complete/delete tasks, pause and resume the timer, and skip sessions. The UI redraws on each tick and on any keypress using `crossterm`'s event polling.

### 2. Timer & Session State Machine
The core logic module manages the Pomodoro cycle as an explicit state machine using a Rust `enum` (`SessionState::Focus`, `SessionState::ShortBreak`, `SessionState::LongBreak`, `SessionState::Paused`). A background thread runs the countdown and sends tick events to the UI via an `mpsc` channel. The state machine handles transitions automatically (e.g. after 4 focus sessions → long break) and exposes pause/resume/skip controls. Session durations will be configurable via a `config.toml` file.

### 3. Persistence & Statistics (`serde_json` / SQLite)
All tasks and completed session records are serialized and saved locally (JSON or a lightweight SQLite database via `rusqlite`). On startup the app loads existing state seamlessly. The stats module computes summaries: total pomodoros completed, time spent per task, daily focus streaks, and a simple historical log. A `--stats` CLI flag will print a summary report without launching the full UI.

### Module Breakdown
```
src/
├── main.rs          # Entry point, CLI argument parsing
├── app.rs           # Top-level app state, event loop
├── timer.rs         # Session state machine, countdown thread
├── tasks.rs         # Task struct, task queue logic
├── ui.rs            # ratatui rendering logic
├── persistence.rs   # Save/load tasks and session history
└── stats.rs         # Statistics computation and formatting
```

### Checkpoint Goals

**Checkpoint 1:**
- Project structure scaffolded with all modules stubbed out
- Basic `ratatui` UI renders in terminal (static layout, no live updates yet)
- Timer state machine implemented and unit tested (transitions, pause/resume)
- Task struct and queue operations (add, complete, delete) implemented

**Checkpoint 2:**
- Live timer integrated into UI (real-time countdown display)
- Task queue fully interactive via keyboard bindings
- Persistence layer working (tasks and sessions save/load correctly across runs)
- Basic stats summary accessible via `--stats` flag

---

## Possible Challenges

- **Concurrency between timer thread and UI thread:** The countdown runs on a separate thread and must communicate with the UI event loop without data races. We will use `mpsc` channels and potentially `Arc<Mutex<T>>` to share state safely, which requires careful design upfront.
- **`ratatui` learning curve:** None of us have used `ratatui` before. Getting comfortable with its layout system (constraints, widgets, stateful lists) will take some initial ramp-up time.
- **Terminal compatibility:** Terminal rendering can behave differently across operating systems and terminal emulators. We will need to test on multiple machines and handle edge cases in `crossterm` event handling.
- **Configurable durations:** Allowing users to customize session lengths via a config file introduces parsing and validation logic that needs to fail gracefully.
- **Keeping the UI responsive while the timer runs:** Ensuring keyboard input is never blocked by the timer thread will require careful use of non-blocking event polling.

---

## References

- [The Pomodoro Technique](https://francescocirillo.com/products/the-pomodoro-technique) — Francesco Cirillo, original method
- [`ratatui` documentation and examples](https://ratatui.rs/)
- [`crossterm` documentation](https://docs.rs/crossterm/latest/crossterm/)
- [`serde` / `serde_json` documentation](https://serde.rs/)
- [`chrono` documentation](https://docs.rs/chrono/latest/chrono/)
- [Rust `mpsc` channels](https://doc.rust-lang.org/std/sync/mpsc/)
- [Awesome Rust — TUI section](https://github.com/rust-unofficial/awesome-rust#text-user-interface) — for inspiration and crate discovery
