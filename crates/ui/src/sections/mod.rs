//! Sections and title rows: `TitleRow`, `FooterBar`, `Section` and `GroupBox`.

mod footer;
#[cfg(feature = "testing")]
mod stories;
mod title_row;
mod words;

pub use footer::FooterBar;
pub use title_row::{HeaderAction, HeaderStrength, TitleOutput, TitleRow, TitleRowStyle};
pub use words::SectionWords;
