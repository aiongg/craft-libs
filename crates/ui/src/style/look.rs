//! Style parts: surface, text, icon and focus looks, the two colour types `Draw` accepts, and `ComponentStyle`.

use craft_behavior::Metrics;
use craft_i18n::Direction;
use craft_theme::{Mix, Stateful, TypeRole, Variant};
use egui::{Color32, CornerRadius, Stroke};

/// A colour a style builder picked from tokens. `Draw` accepts only this or `DataColor`. It has no
/// `From<Color32>`: the one constructor is `StyleCx::color`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleColor(pub(crate) Color32);

impl StyleColor {
    /// The colour as egui paints it.
    pub(crate) fn egui(self) -> Color32 {
        self.0
    }
}

impl Mix for StyleColor {
    fn mix(self, to: StyleColor, t: f32) -> StyleColor {
        StyleColor(self.0.mix(to.0, t))
    }
}

/// A colour from data (a swatch, a layer label dot).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataColor(pub(crate) Color32);

impl DataColor {
    /// A data colour.
    pub const fn new(color: Color32) -> Self {
        DataColor(color)
    }
}

/// The surface a container or part sits on, which picks fill, edge, shadow and ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SurfaceKind {
    /// The window behind everything.
    Background,
    /// A canvas pasteboard.
    Canvas,
    /// Bars, title bars and rails.
    Chrome,
    /// Docked panels.
    Panel,
    /// Raised controls and cards.
    Raised,
    /// A raised surface while pressed.
    Pressed,
    /// Sunken areas.
    Sunken,
    /// Text and value fields.
    Well,
    /// A knob that rides on a track.
    Knob,
    /// Popups and menus.
    Popup,
    /// Dialogs.
    Dialog,
    /// Floating windows and panels.
    Floating,
    /// The scrim behind a modal layer.
    Scrim,
    /// An outlined box without fill.
    Frame,
}

impl SurfaceKind {
    /// The ink text and icons read on this surface.
    pub const fn ink(self) -> super::Ink {
        match self {
            SurfaceKind::Scrim => super::Ink::OnDark,
            _ => super::Ink::OnPanel,
        }
    }
}

/// Which corners of a surface are rounded. `Start` and `End` mirror under right-to-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Corners {
    /// All four.
    #[default]
    All,
    /// The top two.
    Top,
    /// The two on the reading start side.
    Start,
    /// The two on the reading end side.
    End,
    /// None.
    None,
}

impl Corners {
    /// `radius` with the corners this value leaves square zeroed.
    pub fn apply(self, radius: CornerRadius, direction: Direction) -> CornerRadius {
        let left = (direction == Direction::Ltr) == (self == Corners::Start);
        match self {
            Corners::All => radius,
            Corners::None => CornerRadius::ZERO,
            Corners::Top => CornerRadius { sw: 0, se: 0, ..radius },
            Corners::Start | Corners::End if left => CornerRadius { ne: 0, se: 0, ..radius },
            Corners::Start | Corners::End => CornerRadius { nw: 0, sw: 0, ..radius },
        }
    }
}

/// A component's or part's surface.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SurfaceLook {
    /// The surface kind; `None` draws no surface (ghost buttons, list rows).
    pub kind: Option<SurfaceKind>,
    /// The kind while pressed; `None` keeps `kind`.
    pub kind_down: Option<SurfaceKind>,
    /// The state fill, which replaces the kind's fill.
    pub fill: Stateful<StyleColor>,
    /// The state border.
    pub border: Option<Stateful<StyleColor>>,
    /// The border width.
    pub border_width: f32,
    /// The corner radius.
    pub radius: CornerRadius,
    /// Which corners are rounded.
    pub corners: Corners,
}

impl SurfaceLook {
    /// A look with `fill` and no kind, border or radius.
    pub fn new(fill: Stateful<StyleColor>) -> Self {
        SurfaceLook { kind: None, kind_down: None, fill, border: None, border_width: 0.0, radius: CornerRadius::ZERO, corners: Corners::All }
    }

    /// The surface kind in `state`: `kind_down` while pressed, else `kind`.
    pub fn kind_in(&self, state: &super::StateSet) -> Option<SurfaceKind> {
        if state.state == craft_theme::State::Pressed { self.kind_down.or(self.kind) } else { self.kind }
    }

    /// Sets the surface kind.
    pub fn kind(mut self, kind: SurfaceKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Sets the kind used while pressed.
    pub fn kind_down(mut self, kind: SurfaceKind) -> Self {
        self.kind_down = Some(kind);
        self
    }

    /// Sets the corner radius.
    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;
        self
    }

    /// Sets which corners are rounded.
    pub fn corners(mut self, corners: Corners) -> Self {
        self.corners = corners;
        self
    }

    /// Sets the border colours and width.
    pub fn border(mut self, border: Stateful<StyleColor>, width: f32) -> Self {
        self.border = Some(border);
        self.border_width = width;
        self
    }
}

/// A component's text.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextLook {
    /// The type role.
    pub role: TypeRole,
    /// The text colour per state.
    pub color: Stateful<StyleColor>,
}

impl TextLook {
    /// A text look.
    pub fn new(role: TypeRole, color: Stateful<StyleColor>) -> Self {
        TextLook { role, color }
    }
}

/// A component's icon.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconLook {
    /// The side of the icon box.
    pub side: f32,
    /// The icon colour per state.
    pub color: Stateful<StyleColor>,
}

impl IconLook {
    /// An icon look.
    pub fn new(side: f32, color: Stateful<StyleColor>) -> Self {
        IconLook { side, color }
    }
}

/// A component's focus ring.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FocusLook {
    /// The ring stroke.
    pub stroke: Stroke,
    /// How far the ring sits outside the shape.
    pub offset: f32,
}

impl FocusLook {
    /// A focus look.
    pub const fn new(stroke: Stroke, offset: f32) -> Self {
        FocusLook { stroke, offset }
    }
}

/// What `#[derive(ComponentStyle)]` writes for a component's `Style` type.
pub trait ComponentStyle: Clone + PartialEq + Send + Sync + 'static {
    /// The variants the component accepts; the first is the default.
    const VARIANTS: &'static [Variant] = &[];

    /// The component's surface, if it has one.
    fn surface(&self) -> Option<&SurfaceLook> {
        None
    }

    /// The component's focus ring, if it shows one.
    fn focus(&self) -> Option<&FocusLook> {
        None
    }

    /// The component's text looks, in field order.
    fn texts(&self) -> Vec<&TextLook> {
        Vec::new()
    }

    /// The component's icon looks, in field order.
    fn icons(&self) -> Vec<&IconLook> {
        Vec::new()
    }

    /// The component's layout metrics.
    fn metrics(&self) -> Metrics;
}
