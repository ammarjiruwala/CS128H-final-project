# RUN.md — Terminal Pomodoro

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024, tested on rustc 1.93+)
- A terminal emulator that supports 256 colours (most modern terminals do: iTerm2, Windows Terminal, GNOME Terminal, etc.)

## Steps

```bash
git clone https://github.com/ammarjiruwala/CS128H-final-project.git
cd CS128H-final-project
cargo run
```

That's it. Cargo will download all dependencies and build the project on first run.

## Keybindings

| Key | Action |
|-----|--------|
| `Space` | Pause / Resume timer |
| `s` | Skip current session |
| `u` | Undo last completed session |
| `↑` / `↓` | Navigate task list |
| `a` | Add a new task |
| `e` | Edit selected task name |
| `Enter` | Toggle selected task done / not done |
| `d` | Delete selected task |
| `[` / `]` | Scroll stats panel up / down |
| `q` | Quit and save |

## View session history

```bash
cargo run -- --stats
```

Prints an all-time summary with a GitHub-style activity heatmap and your last 10 sessions. Data persists in `pomodoro_tasks.json` and `pomodoro_history.json` in the directory where you run the app.
