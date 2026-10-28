//! Lists: rows for panels that list things (layers, history, channels).
//!
//! - [`ListRow`]: one row. Optional toggle columns (eye, lock) with a sweep gesture, an icon or
//!   a framed thumbnail, a label with an optional detail line, and a trailing slot whose optional
//!   items drop out before the label gets too narrow. A row is drawn from plain inputs (selected,
//!   focused, dimmed, highlighted, renaming, drop marker); it keeps no selection or order of its
//!   own, so a list drives it. It reports pointer input on its response and the edits of its toggle
//!   columns and rename field as [`RowEvent`]s.
//!
//! Rows draw with the `list.*` component tokens; [`ListRow::style`] is a pure function of
//! [`UiTokens`](crate::theme::UiTokens), dumped under `crates/ui/tests/dumps/components/`.

mod rename;
mod row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod row_tests;
mod toggle;

pub use row::COMPONENTS;
pub(crate) use row::stories;
pub use row::{DropMark, ListRow, ListRowStyle, RenameEnd, RowEvent, RowResponse, ThumbSize, TrailingItem};
pub use toggle::{ToggleColumn, ToggleEdit};
