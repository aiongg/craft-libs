//! The canvas preset: `CanvasShell` fixes the slot keys and flags of a document editor, and `CanvasPanel` lays out the
//! canvas panel in its body.
//!
//! ```text
//! +-----------------------------------------+
//! | title                                   |
//! | options                                 |
//! +-------+---------------------------------+
//! | tools |  body (dock with canvas panel)  |
//! +-------+---------------------------------+
//! | status                                  |
//! +-----------------------------------------+
//!
//! canvas panel:  +----------------------+------+
//!                | document tabs        | panel|
//!                +----------------------+ rail |
//!                | document             |      |
//!                +----------------------+------+
//! ```

#[cfg(feature = "testing")]
mod stories;

use super::app_shell::{AppShell, AppShellOutput, RegionBody, RegionFn, ShellRegion};
use super::reveal::PanelsHidden;
use super::screen_mode::ScreenMode;
use super::words::ShellWords;
use crate::contract::{Bind, BodyCx, NoValue, Outcome, PaneEdge, Props as _, ShellPane, ShowContainer as _};
use crate::runtime::env::{BodyMode, Env, Label, ShellEntry, WindowKey};
use crate::runtime::frame::ChromeToggle;
use crate::shell::Slot;
use egui::{Rect, Response, Ui, ViewportId};

/// The slot keys of the preset.
pub mod slots {
    use crate::runtime::env::SlotKey;

    /// The title bar: the menu bar and window controls.
    pub const TITLE: SlotKey = "title";
    /// The options bar, below the title.
    pub const OPTIONS: SlotKey = "options";
    /// The tool rail at the start edge.
    pub const TOOLS: SlotKey = "tools";
    /// The status bar, the lowest bottom bar.
    pub const STATUS: SlotKey = "status";
}

/// A flagged slot of the preset: the host's visibility flag and the closure that fills it.
type Flagged<'a> = Option<(bool, RegionFn<'a>)>;

