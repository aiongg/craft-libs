//! Components and command sets for the command trials.

use craft_behavior::{Metrics, Sense};
use craft_ui::contract::{CommandSet, Common, Component, Container, ContainerProps, Cx, Role};
use craft_ui::runtime::commands::CommandScope;
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Response, Vec2};
use std::cell::RefCell;

/// A style with a fixed size.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FixedStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// App-scope commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "trial")]
#[non_exhaustive]
pub enum AppCommand {
    #[command(label = "Go")]
    Go,
    #[command(label = "Save", default = "Cmd+K")]
    Save,
    #[command(label = "Play", default = "Space")]
    Play,
    #[command(label = "Jump", default = "Cmd+Alt+J")]
    Jump,
    #[command(label = "Show Grid", kind = "toggle")]
    Grid,
    #[command(label = "Hold", kind = "momentary", default = "H")]
    Hold,
    #[command(label = "Brush", kind = "tool", group = "tools")]
    Brush,
}

/// Commands of the `trial.pane` scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "pane", scope = "trial.pane")]
#[non_exhaustive]
pub enum PaneCommand {
    #[command(label = "Zoom In", default = "Cmd+Alt+K")]
    ZoomIn,
}

/// Commands of the `trial.dock` scope, whose instances contain panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "dock", scope = "trial.dock")]
#[non_exhaustive]
pub enum DockCommand {
    #[command(label = "Close", default = "Cmd+Alt+J")]
    Close,
}

pub const PANE: CommandScope = <PaneCommand as CommandSet>::SCOPE;
pub const DOCK: CommandScope = <DockCommand as CommandSet>::SCOPE;

/// A button-like leaf that can report the label and enabled state the pipeline resolved for it.
#[derive(Component)]
pub struct Tap<'a> {
    #[common]
    common: Common,
    seen: Option<&'a RefCell<(String, bool)>>,
    on: Option<&'a std::cell::Cell<bool>>,
}

impl<'a> Tap<'a> {
    pub fn new() -> Self {
        Tap { common: Common::new(), seen: None, on: None }
    }

    pub fn on(mut self, on: &'a std::cell::Cell<bool>) -> Self {
        self.on = Some(on);
        self
    }

    pub fn seen(mut self, seen: &'a RefCell<(String, bool)>) -> Self {
        self.seen = Some(seen);
        self
    }
}

impl Component for Tap<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TapStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, _: &craft_ui::contract::MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(seen) = self.seen {
            *seen.borrow_mut() = (cx.resolved.label.to_string(), cx.resolved.enabled);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        if let Some(on) = self.on {
            on.set(craft_ui::contract::Styled::state(&cx).flags.intersects(craft_ui::style::Flags::ON));
        }
        cx.response.clone()
    }
}

/// A container that is an instance of the `trial.pane` scope.
#[derive(Component)]
#[component(container)]
pub struct Pane {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Pane {
    pub fn new() -> Self {
        Pane { common: Common::new(), props: ContainerProps::new().command_scope(PANE) }
    }
}

impl Component for Pane {
    const ROLE: Role = Role::GROUP;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PaneStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Pane {}

/// A container that is an instance of the `trial.dock` scope.
#[derive(Component)]
#[component(container)]
pub struct Dock {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Dock {
    pub fn new() -> Self {
        Dock { common: Common::new(), props: ContainerProps::new().command_scope(DOCK) }
    }
}

impl Component for Dock {
    const ROLE: Role = Role::GROUP;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DockStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Dock {}

/// A focusable leaf that lists every edit command and records the ones it takes and the text it pastes.
#[derive(Component)]
pub struct Field<'a> {
    #[common]
    common: Common,
    log: &'a RefCell<Vec<String>>,
}

impl<'a> Field<'a> {
    pub fn new(log: &'a RefCell<Vec<String>>) -> Self {
        Field { common: Common::new(), log }
    }
}

impl Component for Field<'_> {
    const ROLE: Role = Role::BUTTON;
    const KEYS: craft_ui::runtime::focus::KeyUse = craft_ui::runtime::focus::KeyUse::CANVAS.with_edits(craft_ui::runtime::focus::EditSet::ALL);
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, _: &craft_ui::contract::MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use craft_ui::contract::Focus as _;
        use craft_ui::runtime::commands::Commands as _;
        while let Some(command) = cx.intents().take_edit() {
            self.log.borrow_mut().push(format!("{command:?}"));
        }
        if let craft_ui::runtime::pending::Pending::Ready(content) = cx.clipboard().paste() {
            self.log.borrow_mut().push(format!("text {}", content.text.as_deref().unwrap_or("")));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A component whose overlay step reads its own context request.
#[derive(Component)]
#[component(overlay)]
pub struct Opens<'a> {
    #[common]
    common: Common,
    seen: &'a std::cell::Cell<bool>,
    ctx: egui::Context,
}

impl<'a> Opens<'a> {
    pub fn new(seen: &'a std::cell::Cell<bool>, ctx: egui::Context) -> Self {
        Opens { common: Common::new(), seen, ctx }
    }
}

impl Component for Opens<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = FixedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: OpensStyleKey) -> FixedStyle {
        FixedStyle::default()
    }

    fn measure(&mut self, _: &craft_ui::contract::MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn overlay(&mut self, _: &mut craft_ui::contract::OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        self.seen.set(self.seen.get() || craft_ui::testing::focus::context_request(&self.ctx, cx.id).is_some());
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
