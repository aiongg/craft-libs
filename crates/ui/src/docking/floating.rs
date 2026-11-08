//! `FloatingGroup`: the window a floating dock group shows in.
//!
//! The group is a movable, resizable container on a layer (or in an OS window) whose rect the caller owns: the
//! caller places it with the rect the dock model keeps and takes the rect the output reports after each step of
//! a gesture. The title row carries the active panel's title and the Menu, Float and Close actions; the body step
//! supplies the move band, the edge grips and the keyboard Move and Size modes.

use super::drop::{DragUnit, start_group_drag};
use super::{PanelKey, PanelLimits};
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
use egui::{Rect, Response, Ui, Vec2};
use std::hash::Hash;

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
/// - Sizing: with `Sizing::Content` (the default) the rect fixes the position and the width, and the height follows
///   the body; with another sizing the rect is the size. The user's resizes are reported either way.
/// - Minimized: only the title row shows, at the width of the rect, and the edges do not resize. The rect given to
///   `new` keeps the height the group has when restored.
/// - Docking: with [`window_of`](FloatingGroup::window_of) set, a move by the title starts a window-unit drag
///   (`GroupDrag`) for the dock's `DockDropTarget`, which shows the drop mark and docks the window on release.
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
    #[prop(quantity = "opacity")]
    opacity: f32,
    /// Whether only the title row shows.
    #[prop]
    minimized: bool,
    /// Whether the height follows the body (`Sizing::Content`) or the rect gives it.
    #[prop]
    sizing: Sizing,
    /// Opens the group in an OS window of this kind.
    #[prop]
    native: Option<WindowSpec>,
    actions: Vec<HeaderAction>,
    taken: Option<HeaderAction>,
    menu_rect: Option<Rect>,
    toggled: bool,
    placed: Option<PlacementChange>,
    /// Starts the window-unit drag; set by `window_of`.
    start_drag: Option<StartDrag>,
    /// A move by the title began this pass.
    moving: bool,
}

type StartDrag = Box<dyn for<'a, 'b> FnOnce(&'a mut Cx<'b, FloatingGroup>)>;

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
            sizing: Sizing::Content,
            native: None,
            actions: ACTIONS.to_vec(),
            taken: None,
            menu_rect: None,
            toggled: false,
            placed: None,
            start_drag: None,
            moving: false,
        }
        .label(title)
    }

    /// The group is the floating window of the dock model that holds `anchor`: a move by its title starts a
    /// window-unit drag, so the dock marks where the window would dock while it moves and docks it on release.
    #[must_use]
    pub fn window_of<P: Clone + Hash + Send + Sync + 'static>(mut self, anchor: P) -> Self {
        let active = PanelKey::of(&anchor);
        self.start_drag = Some(Box::new(move |cx| start_group_drag(cx, DragUnit::Window(anchor), active, None)));
        self
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
    ///
    /// A minimized group, and one with `Sizing::Content`, is placed by its position and width; its height is its
    /// content's, which is the title row alone while minimized.
    fn mount(&self, header: f32) -> Mount {
        let least = Vec2::new(self.limits.min.x, self.limits.min.y + header);
        let most = self.limits.max.map_or(Vec2::INFINITY, |max| max + Vec2::new(0.0, header));
        let mut overlay = Overlay::floating().movable(true).resizable(!self.minimized).opacity(self.opacity);
        overlay = if self.minimized {
            let width = self.rect.width();
            overlay.placed_at(self.rect.min).size_limits(Vec2::new(width, 0.0), Vec2::new(width, f32::INFINITY))
        } else if self.sizing == Sizing::Content {
            let width = self.rect.width().max(least.x).min(most.x);
            overlay.placed_at(self.rect.min).size_limits(Vec2::new(width, least.y), most)
        } else {
            overlay.placed(self.rect).size_limits(least, most)
        };
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

    fn finish(mut self, mut cx: Cx<'_, Self>) -> FloatingGroupOutput {
        if self.moving
            && let Some(start) = self.start_drag.take()
        {
            start(&mut cx);
        }
        FloatingGroupOutput {
            response: cx.response.clone(),
            placed: self.placed,
            action: self.taken,
            menu_rect: self.menu_rect,
            minimized_toggled: self.toggled,
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
        self.moving |= change.kind == PlacementKind::Move && change.phase == GesturePhase::Begin;
        self.placed = Some(change);
    }
}
