use clap::Parser;
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use notify_rust::{Notification, get_bundle_identifier_or_default, set_application};
use ratatui::{DefaultTerminal, Frame};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

mod cli;
mod config;
mod settings;
mod timer;
mod ui;
mod stats;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 25, value_parser = cli::validate_time)]
    working_time: u64,
    #[arg(short, long, default_value_t = 5, value_parser = cli::validate_time)]
    break_time: u64,
    #[arg(short, long, default_value_t = 15, value_parser = cli::validate_time)]
    long_break_time: u64,
    #[arg(short, long, default_value_t = 2, value_parser = cli::validate_time)]
    sessions_until_break_time: u64,
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
    remaining_timer: u64,
    countdown_running: bool,
    timer_active: bool,
    transmitter: mpsc::Sender<u64>,
    running_tx: broadcast::Sender<bool>,
    countdown_task: Option<JoinHandle<()>>,
    transition_pending: bool,
    current_screen: settings::Screen,
    settings: settings::Settings,
    settings_field: settings::SettingsField,
    editing_field: bool,
    input_buffer: String,
    long_break_count: u64,
    settings_saved_message: Option<std::time::Instant>,
    stats: stats::SessionStats,
    stats_saved_message: Option<std::time::Instant>,
}

impl App {
    pub fn new(args: Cli) -> (Self, mpsc::Receiver<u64>) {
        let (tx, rx) = mpsc::channel(100);
        let (running_tx, _) = broadcast::channel(100);

        let settings = config::Config::load_settings().unwrap_or_else(|_| settings::Settings {
            working_time: args.working_time,
            break_time: args.break_time,
            long_break_time: args.long_break_time,
            sessions_until_long_break: args.sessions_until_break_time,
        });

        let stats = stats::SessionStats::load_stats().unwrap_or_default();

        (
            Self {
                app_running: true,
                event_stream: EventStream::new(),
                current_state: TimerState::Work,
                remaining_timer: 0,
                countdown_running: false,
                timer_active: false,
                transmitter: tx,
                running_tx,
                countdown_task: None,
                transition_pending: false,
                current_screen: settings::Screen::Timer,
                settings,
                settings_field: settings::SettingsField::WorkingTime,
                editing_field: false,
                input_buffer: String::new(),
                long_break_count: 0,
                settings_saved_message: None,
                stats,
                stats_saved_message: None,
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

                        let (summary, _body) = match self.current_state {
                            TimerState::Work => {
                                self.long_break_count += 1;

                                self.stats.increment_session();
                                self.save_stats();

                                if self.long_break_count % self.settings.sessions_until_long_break == 0 {
                                    ("Session Finished", "Time for a long break!")
                                } else {
                                    ("Session Finished", "Time for a short break")
                                }
                            },
                            TimerState::Break => {
                                if self.long_break_count > 0 && self.long_break_count % self.settings.sessions_until_long_break == 0 {
                                    self.long_break_count = 0;
                                }
                                ("Break Finished", "Time for another session")
                            }
                        };



                        Notification::new()
                            .summary("Pomodoro")
                            .body(summary)
                            // .message(body)
                            .sound_name("Blow")
                            .icon("alarm")
                            // .main_button(MainButton::SingleAction("Start Next Session"))
                            .show()?;

                        // if let Ok(response) = response {
                        //     notification_actions::handle_response(response);
                        // } else {
                        //    eprint!("Failed to send notification");
                        // };

                        self.current_state = match self.current_state {
                            TimerState::Work => TimerState::Break,
                            TimerState::Break => TimerState::Work,
                        };
                    }
                }
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
                TimerState::Work => self.settings.get_working_time_seconds(),
                TimerState::Break => {
                    if self.long_break_count > 0
                        && self.long_break_count % self.settings.sessions_until_long_break == 0
                    {
                        self.settings.get_long_break_time_seconds()
                    } else {
                        self.settings.get_break_time_seconds()
                    }
                }
            };

            self.remaining_timer = duration;
            self.countdown_running = true;
            self.timer_active = true;

            let tx = self.transmitter.clone();
            let running_rx = self.running_tx.subscribe();

