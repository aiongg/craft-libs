//! `BarItems`: a strip of command items that overflows into a menu of command rows.

use super::words::BarWords;
use crate::buttons::{Button, IconButton, ToggleButton};
use crate::contract::{
    Access, Common, Component, Cx, MeasureCx, Name, OverlayUi, Overlays as _, PaintUi, PartsCx, Props as _, Role, Show as _, ShowContainer as _,
    WordSetCx as _,
};
use crate::display::Divider;
use crate::menu::{Menu, MenuItem};
use crate::primitives::IconSpec;
use crate::runtime::commands::{CommandRef, ResolvedCommand};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Axis, IconId, Metrics, NavShape, PartKey, PartRef, Roving, Sense};
use craft_theme::Variant;
use egui::{Rect, Response, Vec2, pos2};

const ITEM: PartKey = PartKey::new("item");
const MORE: PartKey = PartKey::new("more");

/// What an item shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemDisplay {
    /// The command's icon; the label is the tooltip and the accessible name. A command with no icon shows `Text`.
    #[default]
    Icon,
    /// The command's label.
    Text,
    /// The command's icon before its label. A command with no icon shows `Text`.
    IconText,
}

/// Whether an item may move into the overflow menu, or lives there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FitKeep {
    /// The item leaves the strip when the room runs out, last item first.
    #[default]
    Free,
    /// The item stays in the strip however narrow it gets.
    Pinned,
    /// The item is only in the menu, whatever the room: the strip always shows its "More" part.
    Menu,
}

/// One command of a `BarItems` strip.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BarItem {
    /// The command the item shows and runs.
    pub command: CommandRef,
    /// The section: a rule separates neighbouring items of different sections.
    pub section: u16,
    /// What the item shows.
    pub display: ItemDisplay,
    /// Whether the item may overflow.
    pub keep: FitKeep,
}

impl BarItem {
    /// An item for `command` in section 0, shown as its icon.
    pub fn new(command: impl Into<CommandRef>) -> Self {
        BarItem { command: command.into(), section: 0, display: ItemDisplay::Icon, keep: FitKeep::Free }
    }

    /// Puts the item in `section`.
    #[must_use]
    pub const fn section(mut self, section: u16) -> Self {
        self.section = section;
        self
    }

    /// Sets what the item shows.
    #[must_use]
    pub const fn display(mut self, display: ItemDisplay) -> Self {
        self.display = display;
        self
    }

    /// Sets whether the item may overflow.
    #[must_use]
    pub const fn keep(mut self, keep: FitKeep) -> Self {
        self.keep = keep;
        self
    }
}

/// One item as the fit rule sees it: its extent along the strip, its section and whether it may overflow.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FitItem {
    /// The item's extent along the strip, in points.
    pub extent: f32,
    /// The item's section.
    pub section: u16,
    /// Whether the item may overflow.
    pub keep: FitKeep,
}

impl FitItem {
    /// An item `extent` long in `section`.
    pub const fn new(extent: f32, section: u16, keep: FitKeep) -> Self {
        FitItem { extent, section, keep }
    }
}

/// Which items the strip shows and which go to the overflow menu, as indexes into the input, in input order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Fit {
    /// The items the strip shows.
    pub shown: Vec<usize>,
    /// The items in the overflow menu; empty when everything fits.
    pub overflow: Vec<usize>,
}

fn extent(value: f32) -> f64 {
    if value.is_finite() { f64::from(value.max(0.0)) } else { 0.0 }
}

/// The length of the items marked in `shown`, with `gap` between neighbours of a section and `section_gap` between
/// neighbours of different sections.
fn run_extent(items: &[FitItem], shown: &[bool], gap: f64, section_gap: f64) -> f64 {
    let mut total = 0.0;
    let mut previous: Option<u16> = None;
    for (item, _) in items.iter().zip(shown).filter(|(_, shown)| **shown) {
        if let Some(section) = previous {
            total += if section == item.section { gap } else { section_gap };
        }
        total += extent(item.extent);
        previous = Some(item.section);
    }
    total
}

