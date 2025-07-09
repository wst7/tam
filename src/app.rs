use crate::{
    config,
    model::{Project, Task, TaskStatus},
    services,
    theme::Theme,
    ui,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug)]
pub enum CurrentScreen {
    Project,
    Task,
}

#[derive(Debug)]
pub struct App {
    pub current_screen: CurrentScreen,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub selected_project: Option<usize>, // 选中的project在list中的index
    pub selected_task: Option<usize>,    // 选中的task在list中的index
    pub selected_task_status: Option<TaskStatus>,
    pub edit_project: bool,
    pub add_project: bool,
    pub edit_task: bool,
    pub add_task: bool,
    pub input: Input,
    pub tip_message: Option<String>,
    pub confirm_message: Option<String>,

    pub theme: Theme,
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: CurrentScreen::Project,
            projects: vec![],
            tasks: vec![],
            selected_project: None,
            selected_task: None,
            selected_task_status: Some(TaskStatus::Todo),
            edit_project: false,
            add_project: false,
            edit_task: false,
            add_task: false,
            input: Input::default(),
            tip_message: None,
            confirm_message: None,
            theme: Theme::from(config::get_config().theme.as_str()),
            exit: false,
        }
    }
    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = ratatui::init();
        loop {
            terminal.draw(|f| ui::draw(f, self))?;
            if self.exit {
                break;
            }
            self.handle_event()?;
        }
        ratatui::restore();
        Ok(())
    }

    fn handle_event(&mut self) -> io::Result<()> {
        let event = event::read()?;
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match self.current_screen {
                // project screen
                CurrentScreen::Project => {
                    // tip message view
                    if self.tip_message.is_some() {
                        match key.code {
                            KeyCode::Enter => self.tip_message = None,
                            _ => {}
                        }
                        return Ok(());
                    }
                    // confirm message view
                    if self.confirm_message.is_some() {
                        match key.code {
                            KeyCode::Enter => {
                                self.confirm_message = None;
                                self.delete_project();
                            }
                            KeyCode::Esc => self.confirm_message = None,
                            _ => {}
                        }
                        return Ok(());
                    }
                    // add or edit project view
                    if self.add_project || self.edit_project {
                        match key.code {
                            KeyCode::Enter => self.submit_project(),
                            KeyCode::Esc => self.close_project_popup(),
                            _ => {
                                self.input.handle_event(&event);
                            }
                        }
                        return Ok(());
                    }

                    // projects view
                    match key.code {
                        KeyCode::Char('q') => {
                            self.exit = true;
                        }
                        KeyCode::Enter => {
                            if self.selected_project.is_some() {
                                self.current_screen = CurrentScreen::Task;
                            }
                        }
                        KeyCode::Down => self.next_project(),
                        KeyCode::Up => self.prev_project(),
                        KeyCode::Char('a') => self.add_project = true,
                        KeyCode::Char('e') => {
                            if let Some(project) = self.get_project() {
                                self.edit_project = true;
                                self.input = Input::default().with_value(project.name);
                            } else {
                                self.tip_message = Some(String::from("请先选择项目"));
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(project) = self.get_project() {
                                self.confirm_message =
                                    Some(format!("确认删除项目 {} 吗？", project.name));
                            } else {
                                self.tip_message = Some(String::from("请先选择项目"));
                            }
                        }
                        KeyCode::Char('t') => self.change_theme(),
                        _ => {}
                    }
                }
                // task screen
                CurrentScreen::Task => {
                    // tip message view
                    if self.tip_message.is_some() {
                        match key.code {
                            KeyCode::Enter => self.tip_message = None,
                            _ => {}
                        }
                        return Ok(());
                    }
                    // confirm message view
                    if self.confirm_message.is_some() {
                        match key.code {
                            KeyCode::Enter => {
                                self.confirm_message = None;
                                self.delete_task();
                            }
                            KeyCode::Esc => self.confirm_message = None,
                            _ => {}
                        }
                        return Ok(());
                    }
                    // add or edit task view
                    if self.add_task || self.edit_task {
                        match key.code {
                            KeyCode::Enter => self.submit_task(),
                            KeyCode::Esc => self.close_task_popup(),
                            _ => {
                                self.input.handle_event(&event);
                            }
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') => {
                            self.exit = true;
                        }
                        KeyCode::Down => self.next_task(),
                        KeyCode::Up => self.prev_task(),
                        KeyCode::Left => self.prev_task_status(),
                        KeyCode::Right => self.next_task_status(),
                        KeyCode::Char('a') => self.add_task = true,
                        KeyCode::Char('e') => {
                            if let Some(task) = self.get_task() {
                                self.edit_task = true;
                                self.input = Input::default().with_value(task.title);
                            } else {
                                self.tip_message = Some(String::from("请先选择任务"));
                            }
                        }
                        KeyCode::Char('s') => self.start_task(),
                        KeyCode::Char('c') => self.complete_task(),
                        KeyCode::Char('d') => {
                            if let Some(task) = self.get_task() {
                                self.confirm_message =
                                    Some(format!("确认删除任务 {} 吗？", task.title));
                            } else {
                                self.tip_message = Some(String::from("请先选择任务"));
                            }
                        }
                        KeyCode::Esc => {
                            self.current_screen = CurrentScreen::Project;
                            self.selected_task = None
                        },
                        KeyCode::Char('t') => self.change_theme(),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn close_project_popup(&mut self) {
        self.add_project = false;
        self.edit_project = false;
        self.input.reset();
    }

    fn close_task_popup(&mut self) {
        self.add_task = false;
        self.edit_task = false;
        self.input.reset();
    }

    fn next_project(&mut self) {
        let len = self.projects.len();
        if len == 0 {
            return;
        }
        if let Some(selected) = self.selected_project {
            if selected < len - 1 {
                self.selected_project = Some(selected + 1);
            } else {
                self.selected_project = Some(0);
            }
        } else {
            self.selected_project = Some(0);
        }
    }

    fn prev_project(&mut self) {
        let len = self.projects.len();
        if len == 0 {
            return;
        }
        if let Some(selected) = self.selected_project {
            if selected > 0 {
                self.selected_project = Some(selected - 1);
            } else {
                self.selected_project = Some(len - 1);
            }
        } else {
            self.selected_project = Some(len - 1);
        }
    }

    fn next_task(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        if let Some(selected) = self.selected_task {
            if selected < len - 1 {
                self.selected_task = Some(selected + 1);
            } else {
                self.selected_task = Some(0);
            }
        } else {
            self.selected_task = Some(0);
        }
    }

    fn prev_task(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        if let Some(selected) = self.selected_task {
            if selected > 0 {
                self.selected_task = Some(selected - 1);
            } else {
                self.selected_task = Some(len - 1);
            }
        } else {
            self.selected_task = Some(len - 1);
        }
    }

    fn next_task_status(&mut self) {
        self.selected_task_status = Some(self.selected_task_status.unwrap().next());
        let tasks = self.get_tasks_with_status(self.selected_task_status.unwrap());
        self.selected_task = if tasks.is_empty() { None } else { Some(0) };
    }
    fn prev_task_status(&mut self) {
        self.selected_task_status = Some(self.selected_task_status.unwrap().prev());
        let tasks = self.get_tasks_with_status(self.selected_task_status.unwrap());
        self.selected_task = if tasks.is_empty() { None } else { Some(0) };
    }

    fn submit_project(&mut self) {
        if self.add_project {
            let name = self.input.value_and_reset();
            match services::project::add(String::from(name)) {
                Ok(_) => {}
                Err(err) => {
                    self.tip_message = Some(err.to_string());
                }
            }
            self.add_project = false;
        }
        if self.edit_project {
            if let Some(project) = self.get_project() {
                services::project::edit(project.name, String::from(self.input.value_and_reset()))
                    .unwrap();
            }
            self.edit_project = false;
        }
    }

    fn submit_task(&mut self) {
        if self.add_task {
            let title = self.input.value_and_reset();
            if let Some(project) = self.get_project() {
                log::info!("current project in submit_task: {:?}", project);
                let project_id = project.id;
                services::task::add(String::from(title), project_id).unwrap();
                self.add_task = false;
            } else {
                self.tip_message = Some(String::from("请先选择项目"));
            }
        }
        if self.edit_task {
            if let Some(task) = self.get_task() {
                if let Some(id) = task.id {
                    services::task::edit(
                        id.to_string(),
                        String::from(self.input.value_and_reset()),
                    )
                    .unwrap();
                }
            }
            self.edit_task = false;
        }
    }

    fn delete_project(&mut self) {
        if let Some(project) = self.get_project() {
            services::project::delete(project.name).unwrap();
        }
    }

    fn start_task(&mut self) {
        if let Some(task) = self.get_task() {
            if let Some(id) = task.id {
                match services::task::start(id.to_string()) {
                    Ok(_) => {}
                    Err(_) => {
                        self.tip_message = Some(String::from("任务开始失败"));
                    }
                }
            }
        } else {
            self.tip_message = Some(String::from("请先选择任务"));
        }
    }

    fn complete_task(&mut self) {
        if let Some(task) = self.get_task() {
            if let Some(id) = task.id {
                match services::task::done(id.to_string()) {
                    Ok(_) => {}
                    Err(_) => {
                        self.tip_message = Some(String::from("任务完成失败"));
                    }
                }
            }
        } else {
            self.tip_message = Some(String::from("请先选择任务"));
        }
    }

    fn delete_task(&mut self) {
        if let Some(task) = self.get_task() {
            if let Some(id) = task.id {
                services::task::delete(id.to_string()).unwrap();
            }
        }
    }

    fn change_theme(&mut self) {
        match self.theme.name.as_str() {
            "dark" => {
                self.theme = Theme::light();
            }
            "light" => {
                self.theme = Theme::dark();
            }
            _ => {
                self.theme = Theme::dark();
            }
        }
    }

    pub fn query_projects(&mut self) {
        self.projects = services::project::list().unwrap();
    }
    pub fn query_tasks(&mut self) {
        if let Some(project) = self.get_project() {
            if let Some(id) = project.id {
                self.tasks = services::task::list(id as i32).unwrap();
            }
        }
    }
    pub fn get_project(&mut self) -> Option<Project> {
        if let Some(selected) = self.selected_project {
            return Some(self.projects[selected].clone());
        }
        self.tip_message = Some(String::from("请先选择项目"));
        None
    }

    pub fn get_task(&self) -> Option<Task> {
        if let Some(status) = self.selected_task_status {
            if let Some(selected) = self.selected_task {
                let tasks = self.get_tasks_with_status(status);
                return tasks.get(selected).cloned();
            }
        }
        None
    }
    pub fn get_tasks_with_status(&self, status: TaskStatus) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }
}
