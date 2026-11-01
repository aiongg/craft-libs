//! [`ToolCursor`] and [`CursorHost`]: the pointer over the canvas.
//!
//! Canvas cursors move with the OS pointer, not with the canvas frame rate: a brush outline or a
//! crosshair is an OS cursor bitmap on every desktop platform, so it tracks the hardware pointer
//! instead of trailing it by render, present and compositor latency. Windows' stock crosshair
//! inverts grey into grey and vanishes over the pasteboard, so the crosshair is the library's
//! black-and-white bitmap there too. On the web and on a scaled Wayland window, where a bitmap
//! can't be shown, the outline is painted instead; the Hand and Zoom pointers fall back to the
//! system's. The tool icon and the badges are painted on the tooltip layer, above every window.
//!
//! Photoshop's painting cursors (Preferences › Cursors): Standard (the tool icon), Precise (a
//! crosshair), Normal Brush Tip (the outline where the stroke reaches 50 %) and Full Size Brush
//! Tip (where it reaches 0 %), with Show Crosshair in Brush Tip. Caps Lock swaps a tool cursor for
//! the precise one while it is down. Other tools (the selection tools, Move, Type) are the tool
//! icon with its hotspot and a small badge beside the arrow when the tool wants one
//! ([`CursorBadge`]: `+`, `−`, `×` for the selection mode; scissors and copy for the Move tool).
//!
//! The painted tool pointers (rotate, distort, skew, resize) stay in [`turn`](super::turn_cursor).

mod bitmap;
mod hand;
mod zoom;

use std::sync::Arc;

use egui::emath::GuiRounding as _;
use egui::plugin::Plugin;
use egui::{Color32, Context, CursorIcon, Event, Id, Key, LayerId, Order, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2, vec2};

use self::bitmap::Look;
use crate::icons::{self, Icon};
use crate::theme::UiTokens;

/// The icon sets draw on a box this many units on a side; a tool cursor's hotspot is in these.
const ICON_UNITS: f32 = 24.0;

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
pub struct CursorOptions {
    pub painting: PaintingCursor,
    /// Show Crosshair in Brush Tip: the outline draws its centre mark.
    pub crosshair_in_brush_tip: bool,
    /// Other Cursors: Standard or Precise for the non-painting tools.
    pub other_precise: bool,
    /// Caps Lock swaps a painting cursor for the precise one (Photoshop; PhotoCraft has it).
    pub caps_lock_precise: bool,
    /// Show Only Crosshair While Painting: during a stroke a brush outline gives way to the
    /// precise crosshair ([`BrushCursor::painting`](super::brush::BrushCursor::painting)).
    /// Photoshop's preference; PhotoCraft has it. Off by default.
    pub show_only_crosshair_while_painting: bool,
}

impl Default for CursorOptions {
    /// Photoshop's defaults: Normal Brush Tip, no crosshair, standard other cursors, Caps Lock on,
    /// the outline also while painting.
    fn default() -> Self {
        Self {
            painting: PaintingCursor::NormalBrushTip,
            crosshair_in_brush_tip: false,
            other_precise: false,
            caps_lock_precise: true,
            show_only_crosshair_while_painting: false,
        }
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
    /// The tool's icon at `hotspot` (offset from the icon's top-left in the icon set's 24-unit box),
    /// with a dark outline so it reads on any image, plus a badge beside it.
    Tool { icon: Icon, hotspot: Vec2, badge: Option<CursorBadge> },
    /// The system arrow with a badge beside it.
    Arrow { badge: Option<CursorBadge> },
    /// No pointer: the tool paints its own (a text caret, a transform box's turn arrow).
    Hidden,
}

impl ToolCursor {
    /// The cursor a brush tool shows (sizes from `style`) for `prefs` at `radius` screen points: the outline, the
    /// crosshair or the icon, swapped to the crosshair while Caps Lock is down when
    /// `caps_lock` and the preference allow. `full_size` gives the full-reach radius.
    // pc: crates/ui-egui/src/canvas/cursors.rs:36 (centre mark rule), canvas.rs:1315
    // `draw_canvas_tool_cursor` (Caps Lock and the preference).
    pub fn brush(style: &CursorStyle, prefs: &CursorOptions, radius: f32, full_size: f32, icon: Icon, caps_lock: bool) -> ToolCursor {
        let precise = ToolCursor::Crosshair { length: style.precise_reach, gap: 0.0 };
        if caps_lock && prefs.caps_lock_precise {
            return precise;
        }
        let centre = prefs.crosshair_in_brush_tip;
        match prefs.painting {
            PaintingCursor::Standard => ToolCursor::Tool { icon, hotspot: Vec2::splat(ICON_UNITS / 2.0), badge: None },
            PaintingCursor::Precise => precise,
            PaintingCursor::NormalBrushTip => ToolCursor::Circle { radius, centre },
            PaintingCursor::FullSizeBrushTip => ToolCursor::Circle { radius: full_size, centre },
        }
    }

