use std::cmp::Ordering;

use crate::task::Task;
use cursive::view::Resizable;
use cursive::{
    views::{Dialog, LinearLayout, StackView, TextView},
    Cursive,
};
use cursive_table_view::TableView;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum BasicColumn {
    Index,
    Title,
    Status,
    Created,
}

pub fn render() -> StackView {
    let main_table = new_table();
    let mut views_stack = StackView::new();
    views_stack.add_fullscreen_layer(
        LinearLayout::vertical().child(Dialog::around(main_table.full_screen()).title("Tasks")),
    );
    views_stack
}

pub(super) type TaskTable = TableView<Task, BasicColumn>;
pub(super) fn new_table() -> TaskTable {
    let table: TaskTable = TaskTable::new()
      // .on_submit(|siv: &mut Cursive, _: usize, __: usize| { draw_fullscreen_request_and_response(siv); })
      // .on_select(|siv, _, item| { draw_request_and_response(siv, item) })
      .column(
          BasicColumn::Index,
          "ID",
          |c| c.width(8).ordering(Ordering::Less)
      )
      .column(
          BasicColumn::Title,
          "Title",
          |c| c.width(10)
      )
      .column(
          BasicColumn::Status,
          "Status",
          |c| {c.width(30)}
      )
      .column(
          BasicColumn::Created,
          "Created",
          |c| {c}
      );

    return table;
}
