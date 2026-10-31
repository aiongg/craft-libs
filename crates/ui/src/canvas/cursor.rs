//! [`ToolCursor`] and [`CursorHost`]: the pointer over the canvas.
//!
//! Canvas cursors move with the OS pointer, not with the canvas frame rate: a brush outline, a
//! crosshair or the tool's icon are OS cursor bitmaps on every desktop platform, so they track the
//! hardware pointer instead of trailing it by render, present and compositor latency. Windows'
//! stock crosshair inverts grey into grey and vanishes over the pasteboard, so the crosshair is
//! the library's black-and-white bitmap there too. On the web and where a bitmap can't be shown
//! the host falls back to a system cursor.
//!
//! Photoshop's painting cursors (Preferences › Cursors): Standard (the tool icon), Precise (a
//! crosshair), Normal Brush Tip (the outline where the stroke reaches 50 %) and Full Size Brush
//! Tip (where it reaches 0 %), with Show Crosshair in Brush Tip. Caps Lock swaps a tool cursor for
//! the precise one while it is down. Other tools (the selection tools, Move, Type) are the tool
//! icon with its hotspot and a small badge beside the arrow when the tool wants one
//! ([`CursorBadge`]: `+`, `−`, `×` for the selection mode; scissors and copy for the Move tool).
//!
//! The painted tool pointers (rotate, distort, skew, resize) stay in [`turn`](super::turn_cursor).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use egui::{Color32, Context, CursorIcon, Painter, Pos2, Vec2};

use crate::icons::IconSize;
use crate::primitives::Icon;
use crate::theme::UiTokens;

/// Preferences › Cursors › Painting Cursors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaintingCursor {
    /// The tool's icon at its hotspot.
    Standard,
    /// A crosshair.
    Precise,
    /// The brush tip outline at the 50 % reach.
    #[default]
    NormalBrushTip,
    /// The brush tip outline at the full reach.
    FullSizeBrushTip,
}

/// Preferences › Cursors, as the app stores them; the host reads them each pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CursorPrefs {
    pub painting: PaintingCursor,
    /// Show Crosshair in Brush Tip: the outline draws its centre mark.
    pub crosshair_in_brush_tip: bool,
    /// Other Cursors: Standard or Precise for the non-painting tools.
    pub other_precise: bool,
    /// Caps Lock swaps a painting cursor for the precise one (Photoshop; PhotoCraft has it).
    pub caps_lock_precise: bool,
}

impl Default for CursorPrefs {
    /// Photoshop's defaults: Normal Brush Tip, no crosshair, standard other cursors, Caps Lock on.
    fn default() -> Self {
        Self { painting: PaintingCursor::NormalBrushTip, crosshair_in_brush_tip: false, other_precise: false, caps_lock_precise: true }
    }
}

/// A small mark beside the arrow saying what a press does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CursorBadge {
    /// Selection add (`+`).
    Add,
    /// Selection subtract (`−`).
    Subtract,
    /// Selection intersect (`×`).
    Intersect,
    /// The Move tool cuts the selection out.
    Scissors,
    /// Alt-drag duplicates.
    Copy,
    /// The corner-radius cursor's corner mark (Photoshop's rounded rectangle).
    Corner,
    /// A named icon from the set, at the badge size.
    Icon(&'static str),
}

/// What the pointer shows over the canvas. Sizes are screen points at the pointer; bitmaps grow
/// with the interface scale (decision Q22).
#[derive(Clone, Debug)]
pub enum ToolCursor {
    /// An OS cursor.
    System(CursorIcon),
    /// The black-and-white crosshair bitmap (Precise), `length` each arm, `gap` around the centre.
    Crosshair { length: f32, gap: f32 },
    /// A circle of `radius` (a round brush tip at this zoom), with a centre mark when `centre`.
    Circle { radius: f32, centre: bool },
    /// Closed rings of offsets from the hotspot (a shaped brush tip), with a centre mark.
    Outline { rings: Arc<[Vec<Vec2>]>, centre: bool },
    /// The Hand tool: open, or the fist while dragging.
    Hand { closed: bool },
    /// The Zoom tool's magnifier, with a minus when it zooms out.
    Zoom { out: bool },
    /// The tool's icon at `hotspot` (offset from the icon's top-left in icon units), with a dark
    /// outline so it reads on any image, plus a badge beside it.
    Tool { icon: Icon, hotspot: Vec2, badge: Option<CursorBadge> },
    /// The system arrow with a badge beside it.
    Arrow { badge: Option<CursorBadge> },
    /// No pointer: the tool paints its own (a text caret, a transform box's turn arrow).
    Hidden,
}

