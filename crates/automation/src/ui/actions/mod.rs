//! The Actions panel (F13.06): sets, actions and steps as a tree, the include and dialog columns,
//! rename, drag to move, Button Mode, the footer and the menus. It runs nothing; it reports
//! events (`ActionsPanel`, `ActionsHost`, `ActionsState`, `ActionsEvent`).

mod footer;
mod labels;
mod menu;
mod panel;
mod rows;
pub(crate) mod stories;
mod style;
