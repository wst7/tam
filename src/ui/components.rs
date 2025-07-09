use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

pub fn render_tips(frame: &mut Frame, app: &mut App) {
    if let Some(msg) = &app.tip_message {
        let area = frame.area();
        let tip_area = centered_rect(50, 18, area);

        // 强制清除之前内容避免透字
        frame.render_widget(Clear, tip_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(tip_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("提示")
            .style(Style::default().bg(Color::Black));

        let text = Paragraph::new(msg.clone())
            .style(Style::default().fg(Color::LightYellow))
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(text, chunks[0]);

        let help = Paragraph::new(Line::from_iter(["Enter 关闭".bold()]))
            .alignment(Alignment::Right)
            .style(Style::default().bg(Color::Black));
        frame.render_widget(help, chunks[1]);
    }
}

pub fn render_confirm(frame: &mut Frame, app: &mut App) {
    if let Some(msg) = &app.confirm_message {
        let area = frame.area();
        let tip_area = centered_rect(50, 18, area);

        // 强制清除之前内容避免透字
        frame.render_widget(Clear, tip_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(tip_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("确认")
            .style(Style::default().bg(Color::Black));

        let text = Paragraph::new(msg.clone())
            .style(Style::default().fg(Color::LightYellow))
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(text, chunks[0]);

        let help = Paragraph::new(Line::from_iter([
            "Esc <取消> ".bold(),
            "Enter <删除>".bold().fg(Color::Red),
        ]))
        .alignment(Alignment::Right)
        .style(Style::default().bg(Color::Black));
        frame.render_widget(help, chunks[1]);
    }
}