impl ToolCursor {
    /// The cursor a brush tool shows for `prefs` at `radius` screen points: the outline, the
    /// crosshair or the icon, swapped to the crosshair while Caps Lock is down when
    /// `caps_lock` and the preference allow. `full_size` gives the full-reach radius.
    // pc: crates/ui-egui/src/canvas/cursors.rs:36 (centre mark rule), canvas.rs:1315
    // `draw_canvas_tool_cursor` (Caps Lock and the preference).
    pub fn brush(prefs: &CursorPrefs, radius: f32, full_size: f32, icon: Icon, caps_lock: bool) -> ToolCursor {
        todo!()
    }

    /// The cursor a non-painting tool shows: the icon, or the crosshair with Other Cursors
    /// Precise or Caps Lock.
    // pc: crates/ui-egui/src/canvas.rs:1315.
    pub fn tool(prefs: &CursorPrefs, icon: Icon, hotspot: Vec2, badge: Option<CursorBadge>, caps_lock: bool) -> ToolCursor {
        todo!()
    }
}

/// The look of the bitmap cursors, from [`CursorHost::style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CursorStyle {
    /// Line colour of the outline, crosshair and circle: white.
    pub line: Color32,
    /// The dark halo around the line so it reads on light images: black.
    pub halo: Color32,
    /// Line width in points: `cursor.stroke`.
    pub stroke: f32,
    /// Side of the badge in points: `cursor.badge`.
    pub badge: f32,
    /// Side of the tool icon in points: `cursor.icon` (the icon set's `IconSize::Cursor`).
    pub icon: IconSize,
}

/// Sets the canvas cursor once per pass and clears it at the next pass's start, so a cursor set
/// over the canvas never sticks to a document that closed or a pointer that left. Installed once
/// as an egui plugin by [`CursorHost::install`].
///
/// A bitmap request is marked with `CursorIcon::None` until every widget has had its say; at
/// the pass's end a backend that can't show the bitmap gets the crosshair instead, and a later
/// widget's system cursor wins over the canvas's bitmap.
pub struct CursorHost;

impl CursorHost {
    /// Install the per-pass lifecycle in `ctx` (once; a second call is a no-op). Apps that
    /// show a canvas call it at startup; stories and tests through the gallery.
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs:47 `CursorLifecycle`.
    pub fn install(ctx: &Context) {
        todo!()
    }

    /// Show `cursor` at `pointer` this pass. Bitmaps are rendered at the native pointer scale
    /// (`ctx.native_pixels_per_point()` times the interface scale) and cached by shape and scale;
    /// a side over the platform's limit (255 px on Linux, 2048 on Windows and macOS) falls back
    /// to painting the outline with `painter` and showing the system crosshair. Returns the
    /// `CursorIcon` set (`None` while a bitmap is pending).
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs:73 `circle`, :77 `crosshair`, :81 `outline`,
    // :254 `hand`, :275 `zoom`; canvas.rs:1360 `cursor_badge`.
    pub fn set(ctx: &Context, painter: &Painter, pointer: Pos2, cursor: &ToolCursor, style: &CursorStyle) -> CursorIcon {
        todo!()
    }

    /// Whether the backend can show a bitmap cursor: false on the web and on Wayland without
    /// the cursor-shape protocol ([`set_wayland`](Self::set_wayland)).
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs:191 `set_wayland`, :150 `pointer_elsewhere`.
    pub fn bitmaps_supported(ctx: &Context) -> bool {
        todo!()
    }

    /// Tell the host the window runs on Wayland (the app knows from winit): bitmap cursors are
    /// painted instead of uploaded there.
    pub fn set_wayland(ctx: &Context, wayland: bool) {
        todo!()
    }

    /// Whether Caps Lock is down, from the last key event seen (egui has no lock-key state).
    // pc: crates/ui-egui/src/canvas.rs:1315 (caps lock tracking).
    pub fn caps_lock(ctx: &Context) -> bool {
        todo!()
    }

    pub fn style(tokens: &UiTokens) -> CursorStyle {
        // new: cursor.stroke, cursor.badge, cursor.icon are proposed tokens (spec: Tokens); until
        // they land the values are PhotoCraft's (1 pt, 10 pt, 16 pt).
        todo!()
    }

    /// Paint a badge glyph at `at` (the arrow's lower right) with `style`, for a host that
    /// paints its pointer whole.
    // pc: crates/ui-egui/src/canvas/tool_feedback.rs:136 `badge_glyph`, :160 `draw_badge`.
    pub fn paint_badge(painter: &Painter, at: Pos2, badge: CursorBadge, style: &CursorStyle) {
        todo!()
    }
}
