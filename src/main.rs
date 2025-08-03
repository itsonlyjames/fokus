use clap::Parser;
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use mac_notification_sys::*;
use ratatui::{DefaultTerminal, Frame};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::Duration,
};

mod timer;
mod ui;
mod cli;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 25)]
    #[arg(short, long, default_value_t = 25, value_parser = cli::validate_time)]
    working_time: u64,
    #[arg(short, long, default_value_t = 5)]
    #[arg(short, long, default_value_t = 5, value_parser = cli::validate_time)]
    break_time: u64,
}

impl Cli {
    pub fn get_working_time(&self) -> u64 {
        self.working_time * 60
    }

    pub fn get_break_time(&self) -> u64 {
        self.break_time * 60
    }
}

#[derive(Debug)]
enum TimerState {
    Work,
    Break,
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
    countdown_task: Option<JoinHandle<()>>,
    transition_pending: bool,

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
                countdown_task: None,
                transition_pending: false,
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
                    if secs == 0 && !self.transition_pending {
                        self.transition_pending = true;
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

                        self.current_state = match self.current_state {
                            TimerState::Work => TimerState::Break,
                            TimerState::Break => TimerState::Work,
                        };
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

    fn start_timer(&mut self) {
        if !self.timer_active {
            if let Some(task) = self.countdown_task.take() {
                task.abort();
            }

            self.transition_pending = false;

            let duration = match self.current_state {
                TimerState::Work => self.args.get_working_time(),
                TimerState::Break => self.args.get_break_time(),
            };

            self.remaining_timer = duration;
            self.countdown_running = true;
            self.timer_active = true;

            let tx = self.transmitter.clone();
            let running_rx = self.running_tx.subscribe();

            self.running_tx.send(true).unwrap();

            self.countdown_task = Some(tokio::spawn(async move {
                timer::countdown(duration, tx, running_rx).await;
            }));
        } else {
            self.resume_timer();
        }
    }

    fn resume_timer(&mut self) {
        if !self.countdown_running {
            self.countdown_running = true;
            self.running_tx.send(true).unwrap();
        }
    }

    fn pause_timer(&mut self) {
        if self.remaining_timer > 0 {
            self.countdown_running = !self.countdown_running;
            self.running_tx.send(self.countdown_running).unwrap();
        }
    }

    fn reset_timer(&mut self) {
        if !self.countdown_running {
            if let Some(task) = self.countdown_task.take() {
                task.abort();
            }

            self.remaining_timer = 0;
            self.countdown_running = false;
            self.timer_active = false;
            self.countdown_task = None;
            self.running_tx.send(false).unwrap();
        }
    }

    fn skip_session(&mut self) {
        self.current_state = match self.current_state {
            TimerState::Work => TimerState::Break,
            TimerState::Break => TimerState::Work,
        };
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Char('s')) => self.start_timer(),
            (_, KeyCode::Char('p')) => self.pause_timer(),
            (_, KeyCode::Char('r')) => self.reset_timer(),
            (_, KeyCode::Char('S')) => self.skip_session(),
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.app_running = false;
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