    /// The cursor a non-painting tool shows: the icon, or the crosshair with Other Cursors
    /// Precise or Caps Lock.
    // pc: crates/ui-egui/src/canvas.rs:1315.
    pub fn tool(style: &CursorStyle, prefs: &CursorOptions, icon: Icon, hotspot: Vec2, badge: Option<CursorBadge>, caps_lock: bool) -> ToolCursor {
        // Caps Lock swaps whichever cursor Other Cursors chose for the other one.
        let swapped = caps_lock && prefs.caps_lock_precise;
        if prefs.other_precise != swapped {
            ToolCursor::Crosshair { length: style.crosshair.0, gap: style.crosshair.1 }
        } else {
            ToolCursor::Tool { icon, hotspot, badge }
        }
    }
}

/// The look of the bitmap cursors, from [`CursorHost::style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CursorStyle {
    /// Line colour of the outline, crosshair and circle: near white, whatever the theme (the
    /// pointer lies on document pixels).
    pub line: Color32,
    /// The dark halo around the line so it reads on light images: translucent black.
    pub halo: Color32,
    /// Line width in points: `cursor.stroke`.
    pub stroke: f32,
    /// Side of the badge in points: `cursor.badge`.
    pub badge: f32,
    /// Side of the tool icon in points: `cursor.icon`.
    pub icon: f32,
    /// Where the badge beside the system arrow sits, right of and below its hotspot:
    /// `cursor.badge_beside_x`, `cursor.badge_beside_y`.
    pub badge_beside: Vec2,
    /// Arm of the painting tools' precise crosshair, which has no gap: `cursor.precise_reach`.
    pub precise_reach: f32,
    /// Arm and gap of the other tools' precise crosshair: `cursor.crosshair_reach`,
    /// `cursor.crosshair_gap`.
    pub crosshair: (f32, f32),
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
        if ctx.plugin_opt::<Lifecycle>().is_none() {
            ctx.add_plugin(Lifecycle);
        }
    }

    /// Show `cursor` at `pointer` this pass. Bitmaps are rendered at the native pointer scale
    /// (`ctx.native_pixels_per_point()` times the interface scale) and cached by shape and scale;
    /// a side over the platform's limit (255 px on Linux, 2048 on Windows and macOS) falls back
    /// to painting the outline with `painter` and showing the system crosshair. Returns the
    /// `CursorIcon` set (`None` while a bitmap is pending).
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs:73 `circle`, :77 `crosshair`, :81 `outline`,
    // :254 `hand`, :275 `zoom`; canvas.rs:1360 `cursor_badge`.
    pub fn set(ctx: &Context, painter: &Painter, pointer: Pos2, cursor: &ToolCursor, style: &CursorStyle) -> CursorIcon {
        let icon = match cursor {
            ToolCursor::System(icon) => *icon,
            ToolCursor::Crosshair { length, gap } => show(ctx, painter, pointer, bitmap::Shape::Crosshair { length: *length, gap: *gap }, style),
            ToolCursor::Circle { radius, centre } => show(ctx, painter, pointer, bitmap::Shape::Circle { radius: *radius, centre: *centre }, style),
            ToolCursor::Outline { rings, centre } => {
                show(ctx, painter, pointer, bitmap::Shape::Outline { rings: rings.clone(), centre: *centre }, style)
            }
            ToolCursor::Hand { closed } => {
                let shape = bitmap::Shape::Hand { closed: *closed };
                // winit maps `Grab` and `Grabbing` to the four-arrow move cursor on Windows; the
                // other platforms draw hands.
                match system_shapes_are_wrong() {
                    true => show(ctx, painter, pointer, shape, style),
                    false if *closed => CursorIcon::Grabbing,
                    false => CursorIcon::Grab,
                }
            }
            ToolCursor::Zoom { out } => {
                let shape = bitmap::Shape::Zoom { out: *out };
                // winit maps `ZoomIn` and `ZoomOut` to the plain arrow on Windows.
                match system_shapes_are_wrong() {
                    true => show(ctx, painter, pointer, shape, style),
                    false if *out => CursorIcon::ZoomOut,
                    false => CursorIcon::ZoomIn,
                }
            }
            ToolCursor::Tool { icon, hotspot, badge } => {
                let layer = tooltip_painter(ctx);
                let rect = Rect::from_min_size(pointer - *hotspot / ICON_UNITS * style.icon, Vec2::splat(style.icon));
                paint_icon(&layer, *icon, rect, style.line, style.halo, 1.0);
                if let Some(badge) = badge {
                    Self::paint_badge(&layer, rect.right_bottom(), *badge, style);
                }
                CursorIcon::None
            }
            ToolCursor::Arrow { badge } => {
                if let Some(badge) = badge {
                    Self::paint_badge(&tooltip_painter(ctx), pointer + style.badge_beside, *badge, style);
                }
                CursorIcon::Default
            }
            ToolCursor::Hidden => CursorIcon::None,
        };
        ctx.set_cursor_icon(icon);
        icon
    }

    /// Whether the backend can show a bitmap cursor: false on the web and on Wayland without
    /// the cursor-shape protocol ([`set_wayland`](Self::set_wayland)).
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs:191 `set_wayland`, :150 `pointer_elsewhere`.
    pub fn bitmaps_supported(ctx: &Context) -> bool {
        // eframe's immediate viewports and its web integration don't upload cursor images. On
        // Wayland at any scale but 1 winit sizes the bitmap with a `wp_viewport` destination that
        // some compositors ignore on cursor surfaces, so the outline would show at the wrong size
        // and offset; nothing tells the app which compositor does what.
        let wayland = cfg!(target_os = "linux") && ctx.data(|d| d.get_temp::<bool>(wayland_id())).unwrap_or(false);
        let scaled = wayland && ctx.native_pixels_per_point().unwrap_or_else(|| ctx.pixels_per_point() / ctx.zoom_factor()) != 1.0;
        cfg!(not(target_arch = "wasm32")) && ctx.viewport_id() == egui::ViewportId::ROOT && !scaled
    }

    /// Tell the host the window runs on Wayland (the app knows from winit): bitmap cursors are
    /// painted instead of uploaded there.
    pub fn set_wayland(ctx: &Context, wayland: bool) {
        ctx.data_mut(|d| d.insert_temp(wayland_id(), wayland));
    }

    /// Whether Caps Lock is down, from the last key event seen (egui has no lock-key state).
    /// Needs [`install`](Self::install).
    // pc: crates/ui-egui/src/canvas.rs:1315 (caps lock tracking).
    pub fn caps_lock(ctx: &Context) -> bool {
        ctx.data(|d| d.get_temp::<CapsLock>(caps_lock_id())).is_some_and(|c| c.on)
    }

    pub fn style(tokens: &UiTokens) -> CursorStyle {
        let c = &tokens.component.cursor;
        CursorStyle {
            line: Color32::from_white_alpha(235),
            halo: Color32::from_black_alpha(160),
            stroke: c.stroke,
            badge: c.badge,
            icon: c.icon,
            badge_beside: vec2(c.badge_beside_x, c.badge_beside_y),
            precise_reach: c.precise_reach,
            crosshair: (c.crosshair_reach, c.crosshair_gap),
        }
    }

    /// Paint a badge glyph at `at` (the arrow's lower right) with `style`, for a host that
    /// paints its pointer whole. `at` is the badge's centre.
    // pc: crates/ui-egui/src/canvas/tool_feedback.rs:136 `badge_glyph`, :160 `draw_badge`.
    pub fn paint_badge(painter: &Painter, at: Pos2, badge: CursorBadge, style: &CursorStyle) {
        let rect = Rect::from_center_size(at, Vec2::splat(style.badge));
        let h = style.badge * GLYPH_HALF;
        let glyph = |segments: &[[Vec2; 2]]| {
            // The halo line first, then the ink line over it.
            for (width, color) in [(style.stroke + 2.0, style.halo), (style.stroke, style.line)] {
                for [a, b] in segments {
                    painter.line_segment([at + *a * h, at + *b * h], Stroke::new(width, color));
                }
            }
        };
        match badge {
            CursorBadge::Add => glyph(&[[vec2(-1.0, 0.0), vec2(1.0, 0.0)], [vec2(0.0, -1.0), vec2(0.0, 1.0)]]),
            CursorBadge::Subtract => glyph(&[[vec2(-1.0, 0.0), vec2(1.0, 0.0)]]),
            CursorBadge::Intersect => {
                let d = 0.8;
                glyph(&[[vec2(-d, -d), vec2(d, d)], [vec2(-d, d), vec2(d, -d)]]);
            }
            CursorBadge::Scissors => paint_icon(painter, Icon::Scissors, rect, style.line, style.halo, 1.0),
            CursorBadge::Corner => paint_icon(painter, Icon::Scan, rect, style.line, style.halo, 1.0),
            CursorBadge::Icon(name) => {
                if let Some(icon) = Icon::ALL.into_iter().find(|icon| icon.name() == name) {
                    paint_icon(painter, icon, rect, style.line, style.halo, 1.0);
                }
            }
            CursorBadge::Copy => {
                // A dark arrowhead behind a light one.
                let behind = rect.translate(Vec2::splat(style.badge / 3.0));
                paint_icon(painter, Icon::MousePointer2, behind, Color32::BLACK, style.line, 1.0);
                paint_icon(painter, Icon::MousePointer2, rect, style.line, style.halo, 1.0);
            }
        }
    }
}

