# RUN.md — Terminal Pomodoro
## Steps
```bash
git clone https://github.com/ammarjiruwala/CS128H-final-project.git
cd CS128H-final-project
cargo run
```

## Keybindings for the Terminal Pomodoro

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