//! `ContainerProps` and the types of its fields (16.2).

use super::label::SideLabel;
use super::overlay::Overlay;
use crate::layout::BodyLayout;
use crate::runtime::access::Politeness;
use crate::runtime::commands::CommandScope;
use crate::runtime::focus::FocusPolicy;
use crate::runtime::memory::SaveKey;
use crate::style::SurfaceKind;
use craft_behavior::{CanvasTransform, Roving};
use craft_theme::ControlSize;

pub use crate::runtime::env::ThemeScope;

/// Where a container lives.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub enum Mount {
    /// In the parent's layout.
    #[default]
    Inline,
    /// One of `AppShell`'s panes.
    Pane(ShellPane),
    /// On an egui layer through the overlay host.
    Layer(Overlay),
    /// Over the whole window.
    FullWindow,
    /// In an OS viewport; `fallback` is the layer it uses where viewports are unavailable.
    #[non_exhaustive]
    Viewport {
        /// The layer used in place of the viewport.
        fallback: Overlay,
    },
}

impl Mount {
    /// An OS viewport that falls back to `fallback`.
    pub const fn viewport(fallback: Overlay) -> Mount {
        Mount::Viewport { fallback }
    }
}

/// Where a shell pane sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaneEdge {
    /// What the edge panes leave.
    #[default]
    Center,
    /// The start edge: left in reading order, right under right-to-left.
    Start,
    /// The end edge.
    End,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
}

/// One of `AppShell`'s panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShellPane {
    /// The pane's name.
    pub name: &'static str,
    /// The edge the pane sits on.
    pub edge: PaneEdge,
}

impl ShellPane {
    /// The central pane named `name`.
    pub const fn new(name: &'static str) -> Self {
        ShellPane { name, edge: PaneEdge::Center }
    }

    /// The pane at `edge`.
    pub const fn edge(mut self, edge: PaneEdge) -> Self {
        self.edge = edge;
        self
    }
}

/// The padding tier of a body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Pad {
    /// No padding.
    #[default]
    None,
    /// `space.pad.control`.
    Control,
    /// `space.pad.panel`.
    Regular,
    /// `space.pad.dialog`.
    Roomy,
}

/// How a container's size follows its content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sizing {
    /// The available rect.
    Fill,
    /// The size its content took last pass.
    #[default]
    Content,
}

/// What dragging the header does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DragBand {
    /// Nothing.
    #[default]
    None,
    /// Moves the container.
    Move,
    /// Reorders the container among its siblings.
    Reorder,
}

/// A header region, filled by `Container::header`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HeaderRegion {
    /// What dragging the header does.
    pub band: DragBand,
    /// Whether a rule separates the header from the body. An inline header has no rule.
    pub rule: bool,
    /// Whether the header shares a line with the body: the header at its measured width, the body in the rest.
    pub inline: bool,
    /// Whether the header has a trailing child region at its end edge, filled by `Container::controls`; the header
    /// itself fills the room it leaves.
    pub trailing: bool,
}

impl HeaderRegion {
    /// A header with no drag band, no rule, on its own line and with no trailing region.
    pub const fn new() -> Self {
        HeaderRegion { band: DragBand::None, rule: false, inline: false, trailing: false }
    }

    /// What dragging the header does.
    pub const fn band(mut self, band: DragBand) -> Self {
        self.band = band;
        self
    }

    /// Whether a rule separates the header from the body.
    pub const fn rule(mut self, on: bool) -> Self {
        self.rule = on;
        self
    }

    /// Whether the header shares a line with the body.
    pub const fn inline(mut self, on: bool) -> Self {
        self.inline = on;
        self
    }

    /// Whether the header has a trailing region for the container's controls.
    pub const fn trailing(mut self, on: bool) -> Self {
        self.trailing = on;
        self
    }
}

/// A footer region, filled by `Container::footer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FooterRegion {
    /// Whether a rule separates the body from the footer.
    pub rule: bool,
}

impl FooterRegion {
    /// A footer with no rule.
    pub const fn new() -> Self {
        FooterRegion { rule: false }
    }

    /// Whether a rule separates the body from the footer.
    pub const fn rule(mut self, on: bool) -> Self {
        self.rule = on;
        self
    }
}

/// Which end of the content a scroll region stays at when the content grows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollAnchor {
    /// The start.
    #[default]
    Start,
    /// The end (a timeline pane that follows its newest entry).
    End,
}