/// Half the side of a drawn badge glyph, as a fraction of the badge's side (PhotoCraft: 3.5 of 10).
const GLYPH_HALF: f32 = 0.35;

fn tooltip_painter(ctx: &Context) -> Painter {
    ctx.layer_painter(LayerId::new(Order::Tooltip, Id::NULL.with("craft_ui::canvas::cursor")))
}

/// `icon` in `rect`, `ink` with a 1 point `halo` round it so it reads on any image.
fn paint_icon(painter: &Painter, icon: Icon, rect: Rect, ink: Color32, halo: Color32, halo_width: f32) {
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let px = icons::pixels(rect.width(), ppp);
    let texture = icons::texture(ctx, icon, px);
    let side = px as f32 / ppp;
    let rect = Rect::from_min_size(rect.min.round_to_pixels(ppp), Vec2::splat(side));
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let d = halo_width;
    for offset in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (-d, -d), (d, d), (-d, d), (d, -d)] {
        painter.image(texture, rect.translate(vec2(offset.0, offset.1)), uv, halo);
    }
    painter.image(texture, rect, uv, ink);
}

/// Whether the system's Hand and Zoom pointers are the wrong shapes here, so the host draws them.
fn system_shapes_are_wrong() -> bool {
    cfg!(target_os = "windows")
}