/// `AppShell` with the preset's keys: `title` (top; reveals when visible), `options` (top; reveals), more top bars,
/// `tools` (start; reveals), `status` (bottom; never reveals), more bottom bars. The body is the host's dock; its canvas
/// panel is a [`CanvasPanel`]. Every `visible` argument is the host's flag, passed each frame. The regions are named by
/// `ShellWords`; sizes come from the slots' contents.
#[must_use = "show the shell with `show`"]
#[derive(crate::contract::Pattern)]
pub struct CanvasShell<'a> {
    shell: AppShell<'a>,
    title: Flagged<'a>,
    options: Flagged<'a>,
    tools: Flagged<'a>,
    status: Flagged<'a>,
    top: Vec<(Slot, RegionFn<'a>)>,
    bottom: Vec<(Slot, RegionFn<'a>)>,
}

impl<'a> CanvasShell<'a> {
    /// A shell for `window` with no slots yet.
    pub fn new(window: WindowKey) -> Self {
        CanvasShell { shell: AppShell::new(window), title: None, options: None, tools: None, status: None, top: Vec::new(), bottom: Vec::new() }
    }

    /// The title slot. `visible` is the host's menu bar flag; the slot reveals only when it is on.
    pub fn title(mut self, visible: bool, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.title = Some((visible, Box::new(add)));
        self
    }

    /// The options bar, below the title.
    pub fn options_bar(mut self, visible: bool, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.options = Some((visible, Box::new(add)));
        self
    }

    /// Another top bar, below the options bar and the ones added before it.
    pub fn top(mut self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.top.push((slot, Box::new(add)));
        self
    }

    /// The tool rail at the start edge.
    pub fn tools(mut self, visible: bool, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.tools = Some((visible, Box::new(add)));
        self
    }

    /// The status bar, the lowest bottom bar.
    pub fn status(mut self, visible: bool, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.status = Some((visible, Box::new(add)));
        self
    }

    /// Another bottom bar, above the status bar and the ones added before it.
    pub fn bottom(mut self, slot: Slot, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.bottom.push((slot, Box::new(add)));
        self
    }

    /// The body, the host's dock.
    pub fn body(mut self, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.shell = self.shell.body(add);
        self
    }

    /// The window's screen mode; see [`AppShell::screen_mode`].
    pub fn screen_mode(mut self, mode: Bind<'a, ScreenMode>) -> Self {
        self.shell = self.shell.screen_mode(mode);
        self
    }

    /// Whether the reveal works in body-only mode; see [`AppShell::allow_reveal`].
    pub fn allow_reveal(mut self, allow: bool) -> Self {
        self.shell = self.shell.allow_reveal(allow);
        self
    }

    /// Whether the pointer at an edge reveals; see [`AppShell::edge_reveal`].
    pub fn edge_reveal(mut self, on: bool) -> Self {
        self.shell = self.shell.edge_reveal(on);
        self
    }

    /// The host's flag for the document tabs; see [`AppShell::document_tabs`].
    pub fn document_tabs(mut self, visible: bool) -> Self {
        self.shell = self.shell.document_tabs(visible);
        self
    }

    /// The name of the body region; see [`AppShell::body_label`].
    pub fn body_label(mut self, label: impl Into<Label>) -> Self {
        self.shell = self.shell.body_label(label);
        self
    }

    /// The chrome toggles of this frame; see [`AppShell::chrome_toggles`].
    pub fn chrome_toggles(mut self, toggles: &'a [(ViewportId, ChromeToggle)]) -> Self {
        self.shell = self.shell.chrome_toggles(toggles);
        self
    }

    /// Lays out the slots, then the body in what is left.
    pub fn show(self, ui: &mut Ui) -> AppShellOutput {
        let CanvasShell { mut shell, title, options, tools, status, top, bottom } = self;
        let words = Env::of(ui).words::<ShellWords>();
        if let Some((visible, add)) = title {
            shell = shell.title(Slot::new(slots::TITLE).visible(visible).reveal_if(visible), add);
        }
        if let Some((visible, add)) = options {
            shell = shell.top(Slot::new(slots::OPTIONS).visible(visible).reveal().label(words.options.clone()), add);
        }
        for (slot, add) in top {
            shell = shell.top(slot, add);
        }
        if let Some((visible, add)) = status {
            shell = shell.bottom(Slot::new(slots::STATUS).visible(visible).label(words.status.clone()), add);
        }
        for (slot, add) in bottom {
            shell = shell.bottom(slot, add);
        }
        if let Some((visible, add)) = tools {
            shell = shell.start(Slot::new(slots::TOOLS).visible(visible).reveal().label(words.tools.clone()), add);
        }
        shell.show(ui)
    }
}

/// What a [`CanvasPanel`] drew.
#[derive(Debug)]
#[non_exhaustive]
pub struct CanvasPanelOutput {
    /// The response of the document region.
    pub response: Response,
    /// The rail's rect, when it showed.
    pub rail: Option<Rect>,
    /// The tab strip's rect, when it showed.
    pub tabs: Option<Rect>,
    /// The document area.
    pub document: Rect,
}

impl Outcome for CanvasPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The canvas panel of a dock: the panel rail at the end edge, spanning the tabs and the document; then, in what is
/// left, the document tab strip flush against the document; then the document area, which the shell's backdrop shows
/// through. Fill the rail with `rails::PanelRail` and the tab strip with `tabs::DocumentTabs`.
///
/// - **Tabs.** They show in normal mode when the host says so (`ShellCx::tabs_visible`), or while documents are
///   [`opening`](CanvasPanel::opening) (a file that is still loading has a tab). Body-only mode shows neither tabs nor
///   rail, but the rail shows during a reveal that shows the dock.
/// - **Rail.** It is skipped when the dock is hidden (`ShellCx::dock_visible`, or the panels are hidden with
///   Shift+Tab).
///
/// With no documents the host shows its start screen instead and does not use the panel. Outside a shell the panel
/// shows everything.
#[must_use = "show the panel with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct CanvasPanel<'a> {
    rail: Option<(Label, RegionFn<'a>)>,
    /// The document tab strip.
    #[prop(func = "FnMut(&mut Ui, &mut BodyCx<'_>)")]
    tabs: Option<RegionBody<'a>>,
    /// Documents are opening: the tab strip shows even when the host hides the tabs, in normal mode.
    #[prop]
    opening: bool,
}

impl<'a> CanvasPanel<'a> {
    /// A panel with no rail and no tab strip.
    pub fn new() -> Self {
        CanvasPanel { rail: None, tabs: None, opening: false }
    }

    /// The panel rail, named `label`.
    pub fn rail(mut self, label: impl Into<Label>, add: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.rail = Some((label.into(), Box::new(add)));
        self
    }

    /// Lays the panel out in `ui`, the body of the dock's canvas panel, with `document` in the area left.
    pub fn show(self, ui: &mut Ui, mut document: impl FnMut(&mut Ui, &mut BodyCx<'_>)) -> CanvasPanelOutput {
        let CanvasPanel { rail, tabs, opening } = self;
        let env = Env::of(ui);
        let words = env.words::<ShellWords>();
        let shell = env.get::<ShellEntry>().0;
        let panels_hidden = env.get::<PanelsHidden>().0;
        let dock = shell.as_ref().is_none_or(|shell| shell.dock_visible()) && !panels_hidden;
        let normal = shell.as_ref().is_none_or(|shell| shell.body_mode() == BodyMode::Normal);
        let tabs_shown = shell.as_ref().is_none_or(|shell| shell.tabs_visible()) || (opening && normal);

        let mut rail_rect = None;
        if let (Some((label, mut add)), true) = (rail, dock) {
            let out = ShellRegion::pane(ShellPane::new("panel_rail").edge(PaneEdge::End)).label(label).show(ui, |ui, cx| add(ui, cx));
            rail_rect = Some(out.response.rect);
        }
        let mut tabs_rect = None;
        if let (Some(mut add), true) = (tabs.map(|RegionBody(add)| add), tabs_shown) {
            let out = ShellRegion::pane(ShellPane::new("document_tabs").edge(PaneEdge::Top))
                .label(words.document_tabs.clone())
                .show(ui, |ui, cx| add(ui, cx));
            tabs_rect = Some(out.response.rect);
        }
        let out = ShellRegion::pane(ShellPane::new("document")).label(words.document.clone()).show(ui, |ui, cx| document(ui, cx));
        CanvasPanelOutput { document: out.response.rect, response: out.output.response, rail: rail_rect, tabs: tabs_rect }
    }
}

impl Default for CanvasPanel<'_> {
    fn default() -> Self {
        Self::new()
    }
}
