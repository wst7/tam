mod project_screen;
mod task_screen;
mod components;

use ratatui::Frame;
use crate::app::{App, CurrentScreen};

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
