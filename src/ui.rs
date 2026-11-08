mod components;
mod project_screen;
mod task_screen;

use crate::app::{App, CurrentScreen};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.current_screen {
        CurrentScreen::Project => {
            project_screen::draw(frame, app);
        }
        CurrentScreen::Task => {
            task_screen::draw(frame, app);
        }
    }
}
