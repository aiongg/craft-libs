//! Pure UI behaviours (layer 1): press lifecycle, `ValueDrag`, navigation, item layouts, edit events,
//! `overlay::place`, `fit` and the other rules that read no `Context`.
//!
//! The crate depends on no egui crate: it uses `emath` and the `accesskit` data crate only.

pub mod arrangement;
pub mod canvas;
pub mod columns;
pub mod doc_key;
pub mod eviction;
pub mod flow;
pub mod focus;
pub mod grid;
pub mod input;
pub mod intent;
pub mod item_layout;
pub mod keys;
pub mod label_column;
pub mod layout_memory;
pub mod motion;
pub mod nav;
pub mod one_press;
pub mod part;
pub mod scroll;
pub mod selection;
pub mod sizing;
pub mod value;
pub mod wheel;

pub use arrangement::Arrangement;
pub use canvas::CanvasTransform;
pub use columns::{ColumnKey, ColumnWidth, GridSection, GridView};
pub use doc_key::DocKey;
pub use flow::{Flow, Insets};
pub use focus::{CancelRule, KeyOwner, OwnerClass, ScopeKind};
pub use grid::{GridCursor, GridMeasure, resolve_columns};
pub use input::Input;
pub use intent::{AccessRequest, EditCommand, Granularity, Intent, MoveMods, NavMove, TextPos};
pub use item_layout::{FreeLayout, ItemLayout, SpanLayout};
pub use keys::{DropOperation, KeyRole, Os};
pub use label_column::{LabelColumn, LabelPlacement};
pub use layout_memory::SizeTween;
pub use motion::MotionMode;
pub use nav::{Letters, NavOutline, NavShape, NavState, NavStep, Roving, TypeAhead};
pub use part::{PartKey, PartRef};
pub use scroll::{ScrollPos, ScrollSpan};
pub use sizing::{Metrics, Proposal};
pub use selection::{CellSelection, Selection, SelectionChange, SelectionModel};
pub use value::Value;
pub use wheel::{WheelPhase, WheelUnit};
