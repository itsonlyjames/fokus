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

async fn countdown(seconds: u64, tx: mpsc::Sender<u64>, mut running_rx: broadcast::Receiver<bool>) {
    let mut remaining = seconds;
    let mut is_running = true;
    while remaining > 0 {
        tokio::select! {
            Ok(running) = running_rx.recv() => {
                is_running = running;
            }
            _ = sleep(Duration::from_secs(1)), if is_running => {
                let _ = tx.send(remaining).await;
                remaining -= 1;
            }
        }
    }
    let _ = tx.send(0).await;
}

#[derive(Debug)]
pub struct App {
    running: bool,
    event_stream: EventStream,
    args: Cli,
    countdown_seconds: u64,
    is_countdown_running: bool,
    tx: mpsc::Sender<u64>,
    running_tx: broadcast::Sender<bool>,
}

impl App {
    pub fn new(args: Cli) -> (Self, mpsc::Receiver<u64>, broadcast::Receiver<bool>) {
        let (tx, rx) = mpsc::channel(100);
        let (running_tx, running_rx) = broadcast::channel(100);
        (
            Self {
                running: true,
                event_stream: EventStream::new(),
                args,
                countdown_seconds: 0,
                is_countdown_running: false,
                tx,
                running_tx,
            },
            rx,
            running_rx,
        )
    }

    pub async fn run(
        mut self,
        mut terminal: DefaultTerminal,
        mut rx: mpsc::Receiver<u64>,
    ) -> Result<()> {
        self.running = true;

        while self.running {
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
                    self.countdown_seconds = secs;
                    if secs == 0 {
                        self.is_countdown_running = false;

                        #[cfg(target_os="linux")]
                        Notification::new()
                            .summary("Pomodoro")
                            .body("Session Finished! Time to take a break")
                            .icon("alarm") // Optional: use a system icon
                            .show();


                        #[cfg(target_os = "macos")]
                        send_notification(
                            "Pomodoro",
                            Some("Session Finished!"),
                            "Time to take a break",
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
                if !self.is_countdown_running {
                    self.is_countdown_running = true;
                    let counter = self.args.working * 60;
                    self.countdown_seconds = counter;
                    let tx = self.tx.clone();
                    let running_rx = self.running_tx.subscribe();
                    let work_seconds = counter;
                    let _ = self.running_tx.send(true).unwrap();
                    tokio::spawn(async move {
                        countdown(work_seconds, tx, running_rx).await;
                    });
                }
                // handle pressing s when paused
            }
            (_, KeyCode::Char('p')) => {
                if self.countdown_seconds > 0 {
                    self.is_countdown_running = !self.is_countdown_running;
                    let _ = self.running_tx.send(self.is_countdown_running).unwrap();
                }
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        if !self.is_countdown_running {
            self.running = false;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let bundle = get_bundle_identifier_or_default("terminal");
    set_application(&bundle).unwrap();
    let args = Cli::parse();
    let terminal = ratatui::init();
    let (app, rx, _running_rx) = App::new(args);
    let result = app.run(terminal, rx).await;
    ratatui::restore();
    result
}
