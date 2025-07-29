use crate::{App, TimerState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};

pub fn draw(app: &App, frame: &mut Frame) {
    let title = Line::from(" Pomo! ").bold().red();
    let area = frame.area();

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
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(50),
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
        // Draw the full content inside the inner area as well
        let content = format!(
            "\n\nWork duration: {} minutes\n\
             Break duration: {} minutes\n\n\
             {}",
            app.args.working,
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

        frame.render_widget(Paragraph::new(content).centered(), inner_area);
    };

    // let footer = Paragraph::new("Press `Esc`, `Ctrl-C` or `q` to quit").centered();
    // frame.render_widget(footer, chunks[1]);
}
