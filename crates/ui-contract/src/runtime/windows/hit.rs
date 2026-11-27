use craft_ui_rt::{Pos2, Rect};
use craft_window::Button;
use craft_window::native::{CaptionEvent, Edge, Hits, Zone};
use std::sync::{Arc, Mutex};

/// An edge or corner a window resize starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResizeEdge {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bottom right corner.
    BottomRight,
}

crate::stays!(WindowHit);

/// What a part of the window does for the operating system: a press on it moves the window, resizes it from an edge,
/// or is a caption button. A component tags a part with it through `PartBuilder::exclusive`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowHit {
    /// Drags the window.
    Move,
    /// Resizes the window from the edge or corner.
    Resize(ResizeEdge),
    /// A caption button.
    Caption(Button),
}

/// Where the exclusive parts of one window lie, in points of its viewport: the window procedure answers a Windows
/// hit test from them. Read it with [`WindowHost::caption_regions`].
#[derive(Clone, Debug, PartialEq, Default)]
#[non_exhaustive]
pub struct CaptionRegions {
    /// The caption buttons.
    pub buttons: Vec<(Button, Rect)>,
    /// The edge and corner zones.
    pub edges: Vec<(ResizeEdge, Rect)>,
    /// The areas that drag the window.
    pub moves: Vec<Rect>,
    /// For each drag area, at the same index in `moves`, the parts where an in-app hit lies over its band: an
    /// interactive widget shown above the band's component, such as a bar's button or menu. A press there goes to
    /// that widget under the press-target rule (8.3), so the window does not move from that area there. A cover
    /// belongs to its own area only: a drag area shown above another (a modal's strip over the title bar) covers
    /// the lower one and still answers itself.
    pub covers: Vec<Vec<Rect>>,
}

impl CaptionRegions {
    /// What lies at `point`: a button wins over an edge, and an edge over a drag area. A drag area answers only where
    /// none of its covers lies, so its band is the top in-app hit there.
    pub fn hit(&self, point: Pos2) -> Option<WindowHit> {
        let moves =
            || self.moves.iter().zip(&self.covers).any(|(rect, covers)| rect.contains(point) && !covers.iter().any(|cover| cover.contains(point)));
        self.buttons
            .iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(button, _)| WindowHit::Caption(*button))
            .or_else(|| self.edges.iter().find(|(_, rect)| rect.contains(point)).map(|(edge, _)| WindowHit::Resize(*edge)))
            .or_else(|| moves().then_some(WindowHit::Move))
    }
}

/// What a window's last complete pass published for its hit test: the regions and the scale they are in.
#[derive(Clone, Debug)]
struct Published {
    regions: CaptionRegions,
    /// Physical pixels per point of the viewport, interface zoom included.
    pixels_per_point: f32,
}

/// The Windows hit test of one window: the window procedure asks [`HitSink::answer`] where the OS asks for a hit
/// test, and the answer comes from the exclusive parts the window's last pass declared, so the caption buttons,
/// edges and drag areas the app draws are the ones the OS acts on (snap layouts, native move and resize). The sink
/// holds the scale with the regions, so the window procedure needs nothing from egui.
///
/// Over the maximize button the pointer lies in the non-client area and the app gets no client pointer, so the
/// procedure reports the button's hover, press and release through [`Hits::caption`]; the next pass turns them into
/// the pointer events the button would have had. [`WindowHost::install_hit_sink`] makes the sink; the platform layer
/// that owns the window procedure holds it.
#[derive(Clone, Default)]
pub struct HitSink {
    published: Arc<Mutex<Option<Published>>>,
    /// The caption events the window procedure reported since the last pass.
    events: Arc<Mutex<Vec<CaptionEvent>>>,
    /// What to wake when an event arrives from the window procedure.
    wake: Option<craft_ui_rt::Waker>,
}

impl std::fmt::Debug for HitSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HitSink")
    }
}

impl HitSink {
    #[cfg_attr(not(windows), allow(dead_code))]
    /// A sink for a craft-ui-rt window: caption events wake `waker`'s loop.
    pub(crate) fn linked_rt(waker: craft_ui_rt::Waker) -> Self {
        HitSink { wake: Some(waker), ..HitSink::default() }
    }

    /// Gives the window procedure the regions of the window's last pass, at `pixels_per_point`.
    pub(crate) fn publish_regions(&self, regions: Option<CaptionRegions>, pixels_per_point: f32) {
        self.publish(regions.map(|regions| Published { regions, pixels_per_point }));
    }

    /// What lies at `pixel`, in physical pixels from the top left of the window's client area: the hit test's screen
    /// point after `ScreenToClient`. `None` where no exclusive part lies there: the window procedure leaves the hit
    /// test to the window's content.
    pub fn answer(&self, pixel: Pos2) -> Option<WindowHit> {
        let published = self.published.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Published { regions, pixels_per_point } = published.as_ref()?;
        regions.hit((pixel.to_vec2() / *pixels_per_point).to_pos2())
    }

    fn publish(&self, published: Option<Published>) {
        *self.published.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = published;
    }

    pub(crate) fn take_events(&self) -> Vec<CaptionEvent> {
        std::mem::take(&mut *self.events.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

impl Hits for HitSink {
    fn zone_at(&self, pixel: [f32; 2]) -> Option<Zone> {
        Some(match self.answer(Pos2::new(pixel[0], pixel[1]))? {
            WindowHit::Move => Zone::Move,
            WindowHit::Resize(edge) => Zone::Edge(edge_of(edge)),
            WindowHit::Caption(button) => Zone::Button(button),
        })
    }

    fn caption(&self, event: CaptionEvent) {
        self.events.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(event);
        if let Some(waker) = &self.wake {
            waker.wake();
        }
    }
}

/// The platform layer's name for `edge`.
fn edge_of(edge: ResizeEdge) -> Edge {
    match edge {
        ResizeEdge::Top => Edge::Top,
        ResizeEdge::Bottom => Edge::Bottom,
        ResizeEdge::Left => Edge::Left,
        ResizeEdge::Right => Edge::Right,
        ResizeEdge::TopLeft => Edge::TopLeft,
        ResizeEdge::TopRight => Edge::TopRight,
        ResizeEdge::BottomLeft => Edge::BottomLeft,
        ResizeEdge::BottomRight => Edge::BottomRight,
    }
}
