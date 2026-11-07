//! Sections and title rows: `TitleRow`, `FooterBar`, `Section` and `GroupBox`.

mod apply;
pub(crate) mod footer;
mod group_box;
#[cfg(feature = "testing")]
mod sample;
mod section;
mod title_row;
mod words;

pub use apply::{ApplyRow, ApplyRowStyle};
pub use footer::{DialogRole, FooterAction, FooterBar, FooterButton};
pub use group_box::{GroupBox, GroupBoxStyle};
pub use section::{Section, SectionOutput, SectionStyle};
pub use title_row::{HeaderAction, HeaderStrength, TitleOutput, TitleRow, TitleRowStyle};
pub use words::SectionWords;
