//! `IconicColumn`: a column collapsed to one icon per panel, and the flyouts its icons open.

use super::dock_column::{ColumnGesture, GroupView, Laid, Shared, bar_name, locked, register_edges, show_bar};
use super::{DockSizes, DockSpot, DockTargets, DockWords, Flyout, PanelSpec};
use crate::buttons::IconButton;
use crate::contract::overlay::Popup;
use crate::contract::{
    Access, BodyCx, Common, Component, Container, ContainerProps, Cx, Name, NoValue, Outcome, OverlayUi, Overlays as _, Pad, PaintUi, PartsCx,
    Pointer as _, Props as _, Remember as _, Role, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::menu::GroupItem;
use crate::primitives::IconSpec;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::dock::{Column, DockEdge, Group};
use craft_behavior::{CursorIcon, DockSide, IconId, Metrics, PartKey, PressPhase, Sense, Value};
use egui::{Id, Pos2, Rect, Response, Ui, Vec2};
use std::sync::Arc;

#[cfg(feature = "testing")]
mod stories;

const CELL: PartKey = PartKey::new("cell");
const GRIP: PartKey = PartKey::new("grip");

/// The sizes of an iconic column on top of the column sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconicSizes {
    /// The sizes every column shares.
    pub column: DockSizes,
    /// The height of an icon cell (the rail button).
    pub cell: f32,
    /// How far an icon group's drag band reaches into its first icon.
    pub grip: f32,
    /// The room between two icon groups.
    pub separator: f32,
    /// The height of a flyout whose group has no height of its own.
    pub flyout_height: f32,
}

impl IconicSizes {
    /// The sizes of the style context `cx`.
    pub fn style(cx: &StyleCx<'_>) -> Self {
        let size = &cx.tokens.size;
        IconicSizes {
            column: DockSizes::style(cx),
            cell: size.rail.button,
            grip: size.handle.sm,
            separator: cx.tokens.space.gap.tight,
            flyout_height: size.plot.height,
        }
    }
}

/// What an iconic column reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct IconicColumnOutput<P> {
    /// The column's response.
    pub response: Response,
    /// The rect the column took, for `DockEvent::ColumnPlaced`.
    pub placed: Rect,
    /// The user asked to expand the column (`false`); the host sets `Column::iconic`.
    pub iconic: Option<bool>,
    /// The dock row the user picked in the column menu.
    pub picked: Option<GroupItem>,
    /// The flyout changed this pass: `Some(Some(panel))` when it opened or switched to `panel` (the host makes it
    /// its group's active panel), `Some(None)` when it closed. It comes once for each change.
    pub flyout: Option<Option<P>>,
    /// The gesture the user began on the column bar, an icon group's grip or an icon.
    pub gesture: Option<ColumnGesture<P>>,
}

impl<P> Outcome for IconicColumnOutput<P> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of an iconic column.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct IconicColumnStyle {
    #[style(metrics)]
    metrics: Metrics,
    sizes: IconicSizes,
}

/// The requests and reports the column compares across passes.
#[derive(Clone)]
struct Seen<P> {
    requested: Option<P>,
    reported: Option<P>,
}

impl<P> Default for Seen<P> {
    fn default() -> Self {
        Seen { requested: None, reported: None }
    }
}

/// A column collapsed to icons: the column bar over icon groups, one cell per panel. A click or Enter on an icon
/// opens the flyout of its panel beside the column, switches it to that panel or closes it; one flyout is open at a
/// time. A drag of an icon begins a move of its panel, a drag of the band on top of a group a move of the group, a
/// drag of the bar a move of the column; each is reported as a gesture and closes the flyout. Escape and a press
/// outside close the flyout; with `auto_collapse` the focus leaving it does too.
///
/// The host asks for a flyout with [`open`](IconicColumn::open) (opening a panel from the Window menu) and reads the
/// change in `IconicColumnOutput::flyout`. The column registers the drop bands of its edges and of the halves of
/// each icon group with the dock `dock`. The flyout shows its group through the `view` of
/// [`show_flyouts`](IconicColumn::show_flyouts), with the opened panel active.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container, overlay, probe(IconicColumn<'static, u8>))]
pub struct IconicColumn<'a, P: Value + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    dock: Id,
    column: &'a Column<P>,
    spec: &'a dyn Fn(&P) -> PanelSpec,
    /// The window edge the column sits on, which the flyout opens away from.
    #[prop]
    edge: DockEdge,
    /// Whether Auto-Collapse Iconic Panels is on.
    #[prop]
    auto_collapse: bool,
    /// The panel whose flyout the host asks to open. It acts when the value changes, so the host may keep passing it.
    #[prop(into)]
    open: Option<P>,
    view: Option<GroupView<'a, P>>,
    shared: Shared,
    shown: Option<usize>,
    flyout: Option<Option<P>>,
    gesture: Option<ColumnGesture<P>>,
    toggled: bool,
}

