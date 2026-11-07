//! `DockGroup`: a group of panels as a container: a header of a collapse arrow, a grip, the panels' tab strip and
//! the panel menu button over the body of the active panel.

use super::targets::{DockSizes, DockSpot, DockTargets, GroupDrag, ZoneRule};
use super::words::DockWords;
use super::{PanelDrag, PanelKey, PanelTitle};
use crate::buttons::IconButton;
use crate::contract::{
    Bind, Common, Component, Container, ContainerProps, ContextMenuRequest, Cx, HeaderRegion, ListDrag, MeasureCx, Outcome, PaintUi, Pointer as _,
    Props as _, Role, Show as _, ShowCollection as _, ShowContainer as _, Sizing, Styled as _, Trigger, WordSetCx as _,
};
use crate::controls::disclosure::Disclosure;
use crate::layout_components::Row;
use crate::menu::{GroupItem, Menu, MenuWords, PanelMenu, PanelRow};
use crate::primitives::IconSpec;
use crate::runtime::access::PopupKind;
use crate::runtime::drag::DragPreview;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, IconLook, Ink, Stateful, StyleCx, SurfaceKind};
use crate::tabs::{Tab, TabKind, TabStrip, TornOff};
use craft_behavior::{IconId, Metrics, PressPhase, Sense, keyed};
use egui::{Id, Pos2, Rect, Response, Ui, Vec2};
use std::borrow::Cow;
use std::hash::Hash;

/// The style of a group grip: the icon box.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct GroupGripStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(icon)]
    glyph: IconLook,
}

/// What a group grip reports: its response, the second press of a double click and whether it started a group
/// drag this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct GroupGripOutput {
    /// The grip's response.
    pub response: Response,
    /// The second press of a double click went down on the grip this pass.
    pub double_pressed: bool,
    /// A drag of the whole group started on the grip this pass.
    pub drag_started: bool,
}

impl Outcome for GroupGripOutput {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The handle a group is dragged by. A drag carries a `GroupDrag` of the group's anchor; a double click is reported
/// for the group to minimize.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct GroupGrip {
    #[common]
    common: Common,
    anchor: PanelKey,
    active: PanelKey,
    #[prop]
    movable: bool,
    double: bool,
    started: bool,
}

impl GroupGrip {
    /// The grip of the group named by `anchor`, showing `active`.
    pub fn new(anchor: PanelKey, active: PanelKey) -> Self {
        GroupGrip { common: Common::new(), anchor, active, movable: true, double: false, started: false }
    }
}

impl Component for GroupGrip {
    const ROLE: Role = Role::BUTTON;
    const PARTS: bool = false;
    const SHOWS_LABEL: bool = false;
    const OVERLAYS: bool = false;
    type Style = GroupGripStyle;
    type Output = GroupGripOutput;

    fn style(cx: &StyleCx<'_>, _: GroupGripStyleKey) -> GroupGripStyle {
        let side = cx.control().icon;
        GroupGripStyle { metrics: Metrics::new().min_size(Vec2::splat(side)), glyph: IconLook::new(side, Stateful::new(cx.ink(Ink::OnPanel))) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.style.glyph.side)
    }

    fn sense(&self) -> Sense {
        if self.movable { Sense::CLICK | Sense::DRAG | Sense::FOCUS } else { Sense::CLICK | Sense::FOCUS }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.double = cx.press().double_pressed();
        if self.movable && matches!(cx.press().phase, PressPhase::DragStart { .. }) {
            let preview = DragPreview::frame(Vec2::splat(cx.style.glyph.side), None);
            cx.start_drag(GroupDrag { anchor: self.anchor, active: self.active }, preview);
            self.started = true;
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let mut mark = IconSpec::svg(IconId::new("grip"));
        mark.color = Some(cx.resolve(&cx.style.glyph.color));
        ui.draw().icon(egui::Rect::from_center_size(cx.rect.center(), Vec2::splat(cx.style.glyph.side)), &mark);
    }

    fn finish(self, cx: Cx<'_, Self>) -> GroupGripOutput {
        GroupGripOutput { response: cx.response.clone(), double_pressed: self.double, drag_started: self.started }
    }
}

/// A closure that adds the host's items to a group's header, after the tab strip.
pub struct HeaderHook<'a>(Box<dyn FnMut(&mut Ui) + 'a>);

impl<'a> HeaderHook<'a> {
    /// A hook that runs `add` in the header's row.
    pub fn new(add: impl FnMut(&mut Ui) + 'a) -> Self {
        HeaderHook(Box::new(add))
    }