/// Decides which items stay in a strip `available` long. Everything fits and no item is `Menu`: all items show.
/// Otherwise the `Menu` items are in the menu, and the last `Free` item leaves first, one at a time, until the rest
/// and an overflow control `overflow_extent` long, set off by `gap`, fit; `Pinned` items stay even when they alone
/// are longer than `available`. Negative and non-finite lengths count as zero.
pub fn fit_items(items: &[FitItem], available: f32, gap: f32, section_gap: f32, overflow_extent: f32) -> Fit {
    let (available, gap, section_gap, overflow_extent) = (extent(available), extent(gap), extent(section_gap), extent(overflow_extent));
    let mut shown: Vec<bool> = items.iter().map(|item| item.keep != FitKeep::Menu).collect();
    if shown.iter().all(|shown| *shown) && run_extent(items, &shown, gap, section_gap) <= available {
        return Fit { shown: (0..items.len()).collect(), overflow: Vec::new() };
    }
    let fits = |shown: &[bool]| {
        let before_control = if shown.iter().any(|shown| *shown) { gap } else { 0.0 };
        run_extent(items, shown, gap, section_gap) + before_control + overflow_extent <= available
    };
    while !fits(&shown) {
        let Some(last) = (0..items.len()).rev().find(|&at| shown[at] && items[at].keep == FitKeep::Free) else { break };
        shown[last] = false;
    }
    let (kept, left): (Vec<usize>, Vec<usize>) = (0..items.len()).partition(|&at| shown[at]);
    Fit { shown: kept, overflow: left }
}

/// The style of a bar item strip: the gap between items and the space around the rule of a section change.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BarItemsStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// Between an item and the rule of a section change.
    joint: f32,
    /// The clear space of a section change, on both sides of its rule together.
    section_gap: f32,
}

/// An item whose command resolved this pass.
#[derive(Clone)]
struct Present {
    item: BarItem,
    resolved: ResolvedCommand,
}

/// Where the parts of a strip sit this pass.
struct Laid {
    present: Vec<Present>,
    shown: Vec<(usize, Rect)>,
    rules: Vec<Rect>,
    overflow: Vec<usize>,
    more: Option<Rect>,
}

/// The sizes the faces of a strip measure.
struct Sizes {
    items: Vec<Vec2>,
    rule: f32,
    more: Vec2,
}

