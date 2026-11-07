//! `Menu` (11.4, 15.4): the container of menu rows, on a menu layer, inline, or as a submenu.

use super::item::{LevelMemory, MenuItem, MenuLevel, level_env};
use crate::contract::overlay::{Anchor, Dismiss, Outside};
use crate::contract::{
    Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Entry, Mount, NoValue, Outcome, Overlay, Pad, Props as _, Remember as _, Role,
    Show as _, ShowContainer as _, Trigger,
};
use crate::display::Divider;
use crate::runtime::env::Label;
use crate::runtime::overlay::{CloseReason, request_close};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, NavShape, Roving, Sense, Value};
use egui::{Response, Ui};

/// The style of a menu: the surface comes from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct MenuStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a menu returns: its response and how it closed in this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct MenuOutput {
    /// The menu's response.
    pub response: Response,
    /// How the menu closed in this pass; `None` while it stays open. Reported once for each close.
    pub closed: Option<CloseReason>,
}

impl Outcome for MenuOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// What [`Menu::pick`] returns: the response, how the menu closed and the value of the entry that ran.
#[derive(Debug)]
#[non_exhaustive]
pub struct MenuPick<T> {
    /// The menu's response.
    pub response: Response,
    /// How the menu closed in this pass.
    pub closed: Option<CloseReason>,
    /// The value of the entry that ran this pass.
    pub picked: Option<T>,
}

impl<T: Value> Outcome for MenuPick<T> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The layer of a menu, or of a submenu: a press outside closes it and reaches nothing below, and running a row
/// closes the chain.
pub(crate) fn layer(submenu: bool) -> Overlay {
    let base = if submenu { Overlay::submenu() } else { Overlay::menu() };
    base.dismiss(Dismiss::new().outside(Outside::Absorb).on_activate(true))
}

/// Shows one row per entry, a rule for a separator, and returns the index of the entry that ran. A checked entry
/// shows as the radio mark of a value menu.
pub(crate) fn entry_rows<T>(ui: &mut Ui, entries: &[Entry<T>]) -> Option<usize> {
    let valued = entries.iter().any(Entry::is_checked);
    let mut ran = None;
    for (at, entry) in entries.iter().enumerate() {
        if entry.is_separator() {
            Divider::horizontal().show(ui);
            continue;
        }
        let mut row = MenuItem::new(entry.label().clone()).enabled(entry.is_enabled());
        if valued {
            row = row.radio_mark(entry.is_checked());
        }
        if let Some(icon) = entry.icon_spec() {
            row = row.icon(icon.clone());
        }
        if row.show(ui).ran {
            ran = Some(at);
        }
    }
    ran
}

/// The value menu that hangs from `opener`: a click on the opener opens it, and it shows one row per entry. The open
/// state stays in egui's temporary memory under the opener's id, until the contract lets a menu open without a binding.
pub(crate) fn pick_from<T: Value + 'static>(ui: &mut Ui, label: impl Into<Label>, opener: &Response, entries: &[Entry<T>]) -> MenuPick<T> {
    let state = opener.id.with("menu_open");
    let mut open = crate::contract::clicked(opener) || ui.data(|data| data.get_temp::<bool>(state)).unwrap_or(false);
    let picked = Menu::new(label, Bind::bound(&mut open)).opened_by(opener).pick(ui, entries);
    ui.data_mut(|data| data.insert_temp(state, open));
    picked
}

/// A row that opens a submenu showing the rows of `body`. The row's request opens the submenu; the open state stays
/// in egui's temporary memory under the row's id, because a menu body here has no component memory of its own.
pub(crate) fn submenu_rows(ui: &mut Ui, label: impl Into<Label>, mut body: impl FnMut(&mut Ui)) {
    let label = label.into();
    let head = MenuItem::submenu(label.clone()).show(ui);
    let state = head.response.id.with("submenu_open");
    let mut open = head.opens_submenu || ui.data(|data| data.get_temp::<bool>(state)).unwrap_or(false);
    Menu::submenu(label, &head.response, Bind::bound(&mut open)).show(ui, |ui, _| body(ui));
    ui.data_mut(|data| data.insert_temp(state, open));
}

