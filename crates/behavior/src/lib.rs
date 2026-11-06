//! Pure UI behaviours (layer 1): press lifecycle, `ValueDrag`, navigation, item layouts, edit events,
//! `overlay::place`, `fit` and the other rules that read no `Context`.
//!
//! The crate depends on no egui crate: it uses `emath` and the `accesskit` data crate only.

pub mod axis;
pub mod canvas;
pub mod columns;
pub mod cursor;
pub mod eviction;
pub mod hit;
pub mod item_layout;
pub mod layout_memory;
pub mod marquee;
pub mod mods;
pub mod motion;
pub mod nav;
pub mod one_press;
pub mod part;
pub mod press;
pub mod scroll;
pub mod selection;
pub mod sense;
pub mod value;
pub mod value_drag;
pub mod wheel;

pub use axis::Axis;
pub use canvas::CanvasTransform;
pub use columns::{ColumnKey, ColumnWidth, GridSection, GridView};
pub use cursor::{Cursor, CursorBadge, CursorIcon, CursorPriority, IconId, PaintedCursor};
pub use hit::{HitLevel, HitPriority, HitShape, PressFocus, ResolveFor, Shape, Winner, resolve, resolve_mapped};
pub use item_layout::{FreeLayout, ItemLayout, SpanLayout};
pub use marquee::{MarqueeMode, MarqueeSelect};
pub use mods::Mods;
pub use motion::MotionMode;
pub use nav::NavShape;
pub use part::{PartKey, PartRef};
pub use press::{
    Buttons, MultiTouch, PassInput, PointerButton, PointerEvent, PointerKind, PointerSample, PressCancel, PressMachine, PressPhase, PressStart,
    PressState, PressStep, count_clicks,
};
pub use scroll::{ScrollPos, ScrollSpan};
pub use selection::{CellSelection, Selection, SelectionChange, SelectionModel};
pub use sense::Sense;
pub use value::Value;
pub use value_drag::ValueDrag;
pub use wheel::{Wheel, WheelPhase, WheelTaker, WheelUnit};