/// The distance of one wheel notch.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Notch {
    /// This many text lines.
    Lines(f32),
    /// This share of the view.
    ViewShare(f32),
}

impl Default for Notch {
    fn default() -> Self {
        Notch::Lines(3.0)
    }
}

/// The axes a body scrolls on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollAxes {
    /// Along y.
    Vertical,
    /// Along x.
    Horizontal,
    /// Both.
    Both,
}

/// A scrolling body.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Scroll {
    /// The axes that scroll.
    pub axes: ScrollAxes,
    /// The end the region stays at when the content grows.
    pub anchor: ScrollAnchor,
    /// The distance of one wheel notch.
    pub notch: Notch,
}

impl Scroll {
    const fn on(axes: ScrollAxes) -> Scroll {
        Scroll { axes, anchor: ScrollAnchor::Start, notch: Notch::Lines(3.0) }
    }

    /// Scrolls along y.
    pub const fn vertical() -> Scroll {
        Scroll::on(ScrollAxes::Vertical)
    }

    /// Scrolls along x.
    pub const fn horizontal() -> Scroll {
        Scroll::on(ScrollAxes::Horizontal)
    }

    /// Scrolls along both axes.
    pub const fn both() -> Scroll {
        Scroll::on(ScrollAxes::Both)
    }

    /// Stays at the end when the content grows.
    pub const fn anchored_end(mut self) -> Scroll {
        self.anchor = ScrollAnchor::End;
        self
    }

    /// The distance of one wheel notch.
    pub const fn notch(mut self, notch: Notch) -> Scroll {
        self.notch = notch;
        self
    }

    /// Whether the x axis scrolls.
    pub const fn horizontal_on(&self) -> bool {
        matches!(self.axes, ScrollAxes::Horizontal | ScrollAxes::Both)
    }

    /// Whether the y axis scrolls.
    pub const fn vertical_on(&self) -> bool {
        matches!(self.axes, ScrollAxes::Vertical | ScrollAxes::Both)
    }
}

/// Collapse configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Collapse {
    /// Whether the body starts open.
    pub initially_open: bool,
    /// Whether opening and closing animate.
    pub animated: bool,
}

impl Collapse {
    /// A collapsible body that starts open, animated.
    pub const fn new() -> Self {
        Collapse { initially_open: true, animated: true }
    }

    /// Whether the body starts open.
    pub const fn initially_open(mut self, on: bool) -> Self {
        self.initially_open = on;
        self
    }

    /// Whether opening and closing animate.
    pub const fn animated(mut self, on: bool) -> Self {
        self.animated = on;
        self
    }
}

impl Default for Collapse {
    fn default() -> Self {
        Collapse::new()
    }
}

/// The spacing between the children of a body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Gap {
    /// `space.gap.row`.
    #[default]
    Row,
    /// `space.gap.tight`.
    Tight,
    /// `space.gap.section`.
    Section,
}

/// What a container or collection adds to `Common`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ContainerProps {
    /// Where the container lives.
    pub mount: Mount,
    /// The surface the container paints; `None` for sections, stacks and rows.
    pub surface: Option<SurfaceKind>,
    /// The padding tier.
    pub pad: Pad,
    /// How the size follows the content.
    pub sizing: Sizing,
    /// How the body lays out its children.
    pub layout: BodyLayout,
    /// A header region.
    pub header: Option<HeaderRegion>,
    /// A scroll region.
    pub scroll: Option<Scroll>,
    /// A footer region.
    pub footer: Option<FooterRegion>,
    /// The focus scope of an `Inline`, `Pane` or `FullWindow` mount.
    pub focus: FocusPolicy,
    /// Collapse configuration; the initial state seeds the open slot.
    pub collapse: Option<Collapse>,
    /// The spacing between the children of the body.
    pub body_gap: Gap,
    /// A scope size for the body.
    pub child_size: Option<ControlSize>,
    /// The container is an arena root.
    pub arena: bool,
    /// The body is shown through this transform on its own egui layer.
    pub transform: Option<CanvasTransform>,
    /// Token-changing values for the body.
    pub theme: Option<ThemeScope>,
    /// The container is an instance of this command scope.
    pub command_scope: Option<CommandScope>,
    /// A live region.
    pub live: Option<Politeness>,
    /// A field label.
    pub label: Option<SideLabel>,
    /// The body's members are one roving stop.
    pub roving: Option<Roving>,
    /// Placement, size and open state saved across runs.
    pub save: Option<SaveKey>,
    /// One outer gesture over the body's fields.
    pub outer_edit: bool,
    /// The body holds one collection that owns the scroll: a layer's own body scroll is off.
    pub collection_body: bool,
    /// The labelled rows of the body share the widest label as one column.
    pub label_column: bool,
}

