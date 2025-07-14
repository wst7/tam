use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::App,
    model::TaskStatus,
    ui::components::{centered_rect, render_confirm, render_tips},
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .split(frame.area());

    render_tasks_view(frame, app, chunks[0]);
    render_help_view(frame, app, chunks[1]);

    render_edit_or_add_view(frame, app);
    render_tips(frame, app);
    render_confirm(frame, app);
}

fn render_tasks_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let project = app.get_project();
    if project.is_none() {
        app.tip_message = Some("请先选择项目".to_string());
        return;
    }
    app.query_tasks();

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Min(1), Constraint::Min(1)])
        .split(area);

    render_task_with_status(frame, app, layout[0], TaskStatus::Todo);
    render_task_with_status(frame, app, layout[1], TaskStatus::InProgress);
    render_task_with_status(frame, app, layout[2], TaskStatus::Done);
}

fn render_task_with_status(frame: &mut Frame, app: &mut App, area: Rect, status: TaskStatus) {
    let tasks = app.get_tasks_with_status(status);
    let tasks = tasks
        .iter()
        .map(|t| ListItem::new(t.title.clone()))
        .collect::<Vec<_>>();

    let block = Block::bordered()
        .style(
            Style::default()
                .bg(app.theme.background)
                .fg(app.theme.foreground),
        )
        .title(status.to_string())
        .title_style(Style::default().fg(app.theme.title_foreground));
    let list = List::new(tasks).block(block.clone()).highlight_style(
        Style::default()
            .bg(app.theme.highlight_bg)
            .fg(app.theme.highlight_fg),
    );
    if let Some(st) = app.selected_task_status {
        if st == status {
            let mut state = ListState::default();
            if let Some(selected) = app.selected_task {
                state.select(Some(selected));
            }
            let list = list.block(block.border_style(Style::default().fg(app.theme.highlight_bd)));
            frame.render_stateful_widget(list, area, &mut state);
            return;
        }
    }
    frame.render_widget(list, area);
}

fn render_help_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let key_map = vec![
        ("q", "Quit"),
        ("esc", "Back"),
        ("↑↓←→", "Navigate"),
        ("e", "Edit"),
        ("a", "Add"),
        ("d", "Delete"),
        ("s", "Start"),
        ("c", "Complete"),
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
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(app.theme.background))
                .border_style(Style::default().fg(app.theme.border))
                .title("Help")
                .title_style(Style::default().fg(app.theme.title_foreground)),
        );
    frame.render_widget(help, area);
}

fn render_edit_or_add_view(frame: &mut Frame, app: &mut App) {
    if app.edit_task || app.add_task {
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
            .block(
                Block::bordered()
                    .border_style(Style::default().fg(app.theme.border))
                    .title("输入任务名称")
                    .title_style(Style::default().fg(app.theme.title_foreground)),
            );

        frame.render_widget(input, chunks[0]);

        let help = Paragraph::new(Line::from_iter([
            "Esc 取消, ".to_span(),
            "Enter 保存".bold().fg(app.theme.help_key),
        ]))
        .style(
            Style::default()
                .fg(app.theme.help_text)
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