fn wayland_id() -> Id {
    Id::unique("craft_ui::canvas::cursor::wayland")
}

fn caps_lock_id() -> Id {
    Id::unique("craft_ui::canvas::cursor::caps_lock")
}

fn image_id() -> Id {
    Id::unique("craft_ui::canvas::cursor::image")
}

/// Caps Lock as the last key events left it, and the frame they were read in.
#[derive(Clone, Copy)]
struct CapsLock {
    on: bool,
    frame: u64,
}

/// The bitmap last rendered, so a steady pointer uploads nothing.
#[derive(Clone)]
struct Cached {
    shape: bitmap::Shape,
    scale: f32,
    look: Look,
    image: egui::CustomCursorImage,
}

/// Bitmap pixels per egui point. winit takes an `NSCursor` image's dimensions as logical points on
/// macOS, so there the bitmap scales only with the UI zoom (the monitor's density would enlarge it
/// a second time on Retina displays); Windows, X11 and Wayland take physical pixels.
fn native_scale(ctx: &Context) -> f32 {
    if cfg!(target_os = "macos") { ctx.zoom_factor() } else { ctx.pixels_per_point() }
}

fn cached_image(ctx: &Context, shape: &bitmap::Shape, scale: f32, look: Look) -> Option<egui::CustomCursorImage> {
    if let Some(cached) = ctx.data(|d| d.get_temp::<Cached>(image_id()))
        && cached.shape == *shape
        && cached.scale == scale
        && cached.look == look
    {
        return Some(cached.image);
    }
    let image = bitmap::rasterize(shape, scale, &look)?;
    ctx.data_mut(|d| d.insert_temp(image_id(), Cached { shape: shape.clone(), scale, look, image: image.clone() }));
    Some(image)
}

