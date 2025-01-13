use cursive::view::{Nameable, Resizable};
use cursive::{
    views::{Button, FixedLayout, LinearLayout, SelectView, StackView},
    Cursive, CursiveExt, Rect, View,
};

mod task_table;
pub use task_table::*;

pub fn render(mut siv: Cursive) {
    siv.add_global_callback('q', |s| s.quit());

    let main_table = task_table::render();
    let base_layout =
        LinearLayout::vertical().child(main_table.with_name("task-table").full_width());

    siv.add_fullscreen_layer(base_layout);
    siv.run();
}