/// Where the parts of an iconic column sit.
struct Plan {
    bar: Rect,
    /// `(group, panel, rect)` of each icon cell, top to bottom.
    cells: Vec<(usize, usize, Rect)>,
    /// `(group, rect)` of each group's drag band.
    grips: Vec<(usize, Rect)>,
    /// `(group, rect)` of the cells of each group.
    groups: Vec<(usize, Rect)>,
    body: Rect,
}

impl<'a, P: Value + 'static> IconicColumn<'a, P> {
    /// A column named `label` of the dock `dock`, over `column`; `spec` reads each panel's label, icon and limits.
    pub fn new(label: impl Into<Label>, dock: Id, column: &'a Column<P>, spec: &'a dyn Fn(&P) -> PanelSpec) -> Self {
        let props = ContainerProps::new().surface(SurfaceKind::Panel).sizing(Sizing::Fill).pad(Pad::None);
        IconicColumn {
            common: Common::new(),
            props,
            dock,
            column,
            spec,
            edge: DockEdge::Start,
            auto_collapse: false,
            open: None,
            view: None,
            shared: Shared::default(),
            shown: None,
            flyout: None,
            gesture: None,
            toggled: false,
        }
        .label(label)
    }

    /// Shows the column; `view` shows the group of an open flyout in the rect it gets.
    pub fn show_flyouts(mut self, ui: &mut Ui, view: impl FnMut(&Group<P>, Rect, &mut Ui, &mut BodyCx<'_>) + 'a) -> IconicColumnOutput<P> {
        self.view = Some(Box::new(view));
        let (shared, auto_collapse) = (Arc::clone(&self.shared), self.auto_collapse);
        self.show(ui, move |ui, body| {
            if body.shell().is_some_and(|shell| !shell.dock_visible()) {
                return;
            }
            let Some(laid) = locked(&shared).laid.clone() else { return };
            // A column no wider than the rail has no room for the menu button; the context request opens the menu.
            show_bar(ui, body, laid.bar, false, auto_collapse, laid.menu_button, &shared);
        })
        .output
    }

    /// The panels of the column in order, each with its group.
    fn flat(&self) -> Vec<(usize, usize)> {
        self.column.groups.iter().enumerate().flat_map(|(group, g)| (0..g.panels.len()).map(move |panel| (group, panel))).collect()
    }

    fn panel(&self, group: usize, panel: usize) -> Option<&P> {
        self.column.groups.get(group).and_then(|group| group.panels.get(panel))
    }

    fn plan(&self, rect: Rect, sizes: &IconicSizes) -> Plan {
        let bar = Rect::from_min_max(rect.min, Pos2::new(rect.right(), (rect.top() + sizes.column.bar).min(rect.bottom())));
        let body = Rect::from_min_max(Pos2::new(rect.left(), bar.bottom()), rect.max);
        let (mut y, mut cells, mut grips, mut groups) = (body.top(), Vec::new(), Vec::new(), Vec::new());
        for (at, group) in self.column.groups.iter().enumerate() {
            if at > 0 {
                y += sizes.separator;
            }
            let top = y;
            for panel in 0..group.panels.len() {
                cells.push((at, panel, Rect::from_min_size(Pos2::new(body.left(), y), Vec2::new(body.width(), sizes.cell))));
                y += sizes.cell;
            }
            grips.push((at, Rect::from_min_max(Pos2::new(body.left(), top), Pos2::new(body.right(), (top + sizes.grip).min(y)))));
            groups.push((at, Rect::from_min_max(Pos2::new(body.left(), top), Pos2::new(body.right(), y))));
        }
        Plan { bar, cells, grips, groups, body }
    }

    /// The flyout of the cell `at`: the group with that cell's panel active.
    fn flyout_group(&self, at: usize) -> Option<Group<P>> {
        let (group, panel) = *self.flat().get(at)?;
        let mut shown = self.column.groups.get(group)?.clone();
        shown.active = panel;
        Some(shown)
    }
}