/// What an item's part wears.
enum Face {
    Action(IconButton<'static>),
    Toggle(ToggleButton<'static>),
    Text(Button<'static>),
}

impl Face {
    fn size<C: Component>(self, cx: &MeasureCx<'_, C>) -> Vec2 {
        match self {
            Face::Action(face) => cx.face(face),
            Face::Toggle(face) => cx.face(face),
            Face::Text(face) => cx.face(face),
        }
    }

    fn paint(self, ui: &mut PaintUi<'_>, part: PartRef) {
        match self {
            Face::Action(face) => ui.face(part, face),
            Face::Toggle(face) => ui.face(part, face),
            Face::Text(face) => ui.face(part, face),
        }
    }
}

/// The icon of a command, when the registry names one.
fn icon_of(resolved: &ResolvedCommand) -> Option<IconSpec<'static>> {
    resolved.icon.clone().map(|name| IconSpec::svg(IconId::from(name)))
}

/// What the item shows: its display, or its label when the command has no icon for it.
fn display_of(present: &Present) -> ItemDisplay {
    if present.item.display != ItemDisplay::Text && icon_of(&present.resolved).is_none() { ItemDisplay::Text } else { present.item.display }
}

/// The face of an item: an icon button, a toggle for a command with a checked state, or a text button.
fn face_of(present: &Present) -> Face {
    let (item, resolved) = (&present.item, &present.resolved);
    let icon = icon_of(resolved);
    let toggle = resolved.checked.is_some();
    match (display_of(present), icon) {
        (ItemDisplay::Icon, Some(icon)) if toggle => Face::Toggle(ToggleButton::command(item.command.clone()).icon(icon).icon_only()),
        (ItemDisplay::Icon, Some(icon)) => Face::Action(IconButton::command(icon, item.command.clone())),
        (ItemDisplay::IconText, Some(icon)) if toggle => Face::Toggle(ToggleButton::command(item.command.clone()).icon(icon)),
        (ItemDisplay::IconText, Some(icon)) => Face::Text(Button::command(item.command.clone()).variant(Variant::GHOST).icon(icon)),
        (_, _) if toggle => Face::Toggle(ToggleButton::command(item.command.clone())),
        (_, _) => Face::Text(Button::command(item.command.clone()).variant(Variant::GHOST)),
    }
}

fn along(axis: Axis, size: Vec2) -> f32 {
    match axis {
        Axis::Y => size.y,
        _ => size.x,
    }
}

fn across(axis: Axis, size: Vec2) -> f32 {
    match axis {
        Axis::Y => size.x,
        _ => size.y,
    }
}

/// The part of `rect` that starts `from` points from the strip's start and runs `length` along it, across the
/// whole rect.
fn slot(rect: Rect, axis: Axis, rtl: bool, from: f32, length: f32) -> Rect {
    match axis {
        Axis::Y => Rect::from_min_max(pos2(rect.left(), rect.top() + from), pos2(rect.right(), rect.top() + from + length)),
        _ if rtl => Rect::from_min_max(pos2(rect.right() - from - length, rect.top()), pos2(rect.right() - from, rect.bottom())),
        _ => Rect::from_min_max(pos2(rect.left() + from, rect.top()), pos2(rect.left() + from + length, rect.bottom())),
    }
}

/// A strip of command items. Each item is a part that wears the face of a button resolved from its command: an
/// icon button, a toggle for a command with a checked state, or a text button. The items are one Tab stop with
/// arrow keys between them, and the strip joins the roving group of the body it is shown in. An icon item's tooltip
/// is the command's label and its chord. When the room is too short, the last `Free` items move into a menu behind
/// a "More" part, as rows of the same commands; `Menu` items are always there.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay)]
pub struct BarItems {
    #[common]
    common: Common,
    items: Vec<BarItem>,
    /// The axis the strip runs along.
    #[prop]
    axis: Axis,
    laid: Option<Laid>,
}

impl BarItems {
    /// A strip of `items`, in the order given, running along the horizontal axis.
    pub fn new(items: Vec<BarItem>) -> Self {
        BarItems { common: Common::new(), items, axis: Axis::X, laid: None }
    }

    /// The items whose command the registry knows.
    fn present(&self, cx: &MeasureCx<'_, Self>) -> Vec<Present> {
        let resolve = |item: &BarItem| cx.resolve_command(item.command.clone()).map(|resolved| Present { item: item.clone(), resolved });
        self.items.iter().filter_map(resolve).collect()
    }

    fn sizes(&self, cx: &MeasureCx<'_, Self>, present: &[Present]) -> Sizes {
        let axis = self.axis;
        let items = present.iter().map(|present| face_of(present).size(cx)).collect();
        let rule = if axis == Axis::Y { cx.face(Divider::horizontal()).y } else { cx.face(Divider::vertical()).x };
        let more = cx.face(IconButton::new(IconId::new("more")).label(cx.env.words::<BarWords>().items_more.clone()));
        Sizes { items, rule, more }
    }

    fn fit_items_of(&self, sizes: &Sizes, present: &[Present]) -> Vec<FitItem> {
        present
            .iter()
            .zip(&sizes.items)
            .map(|(present, size)| FitItem::new(along(self.axis, *size), present.item.section, present.item.keep))
            .collect()
    }
}

impl Component for BarItems {
    const ROLE: Role = Role::GROUP;
    type Style = BarItemsStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: BarItemsStyleKey) -> BarItemsStyle {
        let gap = cx.tokens.space.gap.inline;
        BarItemsStyle { metrics: Metrics::new().gap(gap), joint: gap, section_gap: gap * 2.0 }
    }

    /// The whole strip, or the room along the strip when that is shorter.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let present = self.present(cx);
        if present.is_empty() {
            return Vec2::ZERO;
        }
        let sizes = self.sizes(cx, &present);
        let all = vec![true; present.len()];
        let section_gap = cx.style.section_gap + sizes.rule;
        let natural = run_extent(&self.fit_items_of(&sizes, &present), &all, f64::from(cx.gap()), f64::from(section_gap)) as f32;
        let cross = sizes.items.iter().map(|size| across(self.axis, *size)).fold(across(self.axis, sizes.more), f32::max);
        match self.axis {
            Axis::Y => Vec2::new(cross, natural),
            _ => Vec2::new(cx.room.map_or(natural, |room| natural.min(room)), cross),
        }
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    /// Fits the items into the rect, places the shown items, the rules between sections and the "More" part, and
    /// declares the items and "More" as one roving group in reading order.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let axis = self.axis;
        let present = self.present(cx);
        let sizes = self.sizes(cx, &present);
        let (gap, joint, rtl, os) = (cx.gap(), cx.style.joint, cx.flow.is_rtl(), cx.env.shell().os);
        let section_gap = cx.style.section_gap + sizes.rule;
        let more_length = along(axis, sizes.more);
        let fit = fit_items(&self.fit_items_of(&sizes, &present), along(axis, rect.size()), gap, section_gap, more_length);
        let (mut at, mut previous) = (0.0, None::<u16>);
        let (mut shown, mut rules) = (Vec::new(), Vec::new());
        for &index in &fit.shown {
            let section = present[index].item.section;
            if let Some(before) = previous {
                if before == section {
                    at += gap;
                } else {
                    rules.push(slot(rect, axis, rtl, at + joint, sizes.rule));
                    at += section_gap;
                }
            }
            let length = along(axis, sizes.items[index]);
            shown.push((index, slot(rect, axis, rtl, at, length)));
            at += length;
            previous = Some(section);
        }
        let more = (!fit.overflow.is_empty()).then(|| {
            let from = if previous.is_some() { at + gap } else { at };
            slot(rect, axis, rtl, from, more_length)
        });
        for &(index, cell) in &shown {
            let part = cx.part(ITEM.at(index as u64), cell).sense(Sense::CLICK | Sense::FOCUS).focusable();
            if display_of(&present[index]) == ItemDisplay::Icon {
                let resolved = &present[index].resolved;
                part.tooltip(match &resolved.chord {
                    Some(chord) => Label::from(format!("{} ({})", resolved.label, chord.display(os))),
                    None => Label::Shared(resolved.label.clone()),
                });
            }
        }
        if let Some(more) = more {
            cx.part(MORE, more).sense(Sense::CLICK | Sense::FOCUS).focusable().toggles("menu");
        }
        cx.roving(Roving::new(NavShape::Flat));
        self.laid = Some(Laid { present, shown, rules, overflow: fit.overflow, more });
    }