    fn run(&mut self, ui: &mut Ui) {
        (self.0)(ui);
    }
}

/// What a dock group reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockGroupOutput {
    /// The group's response.
    pub response: Response,
    /// The panel whose tab was picked this pass (a click, or the overflow menu); the app makes it the group's active
    /// panel.
    pub picked: Option<PanelKey>,
    /// The panels whose close button or middle click closed their tab this pass.
    pub closed: Vec<PanelKey>,
    /// A context request on a tab (right click, the Menu key, Shift+F10): the group menu opened at the pointer.
    pub menu: Option<ContextMenuRequest<PanelKey>>,
    /// The group menu row that ran this pass; dock commands map it to the action.
    pub item: Option<GroupItem>,
    /// The collapse arrow or a double click on the grip asked to minimize or expand the group.
    pub toggle_minimized: bool,
    /// A tab was dragged out of the dock's windows.
    pub torn_off: Option<TornOff<PanelKey>>,
    /// A drag of the whole group started on the grip.
    pub drag_started: bool,
}

impl Outcome for DockGroupOutput {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of a dock group: the header height and the sizes of the spots it registers.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DockGroupStyle {
    #[style(metrics)]
    metrics: Metrics,
    header: f32,
    sizes: DockSizes,
}

#[derive(Default)]
struct Reported {
    picked: Option<PanelKey>,
    closed: Vec<PanelKey>,
    menu: Option<ContextMenuRequest<PanelKey>>,
    item: Option<GroupItem>,
    toggle_minimized: bool,
    torn_off: Option<TornOff<PanelKey>>,
    drag_started: bool,
}

/// A group of panels: a header of collapse arrow, grip, tab strip and panel menu button over the active panel's
/// body. Show it with `show_at(group_rect)` in the body of a `SplitArea`, with an `IdSource::Id` made from the
/// group's anchor so its state survives a re-dock. Under a `dock` id it registers the spots a drop lands on: the
/// strip, and (while expanded) the sides and the middle of the body.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container, probe(DockGroup<'static, usize>))]
pub struct DockGroup<'a, P: Clone + Hash + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    anchor: P,
    panels: &'a [PanelTitle],
    active: PanelKey,
    /// Only the header shows; the body and its drop spots are left out.
    #[prop(flag)]
    minimized: bool,
    /// The grip drags the group.
    #[prop]
    movable: bool,
    /// Tabs show a close button and close on a middle click.
    #[prop(flag)]
    closable: bool,
    /// The dock whose drop registry the group's spots go to.
    #[prop]
    dock: Option<Id>,
    /// How the body's side zones are sized.
    #[prop]
    zones: ZoneRule,
    /// The rows of the group menu: the panel menu button and a context request on a tab open them.
    #[prop]
    menu: Vec<PanelRow>,
    /// The host's items after the tab strip.
    #[prop(func = "FnMut(&mut egui::Ui)")]
    extra: Option<HeaderHook<'a>>,
    reported: Reported,
}

impl<'a, P: Clone + Hash + Send + Sync + 'static> DockGroup<'a, P> {
    /// The group named by `anchor` (one of its panels) with its `panels`, `active` showing. The group is named by the
    /// active panel's title.
    pub fn new(anchor: P, panels: &'a [PanelTitle], active: PanelKey) -> Self {
        let props = ContainerProps::new().surface(SurfaceKind::Panel).sizing(Sizing::Fill).header(HeaderRegion::new().rule(true).trailing(true));
        let title = panels.iter().find(|panel| panel.key == active).map(|panel| panel.title.clone()).unwrap_or_default();
        DockGroup {
            common: Common::new(),
            props,
            anchor,
            panels,
            active,
            minimized: false,
            movable: true,
            closable: false,
            dock: None,
            zones: ZoneRule::Fraction,
            menu: Vec::new(),
            extra: None,
            reported: Reported::default(),
        }
        .label(title)
    }
}

impl<P: Clone + Hash + Send + Sync + 'static> Component for DockGroup<'_, P> {
    const ROLE: Role = Role::TAB_PANEL;
    const PARTS: bool = false;
    type Style = DockGroupStyle;
    type Output = DockGroupOutput;

