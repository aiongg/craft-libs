//! `WindowEdges`: invisible zones along the edges and corners of an undecorated window. A press that drags on one
//! hands the resize to the operating system.

use crate::contract::{Common, Component, Cx, MeasureCx, NoValue, Outcome, PartsCx, Pointer as _, Props as _, Role};
use crate::runtime::windows::{ResizeEdge, WindowAction, WindowHost};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{CursorIcon, HitLevel, HitPriority, Metrics, PartKey, PointerButton, PressPhase, Sense};
use craft_host::target::{Caps, WindowChrome, WindowState};
use egui::{Pos2, Rect, Response, Ui, Vec2, ViewportId};

#[cfg(feature = "testing")]
mod stories;

/// The zones: one part for each piece of each zone left after the holes are cut.
const ZONE: PartKey = PartKey::new("zone");

/// The keyboard path of a zone.
const REASON: &str = "the operating system resizes an undecorated window";

/// The style of the edge zones.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct WindowEdgesStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The thickness of an edge zone.
    edge: f32,
    /// The side of a corner zone, larger than the edge so a corner is easy to hit.
    corner: f32,
}

/// What the zones report.
#[derive(Debug)]
#[non_exhaustive]
pub struct WindowEdgesOutput {
    /// The response of the zones' root, which has no rect of its own.
    pub response: Response,
    /// The edge a resize started from this pass.
    pub started: Option<ResizeEdge>,
}

impl Outcome for WindowEdgesOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// Eight zones along the edges and corners of the window of one viewport: a press that drags on one starts an
/// operating-system resize from that edge. The zones lie above everything shown before them in the viewport's
/// root `Ui`, which must cover the window (its clip rect is the window), and cut out `holes`, such as the caption
/// buttons at a corner.
///
/// - No zones while the window is maximized or full screen, when the operating system or the window manager draws
///   the frame, or when the window procedure answers the hit test itself (`CaptionRegions::native_hit_test` on
///   Windows).
/// - A corner wins over the edges it overlaps.
/// - The zones take no key focus: the operating system provides the keyboard path to size a window.
#[must_use = "show the zones with `show`"]
#[derive(crate::contract::Component)]
pub struct WindowEdges {
    #[common]
    common: Common,
    viewport: ViewportId,
    holes: Vec<Rect>,
    /// The edge each zone piece resizes from, in the order the pieces are declared.
    pieces: Vec<ResizeEdge>,
    started: Option<ResizeEdge>,
}

impl WindowEdges {
    /// The zones of the window of `viewport`.
    pub fn new(viewport: ViewportId) -> Self {
        WindowEdges { common: Common::new(), viewport, holes: Vec::new(), pieces: Vec::new(), started: None }.id_salt(("window_edges", viewport))
    }

    /// Cuts `holes` out of the zones: places that keep their own press, such as the caption buttons.
    pub fn holes(mut self, holes: &[Rect]) -> Self {
        self.holes = holes.to_vec();
        self
    }

    /// Shows the zones. Returns the edge a resize started from in this pass.
    pub fn show(self, ui: &mut Ui) -> Option<ResizeEdge> {
        let at = Rect::from_min_size(ui.ctx().content_rect().min, Vec2::ZERO);
        <Self as crate::contract::Show>::show_at(self, ui, at).started
    }

    /// Whether the window has zones now.
    fn on(&self, host: &WindowHost) -> bool {
        let shell = crate::runtime::windows::shell_of(&host.ctx, self.viewport);
        let framed_by_app = shell.window.chrome == WindowChrome::Integrated;
        let sized_by_os = matches!(shell.window.state, WindowState::Maximized | WindowState::Fullscreen);
        let answered =
            shell.caps.contains(Caps::OS_RESIZES_UNDECORATED) && host.caption_regions(self.viewport).is_some_and(|regions| regions.native_hit_test);
        framed_by_app && !sized_by_os && !answered
    }
}

