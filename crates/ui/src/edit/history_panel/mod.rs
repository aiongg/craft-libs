//! The History panel and the undo list: snapshot rows above the state rows, the source column,
//! drag to trash, rename in place, the footer and the panel menu (`HistoryPanel`, `UndoList`).

mod footer;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod history_panel_tests;
mod labels;
mod menu;
mod panel;
mod state;
pub(crate) mod stories;
mod style;
mod undo_list;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod undo_list_tests;