    fn style(cx: &StyleCx<'_>, _: DockGroupStyleKey<P>) -> DockGroupStyle {
        DockGroupStyle { metrics: Metrics::new(), header: cx.tokens.size.bar.tab, sizes: DockSizes::style(cx) }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(dock) = self.dock else { return };
        let (rect, rtl, header) = (cx.rect, cx.flow.is_rtl(), cx.style.header);
        let strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header));
        DockTargets::<P>::register(&cx.core.ctx, dock, DockSpot::tab(self.anchor.clone(), self.panels.len()), strip);
        if self.minimized {
            return;
        }
        let body = Rect::from_min_max(Pos2::new(rect.left(), strip.bottom()), rect.max);
        for (side, zone) in self.zones.zones(body, &cx.style.sizes, rtl) {
            DockTargets::<P>::register(&cx.core.ctx, dock, DockSpot::group(self.anchor.clone(), side), zone);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> DockGroupOutput {
        let Reported { picked, closed, menu, item, toggle_minimized, torn_off, drag_started } = self.reported;
        DockGroupOutput { response: cx.response.clone(), picked, closed, menu, item, toggle_minimized, torn_off, drag_started }
    }
}

impl<P: Clone + Hash + Send + Sync + 'static> Container for DockGroup<'_, P> {
    fn header(&mut self, ui: &mut Ui, cx: &mut crate::contract::BodyCx<'_>) {
        let words = cx.words::<DockWords>();
        let menu_words = cx.words::<MenuWords>();
        let from = PanelKey::of(&self.anchor);
        let (panels, active, closable, movable, minimized) = (self.panels, self.active, self.closable, self.movable, self.minimized);
        let menu_rows = self.menu.clone();
        let panel_menu = words.panel_menu.clone();
        let mut extra = self.extra.take();
        let (name, move_name) = (Label::from(words.panels.as_str().to_owned()), words.move_group.fill(&[]));
        let reported = &mut self.reported;
        Row::new().show(ui, |ui, _| {
            let mut open = !minimized;
            let arm_name = if minimized { menu_words.panel_expand.clone() } else { menu_words.panel_minimize.clone() };
            Disclosure::new(Bind::bound(&mut open)).label(arm_name).show(ui);
            if open == minimized {
                reported.toggle_minimized = true;
            }
            let grip = GroupGrip::new(from, active).movable(movable).label(move_name.clone()).show(ui);
            reported.toggle_minimized |= grip.double_pressed;
            reported.drag_started |= grip.drag_started;

            let source = keyed(panels, |panel: &PanelTitle| panel.key).with_text(|panel| Cow::Borrowed(panel.title.as_str()));
            let kind = if closable { TabKind::Document } else { TabKind::Panel };
            let strip = TabStrip::new(name.clone())
                .kind(kind)
                .shown(active)
                .tear_off()
                .drag_payload(move |drag: &ListDrag<PanelKey>| Some(PanelDrag { panel: drag.grabbed, from }))
                .show_items(ui, &source, &mut |key, _| {
                    let title = panels.iter().find(|panel| &panel.key == key).map_or_else(Label::default, |panel| panel.title.clone());
                    Tab::new(title).closable(closable)
                });
            for event in strip.response.events {
                if let crate::contract::CollectionEvent::Selection(craft_behavior::SelectionChange::Set(keys)) = event {
                    reported.picked = keys.last().copied().or(reported.picked);
                }
            }
            reported.picked = strip.picked.or(reported.picked);
            reported.closed.extend(strip.closed);
            reported.menu = strip.menu.or(reported.menu.take());
            reported.torn_off = strip.torn_off.or(reported.torn_off.take());
            if !menu_rows.is_empty() {
                let rows = menu_rows.clone();
                let shown = Menu::context(panel_menu.clone(), &strip.response.response)
                    .show(ui, |ui, _| PanelMenu::new(Vec::new(), rows.clone()).show(ui).picked);
                reported.item = shown.inner.flatten().or(reported.item);
            }
            if let Some(hook) = extra.as_mut() {
                hook.run(ui);
            }
        });
        self.extra = extra;
    }

    fn controls(&mut self, ui: &mut Ui, cx: &mut crate::contract::BodyCx<'_>) {
        if self.menu.is_empty() {
            return;
        }
        let name = cx.words::<DockWords>().panel_menu.clone();
        let button = IconButton::new(IconId::new("more")).popup(PopupKind::Menu).label(name.clone()).show(ui);
        let rows = self.menu.clone();
        let shown = Menu::new(name).opened_by(&button, Trigger::Click).show(ui, |ui, _| PanelMenu::new(Vec::new(), rows.clone()).show(ui).picked);
        self.reported.item = shown.inner.flatten().or(self.reported.item);
    }
}