/// The eight zones of `window`: the edge and the cursor of each, and whether it is a corner. Edges use the
/// one-sided cursors, which every cursor theme draws; corners keep the diagonal ones.
fn zones(window: Rect, edge: f32, corner: f32) -> [(Rect, ResizeEdge, CursorIcon, bool); 8] {
    let square = |at: Pos2| Rect::from_min_size(at, Vec2::splat(corner));
    [
        (Rect::from_min_max(window.left_top(), Pos2::new(window.right(), window.top() + edge)), ResizeEdge::Top, CursorIcon::ResizeNorth, false),
        (
            Rect::from_min_max(Pos2::new(window.left(), window.bottom() - edge), window.right_bottom()),
            ResizeEdge::Bottom,
            CursorIcon::ResizeSouth,
            false,
        ),
        (Rect::from_min_max(window.left_top(), Pos2::new(window.left() + edge, window.bottom())), ResizeEdge::Left, CursorIcon::ResizeWest, false),
        (Rect::from_min_max(Pos2::new(window.right() - edge, window.top()), window.right_bottom()), ResizeEdge::Right, CursorIcon::ResizeEast, false),
        (square(window.left_top()), ResizeEdge::TopLeft, CursorIcon::ResizeNwSe, true),
        (square(window.right_top() - Vec2::new(corner, 0.0)), ResizeEdge::TopRight, CursorIcon::ResizeNeSw, true),
        (square(window.left_bottom() - Vec2::new(0.0, corner)), ResizeEdge::BottomLeft, CursorIcon::ResizeNeSw, true),
        (square(window.right_bottom() - Vec2::splat(corner)), ResizeEdge::BottomRight, CursorIcon::ResizeNwSe, true),
    ]
}

/// The parts of `rect` outside every rect of `holes`.
fn subtract(rect: Rect, holes: &[Rect]) -> Vec<Rect> {
    let mut pieces = vec![rect];
    for hole in holes {
        pieces = pieces
            .into_iter()
            .flat_map(|piece| {
                let cut = piece.intersect(*hole);
                if !cut.is_positive() {
                    return vec![piece];
                }
                [
                    Rect::from_min_max(piece.min, Pos2::new(piece.max.x, cut.min.y)),
                    Rect::from_min_max(Pos2::new(piece.min.x, cut.max.y), piece.max),
                    Rect::from_min_max(Pos2::new(piece.min.x, cut.min.y), Pos2::new(cut.min.x, cut.max.y)),
                    Rect::from_min_max(Pos2::new(cut.max.x, cut.min.y), Pos2::new(piece.max.x, cut.max.y)),
                ]
                .into_iter()
                .filter(Rect::is_positive)
                .collect()
            })
            .collect();
    }
    pieces
}

impl Component for WindowEdges {
    const ROLE: Role = Role::None;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    const SHOWS_LABEL: bool = false;
    type Style = WindowEdgesStyle;
    type Output = WindowEdgesOutput;

    fn style(cx: &StyleCx<'_>, _: WindowEdgesStyleKey) -> WindowEdgesStyle {
        WindowEdgesStyle { metrics: Metrics::new(), edge: cx.tokens.size.handle.sm, corner: cx.tokens.size.handle.md }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// Every zone reaches outside the root's empty rect, so each is its own widget above the viewport's panels.
    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        self.pieces.clear();
        let ctx = cx.ui.ctx().clone();
        if !self.on(&WindowHost::of(&ctx)) {
            return;
        }
        for (zone, edge, cursor, corner) in zones(ctx.content_rect(), cx.style.edge, cx.style.corner) {
            for piece in subtract(zone, &self.holes) {
                let index = self.pieces.len() as u64;
                self.pieces.push(edge);
                cx.part(ZONE.at(index), piece)
                    .sense(Sense::DRAG)
                    .priority(HitPriority::new(HitLevel::Edge, u8::from(corner)))
                    .exclusive()
                    .cursor(cursor)
                    .pointer_only(REASON);
            }
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.started = None;
        for (index, edge) in self.pieces.iter().enumerate() {
            if let PressPhase::DragStart { start, .. } = cx.part(ZONE.at(index as u64)).press.phase
                && start.button == PointerButton::Primary
                && WindowHost::of(&cx.core.ctx).act(self.viewport, WindowAction::StartResize(*edge))
            {
                self.started = Some(*edge);
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> WindowEdgesOutput {
        WindowEdgesOutput { response: cx.response.clone(), started: self.started }
    }
}
