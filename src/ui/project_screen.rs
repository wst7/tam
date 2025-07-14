use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::App,
    ui::components::{centered_rect, render_confirm, render_tips},
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .split(frame.area());

    render_projects_view(frame, app, chunks[0]);
    render_help_view(frame, app, chunks[1]);

    render_edit_or_add_view(frame, app);
    render_tips(frame, app);
    render_confirm(frame, app);
}

fn render_projects_view(frame: &mut Frame, app: &mut App, layout: Rect) {
    app.query_projects();
    let projects = app
        .projects
        .iter()
        .map(|p| ListItem::new(p.name.clone()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    if let Some(selected) = app.selected_project {
        state.select(Some(selected));
    }

    let list = List::new(projects)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .style(
                    Style::default()
                        .bg(app.theme.background)
                        .fg(app.theme.foreground),
                )
                .title("Projects views")
                .title_style(Style::default().fg(app.theme.title_foreground)),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.highlight_bg)
                .fg(app.theme.highlight_fg),
        );

    frame.render_stateful_widget(list, layout, &mut state);
}

fn render_help_view(frame: &mut Frame, app: &mut App, layout: Rect) {
    let key_map = vec![
        ("q", "Quit"),
        ("↑↓", "Navigate"),
        ("↵", "Enter"),
        ("e", "Edit"),
        ("a", "Add"),
        ("d", "Delete"),
        ("t", "Toggle Theme"),
    ];
    let mut spans = vec![];
    for (key, help_text) in key_map {
        spans.push(Span::styled(
            format!("<{}>", key),
            Style::default().fg(app.theme.help_key),
        ));
        spans.push(Span::styled(
            format!("{} ", help_text),
            Style::default().fg(app.theme.help_text),
        ));
    }
    let help = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .style(Style::default().bg(app.theme.background))
                .title("Help")
                .title_style(Style::default().fg(app.theme.title_foreground)),
        );
    frame.render_widget(help, layout);
}

fn render_edit_or_add_view(frame: &mut Frame, app: &mut App) {
    if app.edit_project || app.add_project {
        let area = frame.area();
        // 1. 居中弹窗
        let popup_area = centered_rect(60, 16, area);

        // 2. 清除之前的渲染内容
        frame.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(popup_area);

        let input = Paragraph::new(app.input.value())
            .style(
                Style::default()
                    .fg(app.theme.foreground)
                    .bg(app.theme.background),
            )
            .block(Block::bordered().title("输入项目名称"));

        frame.render_widget(input, chunks[0]);

        let help = Paragraph::new(Line::from_iter([
            "Esc 取消, ".to_span(),
            "Enter 保存".bold(),
        ]))
        .style(
            Style::default()
                .fg(app.theme.foreground)
                .bg(app.theme.background),
        )
        .alignment(Alignment::Right);
        frame.render_widget(help, chunks[1]);

        let width = popup_area.width.max(3) - 3;
        let scroll = app.input.visual_scroll(width as usize);
        let x = app.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((popup_area.x + x as u16, popup_area.y + 1))
    }
}
