use crate::{
    App, TimerState,
    settings::{Screen, SettingsField},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
};

pub fn draw(app: &App, frame: &mut Frame) {
    match app.get_current_screen() {
        Screen::Timer => draw_timer_screen(app, frame),
        Screen::Settings => draw_settings_screen(app, frame),
    }
}

fn draw_timer_screen(app: &App, frame: &mut Frame) {
    let title = Line::from("Pomodoro").bold().red();
    let area = frame.area();
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::bordered()
            .title(title)
            .border_type(BorderType::Rounded),
        area,
    );
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if app.remaining_timer == 0 && !app.countdown_running {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(6),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner_area);

        let settings = app.get_settings();
        let session_count = app.get_session_count();
        let next_break_type =
            if session_count > 0 && (session_count + 1) % settings.sessions_until_long_break == 0 {
                format!("long break ({} min)", settings.long_break_time)
            } else {
                format!("short break ({} min)", settings.break_time)
            };

        let content = format!(
            "Work duration: {} minutes\n\
            Break duration: {} / {} minutes\n\
            Sessions completed: {}\n\
            Next break: {}\n\n\
            Press 's' to start {}",
            settings.working_time,
            settings.break_time,
            settings.long_break_time,
            session_count,
            next_break_type,
            match app.current_state {
                TimerState::Work => "work session",
                TimerState::Break => "break",
            }
        );
        frame.render_widget(Paragraph::new(content).centered(), chunks[1]);

        let controls =
            "Controls: 's' start | 'p' pause | 'r' reset | 'S' skip | 'o' settings | 'q' quit";
        frame.render_widget(
            Paragraph::new(controls)
                .centered()
                .style(Style::default().fg(Color::Gray)),
            chunks[3],
        );
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3), // Timer display
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1), // Controls
            ])
            .split(inner_area);

        let session_info = match app.current_state {
            TimerState::Work => "🍅 Work Session",
            TimerState::Break => {
                let settings = app.get_settings();
                let session_count = app.get_session_count();
                if session_count > 0 && session_count % settings.sessions_until_long_break == 0 {
                    "☕ Long Break"
                } else {
                    "☕ Short Break"
                }
            }
        };

        let timer_content = format!(
            "{}\n{}",
            session_info,
            if app.countdown_running {
                format!(
                    "⏳ Time remaining: {:02}:{:02}",
                    app.remaining_timer / 60,
                    app.remaining_timer % 60
                )
            } else {
                format!(
                    "⏸ Paused: {:02}:{:02}",
                    app.remaining_timer / 60,
                    app.remaining_timer % 60
                )
            }
        );
        frame.render_widget(Paragraph::new(timer_content).centered(), chunks[1]);

        let current_pos = app.remaining_timer;
        let settings = app.get_settings();
        let total_time = match app.current_state {
            TimerState::Work => settings.get_working_time_seconds(),
            TimerState::Break => {
                let session_count = app.get_session_count();
                if session_count > 0 && session_count % settings.sessions_until_long_break == 0 {
                    settings.get_long_break_time_seconds()
                } else {
                    settings.get_break_time_seconds()
                }
            }
        };
        eprintln!("{:?}", total_time);
        let ratio = 1.0 - (current_pos as f64 / total_time as f64);
        eprintln!("{:?}", ratio);
        if ratio > 0.0 && ratio < 1.0 {
            frame.render_widget(
                Gauge::default()
                    .ratio(ratio)
                    .style(match app.current_state {
                        TimerState::Work => Style::default().fg(Color::Red),
                        TimerState::Break => Style::default().fg(Color::Green),
                    }),
                chunks[2],
            );
        }

        let controls = "'p' pause/resume | 'r' reset | 'S' skip | 'o' settings | 'q' quit";
        frame.render_widget(
            Paragraph::new(controls)
                .centered()
                .style(Style::default().fg(Color::Gray)),
            chunks[4],
        );
    };
}

fn draw_settings_screen(app: &App, frame: &mut Frame) {
    let title = Line::from("Settings").bold().yellow();
    let area = frame.area();
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::bordered()
            .title(title)
            .border_type(BorderType::Rounded),
        area,
    );

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let session_count = app.get_session_count();
    let header_text = format!(
        "Configure your Pomodoro settings\nSessions completed: {}",
        session_count
    );
    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, chunks[0]);

    let settings = app.get_settings();
    let current_field = app.get_settings_field();
    let editing = app.is_editing_field();

    let items: Vec<ListItem> = vec![
        create_setting_item(
            "Working Time",
            &format!("{} minutes", settings.working_time),
            matches!(current_field, SettingsField::WorkingTime),
            editing,
            app.get_input_buffer(),
        ),
        create_setting_item(
            "Short Break Time",
            &format!("{} minutes", settings.break_time),
            matches!(current_field, SettingsField::BreakTime),
            editing,
            app.get_input_buffer(),
        ),
        create_setting_item(
            "Long Break Time",
            &format!("{} minutes", settings.long_break_time),
            matches!(current_field, SettingsField::LongBreakTime),
            editing,
            app.get_input_buffer(),
        ),
        create_setting_item(
            "Sessions Until Long Break",
            &settings.sessions_until_long_break.to_string(),
            matches!(current_field, SettingsField::SessionsUntilLongBreak),
            editing,
            app.get_input_buffer(),
        ),
    ];

    let settings_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(settings_list, chunks[1]);

    let instructions = if editing {
        "✏ Editing: Enter numbers | 'Enter' to save | 'Esc' to cancel"
    } else {
        "Navigation: ↑↓ to move | 'Enter' to edit | 'Esc' to return to timer"
    };

    if let Some(save_time) = app.settings_saved_message {
        if save_time.elapsed().as_secs() < 2 {
            let save_msg = Paragraph::new("✓ Settings saved!")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Green));
            frame.render_widget(save_msg, chunks[2]);
        }
    }

    let help = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
    frame.render_widget(help, chunks[3]);
}

fn create_setting_item(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    input_buffer: &str,
) -> ListItem<'static> {
    let display_value = if selected && editing {
        format!("  {}: ❯ {} ❮", label, input_buffer)
    } else if selected {
        format!("❯ {}: {}", label, value)
    } else {
        format!("  {}: {}", label, value)
    };

    let style = if selected {
        if editing {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        }
    } else {
        Style::default().fg(Color::White)
    };

    ListItem::new(Line::from(Span::styled(display_value, style)))
}
