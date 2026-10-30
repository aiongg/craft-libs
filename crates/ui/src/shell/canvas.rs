//! The Canvas preset: [`CanvasShell`] fixes the slot keys and flags of a document editor
//! (PhotoCraft, Studio), and [`ShellCx::canvas_panel`] lays out the canvas panel in its body.
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

use egui::{Frame, Id, Rect, Ui, UiBuilder, accesskit::Role};

use super::slot::SlotKey;
use super::{AppShell, BodyMode, ShellCx, ShellOutput, Slot, WindowKey};
use crate::direction;
use crate::localized::{Labels, Word, Words};
use crate::theme::{Tokens, TokensOf as _};

/// The slot keys of the preset.
mod key {
    use super::SlotKey;

    pub const TITLE: SlotKey = "title";
    pub const OPTIONS: SlotKey = "options";
    pub const TOOLS: SlotKey = "tools";
    pub const STATUS: SlotKey = "status";
}

/// The translated AccessKit names of the preset's regions. `Default` is English; a `None` name
/// leaves the region unnamed, and a shell given no labels uses [`CanvasLabels::of`].
#[derive(Clone, Debug)]
pub struct CanvasLabels {
    pub options: Option<String>,
    pub tools: Option<String>,
    pub status: Option<String>,
}

/// One word per name, in field order.
const WORDS: [Word; 3] = [Word::keyed("shell.options", "Options"), Word::keyed("shell.tools", "Tools"), Word::keyed("shell.status", "Status")];

impl Labels for CanvasLabels {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self { options: None, tools: None, status: None }.localized_in(&Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let text = |i: usize| Some(words.get(WORDS[i]).into_owned());
        Self { options: text(0), tools: text(1), status: text(2) }
    }
}

impl Default for CanvasLabels {
    fn default() -> Self {
        Self::english()
    }
}

type SlotFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// [`AppShell`] with the Canvas preset's keys: `title` (top; reveals when visible), `options`
/// (top; reveals), more top bars, `tools` (start; reveals), `status` (bottom; never reveals),
/// more bottom bars. The body is the host's dock; its canvas panel calls
/// [`ShellCx::canvas_panel`]. Every `visible` argument is the host's flag, passed each frame.
/// Sizes come from the size tokens.
#[must_use = "call `show`"]
pub struct CanvasShell<'a> {
    shell: AppShell<'a>,
    labels: Option<CanvasLabels>,
    title: Option<(bool, SlotFn<'a>)>,
    options: Option<(bool, SlotFn<'a>)>,
    tools: Option<(bool, SlotFn<'a>)>,
    status: Option<(bool, SlotFn<'a>)>,
    top: Vec<(Slot, SlotFn<'a>)>,
    bottom: Vec<(Slot, SlotFn<'a>)>,
}

impl<'a> CanvasShell<'a> {
    pub fn new(window: WindowKey) -> Self {
        Self {
            shell: AppShell::new(window),
            labels: None,
            title: None,
            options: None,
            tools: None,
            status: None,
            top: Vec::new(),
            bottom: Vec::new(),
        }
    }

    /// The title slot (menu bar and controls). `visible` is the host's menu bar flag; the slot
    /// reveals only when it is on.
    pub fn title(mut self, visible: bool, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.title = Some((visible, Box::new(add)));
        self
    }

    /// The options bar, below the title.
    pub fn options(mut self, visible: bool, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.options = Some((visible, Box::new(add)));
        self
    }

    /// Another top bar, below the options bar and the ones added before it.
    pub fn top(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.top.push((slot, Box::new(add)));
        self
    }

    /// The tool rail at the start edge.
    pub fn tools(mut self, visible: bool, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.tools = Some((visible, Box::new(add)));
        self
    }

    /// The status bar, the lowest bottom bar.
    pub fn status(mut self, visible: bool, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status = Some((visible, Box::new(add)));
        self
    }

    /// Another bottom bar, above the status bar and the ones added before it.
    pub fn bottom(mut self, slot: Slot, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom.push((slot, Box::new(add)));
        self
    }

    pub fn labels(mut self, labels: CanvasLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn body_mode(mut self, mode: BodyMode) -> Self {
        self.shell = self.shell.body_mode(mode);
        self
    }

    pub fn allow_reveal(mut self, allow: bool) -> Self {
        self.shell = self.shell.allow_reveal(allow);
        self
    }

    /// The host's flag for the document tabs, read back through [`ShellCx::tabs_visible`].
    pub fn document_tabs(mut self, visible: bool) -> Self {
        self.shell = self.shell.document_tabs(visible);
        self
    }

    pub fn backdrop(mut self, color: Option<egui::Color32>) -> Self {
        self.shell = self.shell.backdrop(color);
        self
    }

    pub fn body_label(mut self, label: impl Into<String>) -> Self {
        self.shell = self.shell.body_label(label);
        self
    }

    /// Lays out the slots, then the body (the dock) in what is left.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui, &ShellCx) -> R + 'a) -> ShellOutput<R> {
        let Self { mut shell, labels, title, options, tools, status, top, bottom } = self;
        let labels = labels.unwrap_or_else(|| CanvasLabels::of(ui));
        if let Some((visible, add)) = title {
            shell = shell.title(Slot::new(key::TITLE).visible(visible).reveal_if(visible), add);
        }
        if let Some((visible, add)) = options {
            shell = shell.top(Slot::new(key::OPTIONS).visible(visible).reveal().labelled(labels.options), add);
        }
        for (slot, add) in top {
            shell = shell.top(slot, add);
        }
        if let Some((visible, add)) = status {
            shell = shell.bottom(Slot::new(key::STATUS).visible(visible).labelled(labels.status), add);
        }
        for (slot, add) in bottom {
            shell = shell.bottom(slot, add);
        }
        if let Some((visible, add)) = tools {
            shell = shell.start(Slot::new(key::TOOLS).visible(visible).reveal().labelled(labels.tools), add);
        }
        shell.show(ui, body)
    }
}

impl Slot {
    fn labelled(self, label: Option<String>) -> Self {
        match label {
            Some(label) => self.label(label),
            None => self,
        }
    }
}

/// The panel rail of the canvas panel: an icon column at the end edge, drawn inside the canvas
/// panel and spanning the tabs and the document. Optional: an app whose dock has its own iconic
/// column leaves it out.
pub struct PanelRail<'a> {
    width: f32,
    visible: bool,
    label: Option<String>,
    add: SlotFn<'a>,
}

impl<'a> PanelRail<'a> {
    /// A rail of exactly `width`. The closure draws its own surface and edge.
    pub fn new(width: f32, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self { width, visible: true, label: None, add: Box::new(add) }
    }

    /// The host's flag for the rail.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// The translated AccessKit name of the rail's region.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// What to add around the document in [`ShellCx::canvas_panel_with`].
#[derive(Default)]
pub struct PanelOptions<'a> {
    /// The panel rail, if the app has one.
    pub rail: Option<PanelRail<'a>>,
    /// Documents are opening: the tab strip shows even when [`ShellCx::tabs_visible`] is false
    /// (PhotoCraft shows a tab for a file that is still loading).
    pub opening: bool,
}

/// What [`ShellCx::canvas_panel`] drew.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CanvasOutput<R = ()> {
    /// What the document closure returned.
    pub inner: R,
    /// The rail's rect, when it showed.
    pub rail: Option<Rect>,
    /// The tab strip's rect, when it showed.
    pub tabs: Option<Rect>,
    /// The document area.
    pub document: Rect,
}

impl ShellCx {
    /// Lays out the canvas panel without a panel rail. See [`canvas_panel_with`](Self::canvas_panel_with).
    pub fn canvas_panel<R>(&self, ui: &mut Ui, document_tabs: impl FnOnce(&mut Ui), document: impl FnOnce(&mut Ui) -> R) -> CanvasOutput<R> {
        self.canvas_panel_with(ui, PanelOptions::default(), document_tabs, document)
    }

    /// Lays out the canvas panel in `ui` (the dock's canvas panel): the panel rail at the end
    /// edge, drawn first and spanning the tabs and the document; then, in what is left, the
    /// document tab strip (only while [`tabs_visible`](Self::tabs_visible) or
    /// [`PanelOptions::opening`] in normal mode), flush against the document; then the document area, which
    /// the backdrop of the shell shows through. The rail is skipped when its `visible` flag is
    /// off, and when the dock is hidden ([`dock_visible`](Self::dock_visible)). With no documents the host draws its start screen instead
    /// and does not call this.
    pub fn canvas_panel_with<R>(
        &self,
        ui: &mut Ui,
        options: PanelOptions<'_>,
        document_tabs: impl FnOnce(&mut Ui),
        document: impl FnOnce(&mut Ui) -> R,
    ) -> CanvasOutput<R> {
        let ctx = ui.ctx().clone();
        let tab_height = Tokens::of(ui).size.bar.tab;
        let mut rail_rect = None;
        if let Some(rail) = options.rail.filter(|r| r.visible && self.dock_visible()) {
            let salt = Id::unique(("app_shell", self.window().as_str(), "panel_rail"));
            let mut builder = UiBuilder::new().accessibility_role(Role::Region);
            if let Some(label) = &rail.label {
                builder = builder.accessibility_label(label.as_str());
            }
            let add = rail.add;
            let r = direction::end_panel(&ctx, salt).resizable(false).show_separator_line(false).frame(Frame::NONE).exact_size(rail.width).show(
                ui,
                |ui| {
                    ui.scope_builder(builder, add);
                },
            );
            rail_rect = Some(r.response.rect);
        }
        let mut tabs_rect = None;
        if self.tabs_visible() || (options.opening && self.body_mode() == BodyMode::Normal) {
            let salt = Id::unique(("app_shell", self.window().as_str(), "document_tabs"));
            let r = egui::Panel::top(salt).resizable(false).show_separator_line(false).frame(Frame::NONE).exact_size(tab_height).show(ui, |ui| {
                // The tab strip sits directly against the document: no gap from the parent's spacing.
                ui.spacing_mut().item_spacing.y = 0.0;
                document_tabs(ui);
            });
            tabs_rect = Some(r.response.rect);
        }
        let central = egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| (ui.max_rect(), document(ui)));
        let (document_rect, inner) = central.inner;
        CanvasOutput { inner, rail: rail_rect, tabs: tabs_rect, document: document_rect }
    }
}
