//! `FloatingGroup`: the window a floating dock group shows in.
//!
//! The group is a movable, resizable container on a layer (or in an OS window) whose rect the caller owns: the
//! caller places it with the rect the dock model keeps and takes the rect the output reports after each step of
//! a gesture. The title row carries the active panel's title and the Menu, Float and Close actions; the body step
//! supplies the move band, the edge grips and the keyboard Move and Size modes.

use super::PanelLimits;
use crate::contract::BodyCx;
use crate::contract::overlay::{PlacementChange, PlacementKind};
use crate::contract::{
    Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Inner, Mount, NoValue, Outcome, Overlay, Pad, Props as _, Role,
    Show as _, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::runtime::env::{Env, Label};
use crate::runtime::windows::WindowSpec;
use crate::sections::{HeaderAction, HeaderStrength, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{GesturePhase, Metrics, Sense};
use egui::{Pos2, Rect, Response, Ui, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// The actions a floating group's title row shows unless `actions` says otherwise.
const ACTIONS: [HeaderAction; 3] = [HeaderAction::Menu, HeaderAction::Float, HeaderAction::Close];

/// The style of a floating group: the surface and padding come from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FloatingGroupStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a floating group reports in a pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct FloatingGroupOutput {
    /// The group's response.
    pub response: Response,
    /// The last step of a move or resize this pass, with the rect the caller keeps: while the group is minimized
    /// the rect has the height the group has when restored.
    pub placed: Option<PlacementChange>,
    /// The title row's action part taken this pass. Float, Close and Menu are the host's: the dock model re-docks or
    /// closes the group, and the host shows the panel menu.
    pub action: Option<HeaderAction>,
    /// The rect of the Menu action part, for a menu hung under it.
    pub menu_rect: Option<Rect>,
    /// Whether a double click on the title band asks to minimize the group, or restore it when it is minimized.
    pub minimized_toggled: bool,
    /// Where the pointer was when a move by the title committed. The host resolves it against the dock's spots
    /// (`DockTargets::resolve`) to dock the group.
    pub released_at: Option<Pos2>,
}

impl Outcome for FloatingGroupOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A floating dock group: a titled window the user moves by its title, resizes by its edges and corners, and
/// minimizes by a double click on the title.
///
/// - Placement: the group shows at the rect given to `new`; the user's moves and resizes come back in
///   `FloatingGroupOutput::placed` and the caller shows the next pass at that rect. A move or resize stays inside
///   `bounds` and the panel `limits` (the limits are the body's; the title row adds its height).
/// - Minimized: only the title row shows, at the width of the rect, and the edges do not resize. The rect given to
///   `new` keeps the height the group has when restored.
/// - Window: with `native` set the group opens in an OS window of that kind, with the layer as the fallback where
///   the platform has no windows.
/// - Show it with [`show_group`](FloatingGroup::show_group): the closure fills the body, and is not called while
///   the group is minimized.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct FloatingGroup {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    rect: Rect,
    /// The rect a move or resize keeps the group inside.
    #[prop]
    bounds: Option<Rect>,
    /// The body's size limits.
    #[prop]
    limits: PanelLimits,
    /// The group's opacity, from 0 (invisible) to 1.
    #[prop]
    opacity: f32,
    /// Whether only the title row shows.
    #[prop]
    minimized: bool,
    /// Opens the group in an OS window of this kind.
    #[prop]
    native: Option<WindowSpec>,
    actions: Vec<HeaderAction>,
    taken: Option<HeaderAction>,
    menu_rect: Option<Rect>,
    toggled: bool,
    placed: Option<PlacementChange>,
    released: Option<Pos2>,
}

impl FloatingGroup {
    /// A floating group titled `title` at `rect`, with the Menu, Float and Close actions.
    pub fn new(title: impl Into<Label>, rect: Rect) -> Self {
        let props = ContainerProps::new()
            .surface(SurfaceKind::Floating)
            .pad(Pad::None)
            .sizing(Sizing::Content)
            .header(HeaderRegion::new().band(DragBand::Move).rule(true));
        FloatingGroup {
            common: Common::new(),
            props,
            rect,
            bounds: None,
            limits: PanelLimits::default(),
            opacity: 1.0,
            minimized: false,
            native: None,
            actions: ACTIONS.to_vec(),
            taken: None,
            menu_rect: None,
            toggled: false,
            placed: None,
            released: None,
        }
        .label(title)
    }

    /// The action parts at the end of the title row, in reading order.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = HeaderAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Shows the group; `body` fills the area under the title row unless the group is minimized.
    pub fn show_group<R>(mut self, ui: &mut Ui, mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Inner<FloatingGroupOutput, R> {
        let header = Env::of(ui).tokens.size.panel_header;
        let mount = self.mount(header);
        self.props = self.props.mount(mount);
        let minimized = self.minimized;
        let shown = self.show(ui, |ui, cx| (!minimized).then(|| body(ui, cx)));
        Inner { output: shown.output, inner: shown.inner.flatten() }
    }

    /// The layer or window the group shows in. `header` is the height of the title row.
    fn mount(&self, header: f32) -> Mount {
        let height = if self.minimized { header } else { self.rect.height() };
        let placed = Rect::from_min_size(self.rect.min, Vec2::new(self.rect.width(), height));
        let least = Vec2::new(self.limits.min.x, if self.minimized { header } else { self.limits.min.y + header });
        let most = self.limits.max.map_or(Vec2::INFINITY, |max| max + Vec2::new(0.0, header));
        let mut overlay = Overlay::floating().movable(true).resizable(!self.minimized).placed(placed).size_limits(least, most).opacity(self.opacity);
        if let Some(bounds) = self.bounds {
            overlay = overlay.bounds(bounds);
        }
        match &self.native {
            Some(spec) => Mount::viewport_with(overlay, spec.clone()),
            None => Mount::Layer(overlay),
        }
    }
}

impl Component for FloatingGroup {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = FloatingGroupStyle;
    type Output = FloatingGroupOutput;

    fn style(_: &StyleCx<'_>, _: FloatingGroupStyleKey) -> FloatingGroupStyle {
        FloatingGroupStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> FloatingGroupOutput {
        FloatingGroupOutput {
            response: cx.response.clone(),
            placed: self.placed,
            action: self.taken,
            menu_rect: self.menu_rect,
            minimized_toggled: self.toggled,
            released_at: self.released,
        }
    }
}

impl Container for FloatingGroup {
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let shown = TitleRow::new(cx.label()).strength(HeaderStrength::Strong).actions(self.actions.iter().copied()).show(ui);
        self.taken = shown.action.or(self.taken);
        self.menu_rect = shown.menu_rect.or(self.menu_rect);
        self.toggled |= cx.band_double_pressed();
    }

    fn placement_changed(&mut self, change: &PlacementChange) {
        let mut change = *change;
        if self.minimized {
            change.rect = Rect::from_min_size(change.rect.min, Vec2::new(change.rect.width(), self.rect.height()));
        }
        if change.kind == PlacementKind::Move && change.phase == GesturePhase::Commit {
            self.released = change.pointer;
        }
        self.placed = Some(change);
    }
}