            let _ = self.running_tx.send(true);

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
            let _ = self.running_tx.send(true);
        }
    }

    fn pause_timer(&mut self) {
        if self.timer_active {
            self.countdown_running = !self.countdown_running;
            let _ = self.running_tx.send(self.countdown_running);
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
            let _ = self.running_tx.send(false);
        } else {
            // hanlde confirmation of reset when timer running
            // so as to ignore accidental presses
        }
    }

    fn skip_session(&mut self) {
        if self.countdown_running {
            if let Some(task) = self.countdown_task.take() {
                task.abort();
            }
            self.countdown_task = None;
            self.running_tx.send(false).unwrap();
        }
        self.remaining_timer = 0;
        self.countdown_running = false;
        self.timer_active = false;

        if matches!(self.current_state, TimerState::Work) {
            self.long_break_count += 1;
            self.stats.increment_session();
            self.save_stats();
        }

        self.current_state = match self.current_state {
            TimerState::Work => TimerState::Break,
            TimerState::Break => TimerState::Work,
        };
    }

    pub fn get_current_screen(&self) -> &settings::Screen {
        &self.current_screen
    }

    pub fn get_settings(&self) -> &settings::Settings {
        &self.settings
    }

    pub fn get_settings_field(&self) -> &settings::SettingsField {
        &self.settings_field
    }

    pub fn is_editing_field(&self) -> bool {
        self.editing_field
    }

    pub fn get_input_buffer(&self) -> &str {
        &self.input_buffer
    }

    pub fn get_long_break_count(&self) -> u64 {
        self.long_break_count
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match self.current_screen {
            settings::Screen::Timer => self.handle_timer_input(key),
            settings::Screen::Settings => self.handle_settings_input(key),
        }
    }

    fn handle_timer_input(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Char('s')) => self.start_timer(),
            (_, KeyCode::Char('p')) => self.pause_timer(),
            (_, KeyCode::Char('r')) => self.reset_timer(),
            (_, KeyCode::Char('S')) => self.skip_session(),
            (_, KeyCode::Char('o')) => self.current_screen = settings::Screen::Settings,
            _ => {}
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) {
        if self.editing_field {
            self.handle_field_editing(key);
        } else {
            self.handle_settings_navigation(key);
        }
    }

    fn handle_settings_navigation(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.current_screen = settings::Screen::Timer,
            KeyCode::Up | KeyCode::Char('k') => self.previous_setting(),
            KeyCode::Down | KeyCode::Char('j') => self.next_setting(),
            KeyCode::Enter => self.start_editing(),
            _ => {}
        }
    }

    fn handle_field_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.cancel_editing(),
            KeyCode::Enter => self.save_field(),
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if self.input_buffer.len() < 3 {
                    // Limit to 999 minutes
                    self.input_buffer.push(c);
                }
            }
            _ => {}
        }
    }

    fn previous_setting(&mut self) {
        self.settings_field = match self.settings_field {
            settings::SettingsField::WorkingTime => settings::SettingsField::SessionsUntilLongBreak,
            settings::SettingsField::BreakTime => settings::SettingsField::WorkingTime,
            settings::SettingsField::LongBreakTime => settings::SettingsField::BreakTime,
            settings::SettingsField::SessionsUntilLongBreak => {
                settings::SettingsField::LongBreakTime
            }
        };
    }

    fn next_setting(&mut self) {
        self.settings_field = match self.settings_field {
            settings::SettingsField::WorkingTime => settings::SettingsField::BreakTime,
            settings::SettingsField::BreakTime => settings::SettingsField::LongBreakTime,
            settings::SettingsField::LongBreakTime => {
                settings::SettingsField::SessionsUntilLongBreak
            }
            settings::SettingsField::SessionsUntilLongBreak => settings::SettingsField::WorkingTime,
        };
    }

    fn start_editing(&mut self) {
        self.editing_field = true;
        self.input_buffer = match self.settings_field {
            settings::SettingsField::WorkingTime => self.settings.working_time.to_string(),
            settings::SettingsField::BreakTime => self.settings.break_time.to_string(),
            settings::SettingsField::LongBreakTime => self.settings.long_break_time.to_string(),
            settings::SettingsField::SessionsUntilLongBreak => {
                self.settings.sessions_until_long_break.to_string()
            }
        };
    }

    fn cancel_editing(&mut self) {
        self.editing_field = false;
        self.input_buffer.clear();
    }

    fn save_field(&mut self) {
        if let Ok(value) = self.input_buffer.parse::<u64>() {
            if value > 0 {
                match self.settings_field {
                    settings::SettingsField::WorkingTime => self.settings.working_time = value,
                    settings::SettingsField::BreakTime => self.settings.break_time = value,
                    settings::SettingsField::LongBreakTime => self.settings.long_break_time = value,
                    settings::SettingsField::SessionsUntilLongBreak => {
                        self.settings.sessions_until_long_break = value
                    }
                }

                match config::Config::save_settings(&self.settings) {
                    Ok(_) => {
                        self.settings_saved_message = Some(std::time::Instant::now());
                    }
                    Err(e) => {
                        eprintln!("Failed to save settings: {}", e);
                    }
                }
            }
        }
        self.editing_field = false;
        self.input_buffer.clear();
    }

    // Add new methods to App:
    fn save_stats(&mut self) {
        match stats::SessionStats::save_stats(&self.stats) {
            Ok(_) => {
                self.stats_saved_message = Some(std::time::Instant::now());
            }
            Err(e) => {
                eprintln!("Failed to save stats: {}", e);
            }
        }
    }
    
    pub fn get_stats(&self) -> &stats::SessionStats {
        &self.stats
    }

    fn quit(&mut self) {
        self.app_running = false;
        use crossterm::execute;
        use crossterm::terminal::{Clear, ClearType};
        let _ = execute!(std::io::stdout(), Clear(ClearType::All));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    #[cfg(feature = "debug")]
    console_subscriber::init();

    // Set identifier for notifications
    #[cfg(target_os = "macos")]
    let bundle = get_bundle_identifier_or_default("terminal");
    set_application(&bundle).unwrap();

    let args = Cli::parse();
    let terminal = ratatui::init();
    let (app, rx) = App::new(args);
    let result = app.run(terminal, rx).await;
    ratatui::restore();
    result
}
