<h1 align="center">Fokus</h1>

Learning Rust by building a [Pomodoro](https://en.wikipedia.org/wiki/Pomodoro_Technique) timer.

Critiques welcome.

---

## Installation

```
cargo install fokus
```

## Usage

```
A Simple Pomodoro TUI Built With Rust

Usage: fokus [OPTIONS]

Options:
  -w, --working-time <WORKING_TIME>                            [default: 25]
  -b, --break-time <BREAK_TIME>                                [default: 5]
  -l, --long-break-time <LONG_BREAK_TIME>                      [default: 15]
  -s, --sessions-until-break-time <SESSIONS_UNTIL_BREAK_TIME>  [default: 2]
  -h, --help                                                   Print help
  -V, --version                                                Print version
```

## TODO

- [ ] Notification clicking takes you to window
- [ ] Notification actions
  - Need to figure out how to async wait notification but still run the main thread
- [x] Fix skip session panicking
- [ ] Fix session counter not working every nth time
