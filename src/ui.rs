use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use crate::App;

pub fn draw(app: &App, frame: &mut Frame) {
    let title = Line::from("Pomodoro").bold().blue();
    let countdown_text = if app.is_countdown_running {
        format!(
            "⏳ Time remaining: {:02}:{:02}",
            app.countdown_seconds / 60,
            app.countdown_seconds % 60
        )
    } else if app.countdown_seconds > 0 {
        format!(
            "⏸ Paused: {:02}:{:02}",
            app.countdown_seconds / 60,
            app.countdown_seconds % 60
        )
    } else {
        "Press 's' to start countdown".to_string()
    };
    let area = frame.area();
    let _chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Main content
    let text = format!(
        "Pomo!\n\n\
        Work duration: {} minutes\n\
        Break duration: {} minutes\n\n\
        {}",
        app.args.working, app.args.break_time, countdown_text
    );

    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title(title))
            .centered(),
        area,
    );

    // let footer = Paragraph::new("Press `Esc`, `Ctrl-C` or `q` to quit").centered();
    // frame.render_widget(footer, chunks[1]);
}
