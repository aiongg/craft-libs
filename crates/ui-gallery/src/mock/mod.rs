//! The mock editor window: the shell, title bar, rails, dock, document tabs, canvas and dialogs of a document editor,
//! composed from library components only.

mod editor;
mod model;
mod panels;
mod stories;

pub use editor::EditorMock;
pub use model::{Document, Layer, MockState, Panel, Tool};