/// A menu: a popup of [`MenuItem`]s and [`Divider`]s the arrow keys move through, one roving group. Press, context
/// and submenu menus are this container with a different opener.
///
/// - Open state: the caller's binding. Escape, a press outside, running a row and the other close rules write
///   `false`, and the output's `closed` says which one.
/// - Opener: [`opened_by`](Menu::opened_by) names the control it hangs from; focus returns to it on a close by key.
///   A context menu hangs from the pointer ([`context`](Menu::context)), a submenu from its row
///   ([`submenu`](Menu::submenu)), which asks for it through `MenuItemOutput::opens_submenu`.
/// - Every row of the body registers with the menu's level: the level gives each its mnemonic.
/// - A menu over value entries shows with [`pick`](Menu::pick).
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct Menu<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    open: Option<Bind<'a, bool>>,
    closed: Option<CloseReason>,
    submenu: bool,
}

impl<'a> Menu<'a> {
    /// A menu named `label`, open while `open` is true.
    pub fn new(label: impl Into<Label>, open: Bind<'a, bool>) -> Self {
        let props =
            ContainerProps::new().surface(SurfaceKind::Popup).pad(Pad::None).roving(Roving::new(NavShape::Flat)).mount(Mount::Layer(layer(false)));
        Menu { common: Common::new(), props, open: Some(open), closed: None, submenu: false }.label(label)
    }

    /// The rows of a menu in the flow of the layer that shows them, with no surface and no close rules of their own:
    /// the menu of a control that owns its popup shows its entries through it.
    pub fn inline(label: impl Into<Label>) -> Self {
        let props = ContainerProps::new().roving(Roving::new(NavShape::Flat));
        Menu { common: Common::new(), props, open: None, closed: None, submenu: false }.label(label)
    }

    /// A context menu named `label`, hanging from where the pointer was when it opened, with `opener` getting the
    /// focus back when it closes by key.
    pub fn context(label: impl Into<Label>, opener: &Response, open: Bind<'a, bool>) -> Self {
        Menu::new(label, open).with_layer(|overlay| overlay.opened_by(opener, Trigger::Context).anchor(Anchor::Pointer))
    }

    /// The submenu of the row `row`, beside it. It closes with the menu the row sits in.
    pub fn submenu(label: impl Into<Label>, row: &Response, open: Bind<'a, bool>) -> Self {
        let mut menu = Menu::new(label, open).with_layer(|_| layer(true).opened_by(row, Trigger::Manual));
        menu.submenu = true;
        menu
    }

    /// The control the menu hangs from. The menu closes when the opener is no longer shown.
    #[must_use]
    pub fn opened_by(self, opener: &Response) -> Self {
        self.with_layer(|overlay| overlay.opened_by(opener, Trigger::Manual))
    }

    fn with_layer(mut self, change: impl FnOnce(Overlay) -> Overlay) -> Self {
        let overlay = match std::mem::take(&mut self.props.mount) {
            Mount::Layer(overlay) => overlay,
            _ => layer(false),
        };
        self.props = self.props.mount(Mount::Layer(change(overlay)));
        self
    }

    /// Shows one row per entry (a rule for a separator) and reports the value of the entry that ran. A checked entry
    /// carries the radio mark.
    pub fn pick<T: Value + 'static>(self, ui: &mut Ui, entries: &[Entry<T>]) -> MenuPick<T> {
        let shown = self.show(ui, |ui, _| entry_rows(ui, entries));
        let picked = shown.inner.flatten().and_then(|at| entries.get(at)).and_then(Entry::value).cloned();
        MenuPick { response: shown.output.response, closed: shown.output.closed, picked }
    }
}

impl Component for Menu<'_> {
    const ROLE: Role = Role::MENU;
    type Style = MenuStyle;
    type Output = MenuOutput;

    fn style(_: &StyleCx<'_>, _: MenuStyleKey) -> MenuStyle {
        MenuStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// A row of a submenu level that took Back asks the menu to close.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let level = MenuLevel::of(cx.memory::<LevelMemory>());
        if level.take_close() {
            request_close(&cx.core.ctx, cx.id, CloseReason::Requested);
        }
    }

    fn content_env(&self, cx: &Cx<'_, Self>, values: &mut crate::runtime::env::AppValues) {
        level_env(cx, values, self.submenu);
    }

    fn finish(self, cx: Cx<'_, Self>) -> MenuOutput {
        MenuOutput { response: cx.response.clone(), closed: self.closed }
    }
}

impl Container for Menu<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
