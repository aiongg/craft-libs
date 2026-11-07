//! Pure UI behaviours (layer 1): press lifecycle, `ValueDrag`, navigation, item layouts, edit events,
//! `overlay::place`, `fit` and the other rules that read no `Context`.
//!
//! The crate depends on no egui crate: it uses `emath` and the `accesskit` data crate only.

pub mod arrangement;
pub mod canvas;
pub mod codec;
pub mod columns;
pub mod command;
pub mod cursor;
pub mod doc_key;
pub mod dock;
pub mod edit_history;
pub mod editor;
pub mod eviction;
pub mod flow;
pub mod focus;
pub mod grid;
pub mod hit;
pub mod input;
pub mod intent;
pub mod item_layout;
pub mod item_source;
pub mod keys;
pub mod label_column;
pub mod layout_memory;
pub mod marquee;
pub mod motion;
pub mod nav;
pub mod one_press;
pub mod overlay;
pub mod part;
pub mod press;
pub mod quantity;
pub mod role_rule;
pub mod scroll;
pub mod selection;
pub mod sense;
pub mod sizing;
pub mod typing;
pub mod value;
pub mod value_drag;
pub mod wheel;

pub use accesskit::{Action as AccessAction, ActionData as AccessData};
pub use arrangement::Arrangement;
pub use canvas::CanvasTransform;
pub use codec::{Caret, Codec, NotAValue, NumberCodec, PlainText, decimals_for_step};
pub use columns::{ColumnKey, ColumnWidth, GridSection, GridView};
pub use command::{CommandFlags, pick_instance, resolve_flags};
pub use cursor::{Cursor, CursorBadge, CursorIcon, CursorPriority, IconId, PaintedCursor};
pub use doc_key::DocKey;
pub use dock::{DockHandle, DockSide, DockTree, GroupId};
pub use edit_history::EditHistory;
pub use editor::EditorInput;
pub use flow::{Axis, Flow, Insets};
pub use focus::{CancelRule, KeyOwner, OwnerClass, PressFocus, ScopeKind};
pub use grid::{GridCursor, GridMeasure, resolve_columns};
pub use hit::{HitLevel, HitPriority, HitShape, ResolveFor, Shape, Winner, resolve, resolve_mapped};
pub use input::Input;
pub use intent::{AccessRequest, EditCommand, Granularity, Intent, MoveMods, NavMove, TextPos};
pub use item_layout::{FreeLayout, ItemLayout, SheetLayout, SpanLayout, TileLayout};
pub use item_source::{DisplayOrder, ItemSource, Keyed, Outline, SizeHint, keyed};
pub use keys::{DropOperation, KeyRole};
pub use label_column::{LabelColumn, LabelPlacement};
pub use layout_memory::SizeTween;
pub use marquee::{MarqueeMode, MarqueeSelect};
pub use motion::{MotionKind, MotionMode, Playback};
pub use nav::{Letters, NavOutline, NavShape, NavState, NavStep, Roving, TypeAhead, wheel_step};
pub use part::{PartKey, PartRef};
pub use press::{
    Buttons, MultiTouch, PassInput, PointerButton, PointerEvent, PointerKind, PointerSample, PressCancel, PressMachine, PressPhase, PressStart,
    PressState, PressStep, count_clicks,
};
pub use quantity::Reader;
pub use role_rule::{GridFacts, RoleRule, is_checkable, is_selectable, role_violations};
pub use scroll::{ScrollPos, ScrollSpan};
pub use selection::{CellAt, CellRange, CellSelection, Selection, SelectionChange, SelectionModel};
pub use sense::Sense;
pub use sizing::{ControlMetrics, Metrics, Proposal};
pub use typing::{EditRun, Ended, EscapeRule, Escaped, Typed, TypingRun};
pub use value::{Applied, CancelReason, EditEvent, EditPolicy, Gesture, GestureId, GesturePhase, Mixed, Validity, Value};
pub use value_drag::{DragWarp, ValueDrag};
pub use wheel::{Wheel, WheelNotch, WheelPhase, WheelTaker, WheelUnit};
