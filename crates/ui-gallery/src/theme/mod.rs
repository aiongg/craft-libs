//! The theme workspace: the panels that explain a theme and its appearance. The host holds one [`ThemeWorkspace`],
//! refreshes it for the theme on show and passes what it holds to the panels, which report picks and edits back in
//! their outputs.

mod browser;
mod chip;
mod detail;
mod inspector;
mod layer;
mod problems;
mod rows;
mod subject;
mod words;

pub use browser::{BrowserOutput, TokenBrowser};
pub use detail::TokenDetail;
pub use inspector::AccessInspector;
pub use layer::{AppLayerEditor, LayerOutput, parse_sizes};
pub use problems::{Problems, ProblemsOutput};
pub use rows::Row;
pub use subject::{Pick, ThemeSubject, ThemeWorkspace};
pub use words::summary;
