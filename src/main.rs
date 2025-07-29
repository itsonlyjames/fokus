use clap::Parser;
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use mac_notification_sys::*;
mod ui;
use ratatui::{DefaultTerminal, Frame};
use tokio::{
    sync::{broadcast, mpsc},
    time::{Duration, sleep},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 25)]
    working: u64,
    #[arg(short, long, default_value_t = 5)]
    break_time: u64,
}

#[derive(Debug)]
enum TimerState {
    Work,
    Break,
}

async fn countdown(seconds: u64, tx: mpsc::Sender<u64>, mut running_rx: broadcast::Receiver<bool>) {
    let mut remaining = seconds;
    let mut is_running = true;

    while remaining > 0 {
        tokio::select! {
            Ok(running) = running_rx.recv() => {
                is_running = running;
            }
            _ = sleep(Duration::from_secs(1)), if is_running => {
                remaining -= 1;
                let _ = tx.send(remaining).await;
            }
        }
    }
    let _ = tx.send(0).await;
}

#[derive(Debug)]
pub struct App {
    app_running: bool,
    event_stream: EventStream,
    current_state: TimerState,
    args: Cli,
    remaining_timer: u64,
    countdown_running: bool,
    timer_active: bool,
    transmitter: mpsc::Sender<u64>,
    running_tx: broadcast::Sender<bool>,
}

impl App {
    pub fn new(args: Cli) -> (Self, mpsc::Receiver<u64>) {
        let (tx, rx) = mpsc::channel(100);
        let (running_tx, _) = broadcast::channel(100);
        (
            Self {
                app_running: true,
                event_stream: EventStream::new(),
                current_state: TimerState::Work,
                args,
                remaining_timer: 0,
                countdown_running: false,
                timer_active: false,
                transmitter: tx,
                running_tx,
            },
            rx,
        )
    }

    pub async fn run(
        mut self,
        mut terminal: DefaultTerminal,
        mut rx: mpsc::Receiver<u64>,
    ) -> Result<()> {
        self.app_running = true;

        while self.app_running {
            terminal.draw(|frame| self.draw(frame))?;
            tokio::select! {
                event = self.event_stream.next().fuse() => {
                    match event {
                        Some(Ok(evt)) => {
                            match evt {
                                Event::Key(key) if key.kind == KeyEventKind::Press => {
                                    self.on_key_event(key);
                                }
                                Event::Mouse(_) => {}
                                Event::Resize(_, _) => {}
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                Some(secs) = rx.recv() => {
                    self.remaining_timer = secs;
                    if secs == 0 {
                        self.countdown_running = false;
                        self.timer_active = false;

                        let (summary, body) = match self.current_state {
                            TimerState::Break => ("Session Finished!", "Time to take a break"),
                            TimerState::Work => ("Break Finished", "Time for another session")
                        };

                        #[cfg(target_os="linux")]
                        Notification::new()
                            .summary("Pomodoro")
                            .body(format!("{} {}", summary, body))
                            .icon("alarm")
                            .show();


                        #[cfg(target_os = "macos")]
                        send_notification(
                            "Pomodoro",
                            Some(summary),
                            body,
                            Some(Notification::new().sound("Blow"))
                        ).unwrap();

                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        ui::draw(self, frame);
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Char('s')) => {
                if !self.timer_active {
                    let (duration, next_state) = match self.current_state {
                        TimerState::Work => (self.args.working * 60, TimerState::Break),
                        TimerState::Break => (self.args.break_time * 60, TimerState::Work),
                    };

                    self.remaining_timer = duration;
                    self.countdown_running = true;
                    self.timer_active = true;

                    let tx = self.transmitter.clone();
                    let running_rx = self.running_tx.subscribe();

                    self.running_tx.send(true).unwrap();

                    tokio::spawn(async move {
                        countdown(duration, tx, running_rx).await;
                    });

                    self.current_state = next_state;
                } else if !self.countdown_running {
                    self.countdown_running = true;
                    self.running_tx.send(true).unwrap();
                }
            }
            (_, KeyCode::Char('p')) => {
                if self.remaining_timer > 0 {
                    self.countdown_running = !self.countdown_running;
                    self.running_tx.send(self.countdown_running).unwrap();
                }
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        if !self.countdown_running {
            self.app_running = false;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Set identifier for notifications
    let bundle = get_bundle_identifier_or_default("terminal");
    set_application(&bundle).unwrap();

    let args = Cli::parse();
    let terminal = ratatui::init();
    let (app, rx) = App::new(args);
    let result = app.run(terminal, rx).await;
    ratatui::restore();
    result
}
