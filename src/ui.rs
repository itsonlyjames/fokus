use crate::{App, TimerState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Clear, Gauge, Paragraph},
};

pub fn draw(app: &App, frame: &mut Frame) {
    let title = Line::from(" Pomo! ").bold().red();
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
        // Use Layout to get a vertically centered chunk inside inner_area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(inner_area);

        let content = format!(
            "Press 's' to start {}",
            match app.current_state {
                TimerState::Work => "session",
                TimerState::Break => "break",
            }
        );

        // Render the paragraph centered inside the middle chunk (chunks[1])
        frame.render_widget(Paragraph::new(content).centered(), chunks[1]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(4),
                Constraint::Fill(1),
                Constraint::Percentage(3),
            ])
            .split(inner_area);

        // Draw the full content inside the inner area as well
        let content = format!(
            "Work duration: {} minutes\n\
             Break duration: {} minutes\n\n\
             {}",
            app.args.working_time,
            app.args.break_time,
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

        frame.render_widget(Paragraph::new(content).centered(), chunks[1]);

        let current_pos = app.remaining_timer;
        let total_time = match app.current_state {
            TimerState::Work => app.args.get_working_time(),
            TimerState::Break => app.args.get_break_time(),
        };
        let ratio = 1.0 - (current_pos as f64 / total_time as f64);

        frame.render_widget(Gauge::default().ratio(ratio), chunks[3]);
    };

    // let footer = Paragraph::new("Press `Esc`, `Ctrl-C` or `q` to quit").centered();
    // frame.render_widget(footer, chunks[1]);
}