/// A bitmap pointer: uploaded where the backend takes one, painted otherwise.
fn show(ctx: &Context, painter: &Painter, at: Pos2, shape: bitmap::Shape, style: &CursorStyle) -> CursorIcon {
    let look = Look { line: style.line, halo: style.halo, stroke: style.stroke };
    if CursorHost::bitmaps_supported(ctx) {
        let scale = native_scale(ctx);
        if let Some(image) = cached_image(ctx, &shape, scale, look) {
            ctx.set_cursor_image(Some(image));
            return CursorIcon::None;
        }
        // An enormous tip cannot fit an OS cursor: paint its full-size outline, with an OS
        // crosshair at the hotspot so the pointer still reacts at once.
        paint(painter, at, &shape, &look);
        if let Some(image) = cached_image(ctx, &bitmap::Shape::Crosshair { length: style.precise_reach, gap: style.crosshair.1 }, scale, look) {
            ctx.set_cursor_image(Some(image));
        }
        return CursorIcon::None;
    }
    paint(painter, at, &shape, &look);
    CursorIcon::None
}

fn paint(painter: &Painter, at: Pos2, shape: &bitmap::Shape, look: &Look) {
    if bitmap::extent(shape).is_none() {
        return;
    }
    for (width, color) in [(look.stroke + 2.0, look.halo), (look.stroke, look.line)] {
        let stroke = Stroke::new(width, color);
        match shape {
            bitmap::Shape::Circle { radius, centre } => {
                painter.circle_stroke(at, *radius, stroke);
                if *centre {
                    paint_crosshair(painter, at, 3.0, 0.0, stroke);
                }
            }
            bitmap::Shape::Crosshair { length, gap } => paint_crosshair(painter, at, *length, *gap, stroke),
            bitmap::Shape::Outline { rings, centre } => {
                for ring in rings.iter().filter(|ring| ring.len() >= 2) {
                    painter.add(Shape::closed_line(ring.iter().map(|v| at + *v).collect(), stroke));
                }
                if *centre {
                    paint_crosshair(painter, at, 3.0, 0.0, stroke);
                }
            }
            // Only ever a bitmap: without one the system draws its own grab and zoom pointers.
            bitmap::Shape::Hand { .. } | bitmap::Shape::Zoom { .. } => {}
        }
    }
}

fn paint_crosshair(painter: &Painter, at: Pos2, length: f32, gap: f32, stroke: Stroke) {
    for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
        painter.line_segment([at + d * gap, at + d * length], stroke);
    }
}

/// The per-pass lifecycle: clears the bitmap request at the pass's start, reads Caps Lock, and at
/// the end swaps a bitmap request the backend can't take for the crosshair.
struct Lifecycle;

impl Plugin for Lifecycle {
    fn debug_name(&self) -> &'static str {
        "craft_ui::canvas::cursor"
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx();
        ctx.set_cursor_image(None);
        track_caps_lock(ctx);
    }

    fn on_end_pass(&mut self, ui: &mut Ui) {
        ui.ctx().output_mut(|o| {
            if o.cursor_image.is_some() {
                if o.cursor_icon == CursorIcon::None {
                    // None marks our request until all widgets have had their say. A backend that
                    // cannot upload the bitmap must still leave a visible pointer.
                    o.cursor_icon = CursorIcon::Crosshair;
                } else {
                    o.cursor_image = None;
                }
            }
        });
    }
}

/// Folds the frame's Caps Lock key events into the state, once per frame. Elsewhere the key
/// reports a press for each toggle; macOS reports it down while the lock is on.
fn track_caps_lock(ctx: &Context) {
    let frame = ctx.cumulative_frame_nr();
    let mut state = ctx.data(|d| d.get_temp::<CapsLock>(caps_lock_id())).unwrap_or(CapsLock { on: false, frame: u64::MAX });
    if state.frame == frame {
        return;
    }
    ctx.input(|i| {
        for event in &i.events {
            if let Event::Key { key: Key::CapsLock, pressed, repeat: false, .. } = event {
                if cfg!(target_os = "macos") {
                    state.on = *pressed;
                } else if *pressed {
                    state.on = !state.on;
                }
            }
        }
    });
    state.frame = frame;
    ctx.data_mut(|d| d.insert_temp(caps_lock_id(), state));
}
