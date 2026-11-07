//! Preset patterns on collections and overlays: `PresetBrowser`, `PresetPicker`, `PresetFooter` and `NamedPresetRow`.

mod words;

// Preset data, footer, files and panel menu.
mod event;
mod files;
mod footer;
mod model;
mod panel_menu;
mod state;

pub use event::{PresetEvent, PresetOutput};
pub use files::{PresetExport, PresetFileEvent, PresetFileFailure, PresetFileRequest, PresetFiles};
pub use footer::{PresetFooter, PresetFooterEvent, PresetFooterOutput};
pub use model::{
    CellShape, GroupPath, PresetChange, PresetError, PresetGroup, PresetId, PresetItem, PresetKey, PresetKind, PresetLibrary, PresetRow, Preview,
    PreviewPaint, ThumbInfo, children_end, has_captions, nest_allowed,
};
pub use panel_menu::{PresetCommands, PresetMenuAction, PresetPanelMenu};
pub use state::{DefaultOpen, PresetState, PresetView, default_is_open};
pub use words::PresetWords;
