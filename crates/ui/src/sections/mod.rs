//! Sections and title rows: `TitleRow`, `FooterBar`, `Section` and `GroupBox`.

mod footer;
mod group_box;
#[cfg(feature = "testing")]
mod stories;
mod title_row;
mod words;

pub use footer::{DialogRole, FooterAction, FooterBar, FooterButton};
pub use group_box::{GroupBox, GroupBoxStyle};
pub use title_row::{HeaderAction, HeaderStrength, TitleOutput, TitleRow, TitleRowStyle};
pub use words::SectionWords;
