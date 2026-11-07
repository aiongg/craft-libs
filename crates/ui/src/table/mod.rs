//! `Table` and `MetadataTable`, as `List` with columns.

mod sort;
#[cfg(feature = "testing")]
mod stories;
mod view;
mod words;

pub use sort::sort_rows;
pub use view::{Table, TableStyle};
