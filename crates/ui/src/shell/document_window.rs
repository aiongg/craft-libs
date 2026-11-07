//! `DocumentWindow`: a document's window, a container in its own OS viewport.

#[cfg(feature = "testing")]
mod stories;

use crate::contract::overlay::OverlayFit;
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, NoValue, Outcome, Overlay, Pad,
    Props as _, Role, SaveKey, Show as _, Sizing, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::runtime::overlay::CloseReason;
use crate::runtime::windows::{WindowAction, WindowHost, WindowSpec};
use crate::sections::{HeaderAction, HeaderStrength, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use craft_window::{CASCADE_STEP, cascade, monitor_of};
use egui::{Rect, Response, Ui, Vec2};

/// The style of a document window: the surface comes from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DocumentWindowStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a document window reports: its response, whether it was asked to close in this pass, and whether it has OS
/// focus.
#[derive(Debug)]
#[non_exhaustive]
pub struct DocumentWindowOutput {
    /// The window's response.
    pub response: Response,
    /// The window was asked to close in this pass, by its OS close button or the fallback layer's close action.
    /// Reported once for each request.
    pub close_requested: bool,
    /// The window has OS focus. False in the fallback layer, which has no window of its own.
    pub focused: bool,
}

impl Outcome for DocumentWindowOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A document's window: the container of one entry of the app's window list, in its own OS window.
///
/// The app shows one for each open document and stops showing it to close it. The body holds the document's whole
/// window, including its title bar and shell; the window adds none of its own where the OS window exists.
///
/// - **Placement.** The window opens where [`first_placed`](DocumentWindow::first_placed) puts it (see
///   [`cascaded`](DocumentWindow::cascaded)) until a placement saved under [`remember`](DocumentWindow::remember)
///   takes over. The minimum and maximum size (through [`overlay`](DocumentWindow::overlay)) and the resizability are the
///   fallback layer's, which the OS window reads.
/// - **Close.** The OS close button, and the close action of the fallback layer, report `close_requested`. The
///   window closes at once unless the app binds [`opened`](DocumentWindow::opened) to a value it sets again, for
///   example while it asks about unsaved changes.
/// - **Title and focus.** The title is the label given to `new`; it follows the document while the app changes it.
///   [`raise`](DocumentWindow::raise) brings the window forward in the pass it is set.
/// - **Fallback.** Where OS windows are unavailable the document shows on a movable, resizable layer with a title row.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct DocumentWindow<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// Raises the window and gives it OS focus in this pass.
    #[prop(flag)]
    raise: bool,
    closed: Option<CloseReason>,
}

impl DocumentWindow<'_> {
    /// A window titled `title`, resizable, with the installed decision's frame.
    pub fn new(title: impl Into<Label>) -> Self {
        let fallback = Overlay::floating().resizable(true).fit(OverlayFit::Resizable);
        let props = ContainerProps::new()
            .surface(SurfaceKind::Background)
            .pad(Pad::None)
            .sizing(Sizing::Fill)
            .header(HeaderRegion::new().band(DragBand::Move))
            .mount(Mount::viewport_with(fallback, WindowSpec::document()));
        DocumentWindow { common: Common::new(), props, open: None, raise: false, closed: None }.label(title)
    }

    /// Changes the layer the window falls back to, for the settings without a setter of their own: the smallest and
    /// largest size (`Overlay::size_limits`), which the OS window takes as its limits.
    #[must_use]
    pub fn overlay(mut self, change: impl FnOnce(Overlay) -> Overlay) -> Self {
        self.props = match std::mem::take(&mut self.props.mount) {
            Mount::Viewport { fallback, spec } => self.props.mount(Mount::Viewport { fallback: change(fallback), spec }),
            other => self.props.mount(other),
        };
        self
    }

    /// Where the window opens, as its outer rect in desktop points. A saved placement comes first.
    #[must_use]
    pub fn first_placed(self, rect: Rect) -> Self {
        self.overlay(|overlay| overlay.placed(rect))
    }

    /// Whether the user can resize the window.
    #[must_use]
    pub fn resizable(self, on: bool) -> Self {
        self.overlay(|overlay| overlay.resizable(on))
    }

    /// Saves the window's placement and size under `key`, so a reopened document sits where it was left.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
        self
    }

    /// Where a window of `size` opens when it follows the window at `previous`: one cascade step down and across;
    /// an axis on which that would leave the monitor starts again at the monitor's edge. `monitors` are in the points of the window
    /// at `previous`; without monitors the step is not bounded.
    pub fn cascade_in(previous: Rect, size: Vec2, monitors: &[Rect]) -> Rect {
        let rect = |rect: Rect| craft_window::Rect::new(rect.min.x, rect.min.y, rect.width(), rect.height());
        let monitors: Vec<craft_window::Rect> = monitors.iter().copied().map(rect).collect();
        match monitor_of(rect(previous), &monitors) {
            Some(bounds) => {
                let at = cascade(rect(previous), [size.x, size.y], bounds, CASCADE_STEP);
                Rect::from_min_size(egui::pos2(at.x, at.y), size)
            }
            None => Rect::from_min_size(previous.min + Vec2::splat(CASCADE_STEP), size),
        }
    }

    /// [`cascade_in`](DocumentWindow::cascade_in) over the monitors the window host lists.
    pub fn cascaded(ui: &Ui, previous: Rect, size: Vec2) -> Rect {
        let host = WindowHost::of(ui.ctx());
        let scale = host.viewport_at(previous.center()).and_then(|viewport| host.facts(viewport)).map_or(1.0, |facts| facts.scale);
        let monitors: Vec<Rect> = host.monitors().iter().map(|monitor| monitor.points_at(scale)).collect();
        Self::cascade_in(previous, size, &monitors)
    }
}

impl Component for DocumentWindow<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = DocumentWindowStyle;
    type Output = DocumentWindowOutput;

    fn style(_: &StyleCx<'_>, _: DocumentWindowStyleKey) -> DocumentWindowStyle {
        DocumentWindowStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> DocumentWindowOutput {
        let host = WindowHost::of(&cx.ctx);
        let viewport = host.viewport_of(cx.id);
        if self.raise
            && let Some(viewport) = viewport
        {
            host.act(viewport, WindowAction::Focus);
        }
        let focused = viewport.and_then(|viewport| host.facts(viewport)).is_some_and(|facts| facts.focused);
        DocumentWindowOutput { response: cx.response.clone(), close_requested: self.closed.is_some(), focused }
    }
}

impl Container for DocumentWindow<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    /// The title row of the fallback layer; an OS window has the title bar the app composes in its body.
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        if WindowHost::of(cx.memory.ctx()).viewports_available() {
            return;
        }
        let shown = TitleRow::new(cx.label()).strength(HeaderStrength::Strong).actions([HeaderAction::Close]).show(ui);
        if shown.action == Some(HeaderAction::Close) {
            cx.request_close();
        }
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
