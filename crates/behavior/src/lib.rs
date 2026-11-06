//! Pure UI behaviours (layer 1): press lifecycle, `ValueDrag`, navigation, item layouts, edit events,
//! `overlay::place`, `fit` and the other rules that read no `Context`.
//!
//! The crate depends on no egui crate: it uses `emath` and the `accesskit` data crate only.

pub mod canvas;
pub mod columns;
pub mod item_layout;
pub mod layout_memory;
pub mod motion;
pub mod nav;
pub mod one_press;
pub mod part;
pub mod scroll;
pub mod selection;
pub mod value;
pub mod wheel;

pub use canvas::CanvasTransform;
pub use columns::{ColumnKey, ColumnWidth, GridSection, GridView};
pub use item_layout::{FreeLayout, ItemLayout, SpanLayout};
pub use motion::MotionMode;
pub use nav::NavShape;
pub use part::{PartKey, PartRef};
pub use scroll::{ScrollPos, ScrollSpan};
pub use selection::{CellSelection, Selection, SelectionChange, SelectionModel};
pub use value::Value;
pub use wheel::{WheelPhase, WheelUnit};