    /// The strip joins the body's roving group; a click or the activation key on an item runs its command.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
        let Some(laid) = &self.laid else { return };
        for &(index, _) in &laid.shown {
            if let Some(source) = cx.part_activated(ITEM.at(index as u64)) {
                cx.fire(laid.present[index].item.command.clone(), source);
            }
        }
    }

    /// The overflow menu: one command row per overflowed item, a rule between sections.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let Some(laid) = self.laid.as_ref().filter(|laid| laid.more.is_some()) else { return };
        let popup = cx.popup("menu");
        let (present, overflow) = (&laid.present, &laid.overflow);
        let name = cx.label();
        ui.show(popup, crate::menu::layer(false), |ui, _| {
            Menu::inline(name.clone()).show(ui, |ui, _| {
                let mut previous: Option<u16> = None;
                for &index in overflow {
                    let item = &present[index].item;
                    if previous.is_some_and(|section| section != item.section) {
                        Divider::horizontal().show(ui);
                    }
                    MenuItem::command(item.command.clone()).show(ui);
                    previous = Some(item.section);
                }
            });
        });
    }

    /// A strip the caller did not label is named by the words of the family.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if cx.resolved.label.is_empty() {
            access.name(Name::Text(cx.words::<BarWords>().items_actions.as_str().to_owned()));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(laid) = &self.laid else { return };
        for &(index, _) in &laid.shown {
            face_of(&laid.present[index]).paint(ui, ITEM.at(index as u64));
        }
        for (at, rule) in laid.rules.iter().enumerate() {
            if self.axis == Axis::Y {
                ui.child(("rule", at), *rule, Divider::horizontal());
            } else {
                ui.child(("rule", at), *rule, Divider::vertical());
            }
        }
        if laid.more.is_some() {
            ui.face(MORE, IconButton::new(IconId::new("more")).label(cx.words::<BarWords>().items_more.clone()));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