impl<P: Value + 'static> Component for IconicColumn<'_, P> {
    const ROLE: Role = Role::GROUP;
    type Style = IconicColumnStyle;
    type Output = IconicColumnOutput<P>;

    fn style(cx: &StyleCx<'_>, _: IconicColumnStyleKey<P>) -> IconicColumnStyle {
        IconicColumnStyle { metrics: Metrics::new(), sizes: IconicSizes::style(cx) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// The icon cells, each toggling the flyout of its panel, and the drag band of every group. The cells win the
    /// top of the first icon over the band, which is declared first.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let sizes = cx.style.sizes;
        let plan = self.plan(rect, &sizes);
        locked(&self.shared).laid = Some(Laid { bar: plan.bar, menu_button: plan.bar.width() > sizes.column.iconic_least });
        cx.part(super::dock_column::BAR, plan.bar)
            .sense(Sense::DRAG)
            .cursor(CursorIcon::Grab)
            .drag_cursor(CursorIcon::Grabbing)
            .pointer_only("Move Panel To in the panel menu moves the panels of the column");
        for (at, (_, _, cell)) in plan.cells.iter().enumerate() {
            let Some(panel) = self.flat().get(at).and_then(|(group, panel)| self.panel(*group, *panel)) else { continue };
            let label = (self.spec)(panel).label;
            cx.part(CELL.at(at as u64), *cell)
                .sense(Sense::CLICK | Sense::DRAG | Sense::TAKE_FOCUS)
                .focusable()
                .tooltip(label)
                .toggles_at("flyout", at);
        }
        for (group, grip) in &plan.grips {
            cx.part(GRIP.at(*group as u64), *grip)
                .sense(Sense::DRAG)
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only("Move Panel To in the panel menu moves the group's panels");
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let sizes = cx.style.sizes;
        let (rect, rtl, ctx) = (cx.rect, cx.flow.is_rtl(), cx.core.ctx.clone());
        let plan = self.plan(rect, &sizes);
        let flat = self.flat();
        let popup = cx.popup("flyout");
        let dragging =
            |phase: PressPhase| matches!(phase, PressPhase::DragStart { start, .. } if start.button == craft_behavior::PointerButton::Primary);
        let mut began = false;
        if dragging(cx.part(super::dock_column::BAR).press.phase)
            && let Some(anchor) = self.column.groups.first().and_then(|group| group.panels.first())
        {
            self.gesture = Some(ColumnGesture::Column { anchor: anchor.clone() });
            began = true;
        }
        self.toggled = cx.part(super::dock_column::BAR).press.double_pressed();
        for (group, _) in &plan.grips {
            if dragging(cx.part(GRIP.at(*group as u64)).press.phase)
                && let Some(anchor) = self.column.groups.get(*group).and_then(|group| group.panels.first())
            {
                self.gesture = Some(ColumnGesture::Group { anchor: anchor.clone() });
                began = true;
            }
        }
        for (at, (group, panel)) in flat.iter().enumerate() {
            if dragging(cx.part(CELL.at(at as u64)).press.phase)
                && let Some(panel) = self.panel(*group, *panel)
            {
                self.gesture = Some(ColumnGesture::Panel { panel: panel.clone() });
                began = true;
            }
        }
        let (requested, reported) = {
            let seen = cx.memory::<Seen<P>>();
            (seen.requested.clone(), seen.reported.clone())
        };
        if self.open != requested {
            if let Some(at) = self.open.as_ref().and_then(|open| flat.iter().position(|(group, panel)| self.panel(*group, *panel) == Some(open))) {
                popup.at(at).open(cx);
            }
            cx.memory::<Seen<P>>().requested = self.open.clone();
        }
        let opens: Vec<bool> = (0..flat.len()).map(|at| popup.at(at).is_open(cx)).collect();
        if began {
            for (at, open) in opens.iter().enumerate() {
                if *open {
                    popup.at(at).close(cx);
                }
            }
        }
        let now = opens.iter().position(|open| *open && !began);
        self.shown = now;
        let now_panel = now.and_then(|at| flat.get(at)).and_then(|(group, panel)| self.panel(*group, *panel)).cloned();
        if now_panel != reported {
            cx.memory::<Seen<P>>().reported = now_panel.clone();
            self.flyout = Some(now_panel);
        }
        self.register(&ctx, rect, rtl, &plan, &sizes);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        let flat = self.flat();
        for (at, (group, panel)) in flat.iter().enumerate() {
            let Some(panel) = self.panel(*group, *panel) else { continue };
            let spec = (self.spec)(panel);
            let icon: IconSpec<'static> = spec.icon.unwrap_or_else(|| IconId::new("more").into());
            ui.face(CELL.at(at as u64), IconButton::new(icon).label(spec.label).selected(self.shown == Some(at)));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<DockWords>();
        let sizes = cx.style.sizes;
        let plan = self.plan(cx.rect, &sizes);
        access.node(super::dock_column::BAR, Role::BUTTON).name = Name::Text(bar_name(self.column, self.spec, &words.move_column));
        for (at, (group, panel)) in self.flat().iter().enumerate() {
            let Some(panel) = self.panel(*group, *panel) else { continue };
            access.node(CELL.at(at as u64), Role::BUTTON).name = Name::Text((self.spec)(panel).label.as_str().to_owned());
        }
        for (group, _) in &plan.grips {
            let Some(shown) = self.column.groups.get(*group).and_then(|group| group.active_panel()) else { continue };
            let name = words.move_group.fill(&[("name", (self.spec)(shown).label.as_str())]);
            access.node(GRIP.at(*group as u64), Role::BUTTON).name = Name::Text(name);
        }
    }

    /// The flyout of every cell whose popup is open, beside the column.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("flyout");
        let sizes = cx.style.sizes;
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let plan = self.plan(rect, &sizes);
        let width = sizes.column.expanded_width(self.column, self.spec);
        let (edge, auto_collapse) = (self.edge, self.auto_collapse);
        let groups: Vec<Option<Group<P>>> = (0..self.flat().len()).map(|at| self.flyout_group(at)).collect();
        let Some(view) = self.view.as_mut() else { return };
        for (at, group) in groups.into_iter().enumerate() {
            let (Some(group), Some((_, _, cell))) = (group, plan.cells.get(at)) else { continue };
            let height = group.height.filter(|height| height.is_finite() && *height > 0.0).unwrap_or(sizes.flyout_height);
            let height = height.max(sizes.column.strip + sizes.column.least);
            Flyout::new(edge, Vec2::new(width, height), auto_collapse)
                .show(ui, popup.at(at), *cell, rtl, |ui, body, rect| view(&group, rect, ui, body));
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> IconicColumnOutput<P> {
        let bar = std::mem::take(&mut locked(&self.shared).bar);
        if let Some(opener) = bar.opener {
            Popup::toggle(opener, &mut cx);
        }
        let iconic = bar.iconic.or(self.toggled.then_some(false));
        IconicColumnOutput { response: cx.response.clone(), placed: cx.rect, iconic, picked: bar.picked, flyout: self.flyout, gesture: self.gesture }
    }
}

impl<P: Value + 'static> IconicColumn<'_, P> {
    /// Registers the drop bands of the column's edges, and the upper and lower half of every icon group: a drop on
    /// the upper half stacks above the group, one on the lower half below it.
    fn register(&self, ctx: &egui::Context, rect: Rect, rtl: bool, plan: &Plan, sizes: &IconicSizes) {
        let Some(first) = self.column.groups.first().and_then(|group| group.panels.first()) else { return };
        register_edges(ctx, self.dock, first, rect, rtl, &sizes.column);
        let half = sizes.separator * 0.5;
        let last = plan.groups.len().saturating_sub(1);
        for (at, group) in &plan.groups {
            let Some(anchor) = self.column.groups.get(*at).and_then(|group| group.panels.first()) else { continue };
            let middle = group.center().y;
            let upper_top = if *at == 0 { plan.body.top() } else { group.top() - half };
            let lower_bottom = if *at == last { plan.body.bottom() } else { group.bottom() + half };
            let upper = Rect::from_x_y_ranges(group.x_range(), upper_top..=middle);
            let lower = Rect::from_x_y_ranges(group.x_range(), middle..=lower_bottom);
            DockTargets::register(ctx, self.dock, DockSpot::group(anchor.clone(), DockSide::Top), upper);
            DockTargets::register(ctx, self.dock, DockSpot::group(anchor.clone(), DockSide::Bottom), lower);
        }
    }
}

impl<P: Value + 'static> Container for IconicColumn<'_, P> {}