impl Default for ContainerProps {
    fn default() -> Self {
        ContainerProps::new()
    }
}

impl ContainerProps {
    /// Props with every field at its default.
    pub const fn new() -> Self {
        ContainerProps {
            mount: Mount::Inline,
            surface: None,
            pad: Pad::None,
            sizing: Sizing::Content,
            layout: BodyLayout::Stack,
            header: None,
            scroll: None,
            footer: None,
            focus: FocusPolicy::NONE,
            collapse: None,
            body_gap: Gap::Row,
            child_size: None,
            arena: false,
            transform: None,
            theme: None,
            command_scope: None,
            live: None,
            label: None,
            roving: None,
            save: None,
            outer_edit: false,
            collection_body: false,
            label_column: false,
        }
    }

    /// Where the container lives.
    pub fn mount(mut self, mount: Mount) -> Self {
        self.mount = mount;
        self
    }

    /// The surface the container paints.
    pub const fn surface(mut self, surface: SurfaceKind) -> Self {
        self.surface = Some(surface);
        self
    }

    /// The padding tier.
    pub const fn pad(mut self, pad: Pad) -> Self {
        self.pad = pad;
        self
    }

    /// How the size follows the content.
    pub const fn sizing(mut self, sizing: Sizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// How the body lays out its children.
    pub fn layout(mut self, layout: BodyLayout) -> Self {
        self.layout = layout;
        self
    }

    /// A header region.
    pub const fn header(mut self, header: HeaderRegion) -> Self {
        self.header = Some(header);
        self
    }

    /// A scroll region.
    pub const fn scroll(mut self, scroll: Scroll) -> Self {
        self.scroll = Some(scroll);
        self
    }

    /// The body holds one collection that owns the scroll (a popup list): a layer's own body scroll is off, and the
    /// collection scrolls inside the layer's capped room.
    pub const fn collection_body(mut self) -> Self {
        self.collection_body = true;
        self
    }

    /// A footer region.
    pub const fn footer(mut self, footer: FooterRegion) -> Self {
        self.footer = Some(footer);
        self
    }

    /// The focus scope.
    pub const fn focus(mut self, policy: FocusPolicy) -> Self {
        self.focus = policy;
        self
    }

    /// Collapse configuration.
    pub const fn collapse(mut self, collapse: Collapse) -> Self {
        self.collapse = Some(collapse);
        self
    }

    /// The spacing between the children of the body.
    pub const fn body_gap(mut self, gap: Gap) -> Self {
        self.body_gap = gap;
        self
    }

    /// A scope size for the body.
    pub const fn child_size(mut self, size: ControlSize) -> Self {
        self.child_size = Some(size);
        self
    }

    /// Makes the container an arena root.
    pub const fn arena(mut self, on: bool) -> Self {
        self.arena = on;
        self
    }

    /// Shows the body through `transform` on its own egui layer.
    pub const fn transform(mut self, transform: CanvasTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Token-changing values for the body.
    pub fn theme(mut self, theme: ThemeScope) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Makes the container an instance of `scope`.
    pub const fn command_scope(mut self, scope: CommandScope) -> Self {
        self.command_scope = Some(scope);
        self
    }

    /// A live region.
    pub const fn live(mut self, politeness: Politeness) -> Self {
        self.live = Some(politeness);
        self
    }

    /// A field label.
    pub fn label(mut self, label: SideLabel) -> Self {
        self.label = Some(label);
        self
    }

    /// Makes the body's members one roving stop.
    pub const fn roving(mut self, roving: Roving) -> Self {
        self.roving = Some(roving);
        self
    }

    /// Saves placement, size and open state under `key`.
    pub fn save(mut self, key: SaveKey) -> Self {
        self.save = Some(key);
        self
    }

    /// One outer gesture over the body's fields.
    pub const fn outer_edit(mut self, on: bool) -> Self {
        self.outer_edit = on;
        self
    }

    /// The labelled rows of the body share the widest label as one column. A nested container with its own
    /// label column opens its own group; a row reads the innermost.
    pub const fn label_column(mut self, on: bool) -> Self {
        self.label_column = on;
        self
    }
}
