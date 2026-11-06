//! Pure UI behaviours (layer 1): press lifecycle, `ValueDrag`, navigation, item layouts, edit events,
//! `overlay::place`, `fit` and the other rules that read no `Context`.
//!
//! The crate depends on no egui crate: it uses `emath` and the `accesskit` data crate only.

pub mod arrangement;
pub mod axis;
pub mod canvas;
pub mod columns;
pub mod cursor;
pub mod doc_key;
pub mod eviction;
pub mod flow;
pub mod focus;
pub mod grid;
pub mod hit;
pub mod input;
pub mod intent;
pub mod item_layout;
pub mod keys;
pub mod label_column;
pub mod layout_memory;
pub mod marquee;
pub mod motion;
pub mod nav;
pub mod one_press;
pub mod part;
pub mod press;
pub mod scroll;
pub mod selection;
pub mod sense;
pub mod sizing;
pub mod value;
pub mod value_drag;
pub mod wheel;

pub use arrangement::Arrangement;
pub use axis::Axis;
pub use canvas::CanvasTransform;
pub use columns::{ColumnKey, ColumnWidth, GridSection, GridView};
pub use cursor::{Cursor, CursorBadge, CursorIcon, CursorPriority, IconId, PaintedCursor};
pub use doc_key::DocKey;
pub use flow::{Flow, Insets};
pub use focus::{CancelRule, KeyOwner, OwnerClass, PressFocus, ScopeKind};
pub use grid::{GridCursor, GridMeasure, resolve_columns};
pub use hit::{HitLevel, HitPriority, HitShape, ResolveFor, Shape, Winner, resolve, resolve_mapped};
pub use input::Input;
pub use intent::{AccessRequest, EditCommand, Granularity, Intent, MoveMods, NavMove, TextPos};
pub use item_layout::{FreeLayout, ItemLayout, SpanLayout};
pub use keys::{DropOperation, KeyRole};
pub use label_column::{LabelColumn, LabelPlacement};
pub use layout_memory::SizeTween;
pub use marquee::{MarqueeMode, MarqueeSelect};
pub use motion::MotionMode;
pub use nav::{Letters, NavOutline, NavShape, NavState, NavStep, Roving, TypeAhead};
pub use part::{PartKey, PartRef};
pub use press::{
    Buttons, MultiTouch, PassInput, PointerButton, PointerEvent, PointerKind, PointerSample, PressCancel, PressMachine, PressPhase, PressStart,
    PressState, PressStep, count_clicks,
};
pub use scroll::{ScrollPos, ScrollSpan};
pub use sizing::{ControlMetrics, Metrics, Proposal};
pub use selection::{CellSelection, Selection, SelectionChange, SelectionModel};
pub use sense::Sense;
pub use value::Value;
pub use value_drag::ValueDrag;
pub use wheel::{Wheel, WheelPhase, WheelTaker, WheelUnit};
