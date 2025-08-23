use crate::{
    App, TimerState,
    settings::{Screen, Settings, SettingsField},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(app: &App, frame: &mut Frame) {
    match app.get_current_screen() {
        Screen::Timer => draw_timer_screen(app, frame),
        Screen::Settings => draw_settings_screen(app, frame),
    }
}

fn draw_timer_screen(app: &App, frame: &mut Frame) {
    let title = Line::from("Fokus").bold().red();
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
        let controls_text = "'s' start | 'S' skip | 'o' settings | 'q' quit | '?' hide help";
        let constraints = if app.show_help {
            let controls_height = calculate_text_height(controls_text, inner_area.width);
            vec![
                Constraint::Min(0),
                Constraint::Length(6),
                Constraint::Min(0),
                Constraint::Length(controls_height),
            ]
        } else {
            vec![
                Constraint::Min(0),
                Constraint::Length(6),
                Constraint::Min(0),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let Settings {
            working_time,
            break_time,
            long_break_time,
            ..
        } = app.get_settings();

        let content = format!(
            "Fokus duration: {} minutes\n\
            Break duration: {} / {} minutes\n\
            Sessions completed: {} (today: {})\n\n\
            Press 's' to start {}",
            working_time,
            break_time,
            long_break_time,
            app.stats.get_total_sessions(),
            app.stats.get_today_sessions(),
            match app.current_state {
                TimerState::Work => "fokus session",
                TimerState::Break => {
                    let settings = app.get_settings();
                    let count_until_long_break = app.get_long_break_count();
                    if count_until_long_break > 0
                        && count_until_long_break % settings.sessions_until_long_break == 0
                    {
                        "long break"
                    } else {
                        "short break"
                    }
                }
            }
        );
        frame.render_widget(Paragraph::new(content).centered(), chunks[1]);

        if app.show_help {
            frame.render_widget(
                Paragraph::new(controls_text)
                    .centered()
                    .style(Style::default().fg(Color::Gray))
                    .wrap(Wrap { trim: true }),
                chunks[3],
            );
        }
    } else {
        let controls_text = match app.countdown_running {
            true => "'p' pause | 'S' skip | 'o' settings | 'q' quit | '?' hide help",
            false => "'p' resume | 'r' reset | 'S' skip | 'o' settings | 'q' quit | '?' hide help",
        };
        let constraints = if app.show_help {
            let controls_height = calculate_text_height(controls_text, inner_area.width);
            vec![
                Constraint::Min(0),
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(controls_height),
            ]
        } else {
            vec![
                Constraint::Min(0),
                Constraint::Length(4),
                Constraint::Min(0),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let session_info = match app.current_state {
            TimerState::Work => "🎧 Fokus Session",
            TimerState::Break => {
                let settings = app.get_settings();
                let count_until_long_break = app.get_long_break_count();
                if count_until_long_break > 0
                    && count_until_long_break % settings.sessions_until_long_break == 0
                {
                    "☕ Long Break"
                } else {
                    "☕ Short Break"
                }
            }
        };

        let timer_content = format!(
            "{}\n\n{}",
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

        if app.show_help {
            frame.render_widget(
                Paragraph::new(controls_text)
                    .centered()
                    .style(Style::default().fg(Color::Gray))
                    .wrap(Wrap { trim: true }),
                chunks[3],
            );
        }
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

    let instructions_text = if app.is_editing_field() {
        "✏ Editing: Enter numbers | 'Enter' to save | 'Esc' to cancel"
    } else {
        "Navigation: ↑↓ to move | 'Enter' to edit | 'Esc' to return to timer"
    };
    let instructions_height =
        calculate_text_height(instructions_text, inner_area.width.saturating_sub(2));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
            Constraint::Length(instructions_height + 2), // +2 for borders
        ])
        .split(inner_area);

    let total_sessions = app.stats.get_total_sessions();
    let header_text = format!(
        "Configure your Fokus settings\nSessions completed: {}",
        total_sessions
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
            "Fokus Time",
            &format!("{} minutes", settings.working_time),
            matches!(current_field, SettingsField::WorkingTime),
            editing,
            app.get_input_buffer(),
        ),
        create_setting_item(
            "Break Time",
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

    if let Some(save_time) = app.settings_saved_message {
        if save_time.elapsed().as_secs() < 2 {
            let save_msg = Paragraph::new("✓ Settings saved!")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Green));
            frame.render_widget(save_msg, chunks[2]);
        }
    }

    let help = Paragraph::new(instructions_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true })
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

fn calculate_text_height(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }

    let chars_per_line = width as usize;
    let total_chars = text.chars().count();

    if total_chars <= chars_per_line {
        1
    } else {
        ((total_chars + chars_per_line - 1) / chars_per_line) as u16
    }
}
