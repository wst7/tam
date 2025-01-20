use crate::query::{add_task, get_all_tasks};
use crate::task::{Task, TaskStatus};
use cursive::view::Nameable;
use cursive::view::Resizable;
use cursive::views::{EditView, ResizedView, ScrollView};
use cursive::Cursive;
use cursive::{
    align::HAlign,
    views::{Button, Dialog, LinearLayout, ListView, Panel, TextView},
};

#[derive(Clone)]
pub struct KanbanBoard {
    pub todo_tasks: Vec<(usize, Task)>,
    pub in_progress_tasks: Vec<(usize, Task)>,
    pub done_tasks: Vec<(usize, Task)>,
}

impl KanbanBoard {
    pub fn new() -> Self {
        KanbanBoard {
            todo_tasks: Vec::new(),
            in_progress_tasks: Vec::new(),
            done_tasks: Vec::new(),
        }
    }

    fn load_tasks(&mut self) {
        let tasks = get_all_tasks().unwrap();
        self.done_tasks = Vec::new();
        self.in_progress_tasks = Vec::new();
        self.todo_tasks = Vec::new();
        for (i, task) in tasks {
            match task.status {
                TaskStatus::Todo => self.todo_tasks.push((i, task.clone())),
                TaskStatus::InProgress => self.in_progress_tasks.push((i, task.clone())),
                TaskStatus::Done => self.done_tasks.push((i, task.clone())),
                _ => (),
            }
        }
    }

    pub fn render(&mut self) -> Dialog {
        self.load_tasks();
        let todo_panel = create_task_panel(self, TaskStatus::Todo);
        let in_progress_panel = create_task_panel(self, TaskStatus::InProgress);
        let done_panel = create_task_panel(self, TaskStatus::Done);

        Dialog::around(
            LinearLayout::vertical()
                .child(
                    LinearLayout::horizontal()
                        .child(todo_panel)
                        .child(in_progress_panel)
                        .child(done_panel),
                )
                .child(
                    LinearLayout::horizontal()
                        .child(Button::new("q: Quit", |s| s.quit()).min_width(16))
                        .child(Button::new("a: Add Task", |s| add_task_dialog(s)).min_width(16)),
                ),
        )
        .title("Tasks Kanban Board")
        .h_align(HAlign::Center)
    }
}

// 创建任务面板
fn create_task_panel(
    board: &KanbanBoard,
    status: TaskStatus,
) -> ResizedView<Panel<ScrollView<ListView>>> {
    let tasks = match status {
        TaskStatus::Todo => &board.todo_tasks,
        TaskStatus::InProgress => &board.in_progress_tasks,
        TaskStatus::Done => &board.done_tasks,
        _ => [].as_ref(),
    };

    let mut list_view = ListView::new();

    for (id, task) in tasks {
        list_view.add_child(
            format!("{id}:"),
            LinearLayout::horizontal().child(TextView::new(&task.title)),
        );
    }
    ResizedView::with_fixed_width(
        30,
        Panel::new(ScrollView::new(list_view)).title(status.to_string()),
    )
}

// 添加任务对话框
pub fn add_task_dialog(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical().child(EditView::new().with_name("task_title").fixed_width(40)),
        )
        .title("Add Task")
        .button("Add", |s| {
            let title = match s.call_on_name("task_title", |view: &mut EditView| view.get_content())
            {
                Some(title) => title.to_string(),
                None => "".to_string(),
            };
            if title.is_empty() {
                s.add_layer(Dialog::info("Task title is required!"));
                return;
            }
            let mut task = Task::default();
            task.set_title(title.clone());
            let _ = add_task(task);

            rerender_board(s);
        })
        .button("Cancel", |s| {
            s.pop_layer();
        }),
    );
}

fn rerender_board(siv: &mut Cursive) {
    // 刷新界面
    siv.pop_layer();
    if let Some(board) = siv.user_data::<KanbanBoard>() {
        let board_view = board.render();
        siv.add_layer(board_view);
    } else {
        siv.add_layer(Dialog::info("Kanban board not found!"));
    }
}
