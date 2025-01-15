use cursive::{Cursive, CursiveExt};
use kanban::{add_task_dialog, KanbanBoard};

mod kanban;
pub fn start(mut siv: Cursive) {
    siv.add_global_callback('q', |s| s.quit());
    siv.add_global_callback('a', |s| add_task_dialog(s));

    let mut board = KanbanBoard::new();
    siv.set_user_data(board.clone());
    siv.add_layer(board.render());

    siv.run();
}
