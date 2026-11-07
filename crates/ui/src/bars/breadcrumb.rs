//! `BreadcrumbBar`: a path as clickable segments, the current one last. A leaf whose crumbs are parts wearing `Link`
//! faces in one part roving group. The leading crumbs that do not fit stand behind an ellipsis part that opens a menu
//! of them.

use super::words::BreadcrumbWords;
use crate::buttons::Link;
use crate::contract::{
    Access, ActivateOn, Bind, CollectionEvent, Common, Component, Cx, Entry, Focus as _, LeadOnOpen, MeasureCx, Name, Outcome, Overlay, OverlayUi, Overlays as _, PaintUi,
    PartsCx, Pointer as _, Props as _, Role, SelectMode, ShowCollection as _, Styled as _, WordSetCx as _,
};
use crate::display::{Elide, Shade, Text};
use crate::list::{List, ListRow};
use crate::primitives::IconSpec;
use crate::runtime::env::Label;
use crate::runtime::focus::{ActivateKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, IconLook, Stateful, StyleCx, TypeRole};
use craft_behavior::{
    Axis, DisplayOrder, Flow, IconId, ItemSource, Metrics, NavShape, PartKey, PartRef, Roving, Selection, Sense, Value,
};
use egui::{Rect, Response, Vec2};

#[cfg(feature = "testing")]
mod stories;

const HIDDEN: PartKey = PartKey::new("hidden");
const CRUMB: PartKey = PartKey::new("crumb");

/// The most crumbs shown before the rest go behind the ellipsis.
const MOST_SHOWN: usize = 4;

/// What separates crumbs in the whole path a tooltip shows.
const PATH_JOINT: &str = " \u{203a} ";

/// The style of a breadcrumb bar.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BreadcrumbBarStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The chevron between crumbs.
    #[style(icon)]
    separator: IconLook,
    /// Around each separator and the ellipsis.
    gap: f32,
    /// The least room a shown crumb is given; the bar shows as many crumbs as fit at this width, at most four.
    min_crumb: f32,
    height: f32,
}

/// What a breadcrumb bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BreadcrumbOutput<K> {
    /// The bar's response.
    pub response: Response,
    /// The crumb opened this pass. The last crumb, where the user is, never reports.
    pub picked: Option<K>,
}

impl<K> Outcome for BreadcrumbOutput<K> {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// Where the parts of one pass sit.
#[derive(Default)]
struct Laid {
    /// How many leading crumbs stand behind the ellipsis.
    hidden: usize,
    ellipsis: Option<Rect>,
    /// The shown crumbs' cells, whether each is cut short, in order.
    cells: Vec<(Rect, bool)>,
    separators: Vec<Rect>,
}

/// A path of crumbs, the root first. Each crumb but the last is a link that reports its value in the output.
#[derive(crate::contract::Component)]
#[component(overlay, probe(BreadcrumbBar<'static, u8>))]
pub struct BreadcrumbBar<'a, K: Value + 'static> {
    #[common]
    common: Common,
    crumbs: &'a [Entry<K>],
    picked: Option<K>,
    laid: Laid,
    whole: Label,
}

impl<'a, K: Value + 'static> BreadcrumbBar<'a, K> {
    /// A bar named `label` over `crumbs`, the root first.
    pub fn new(label: impl Into<Label>, crumbs: &'a [Entry<K>]) -> Self {
        debug_assert!(crumbs.iter().all(|crumb| !crumb.is_separator()), "a `BreadcrumbBar` has no separators");
        BreadcrumbBar { common: Common::new(), crumbs, picked: None, laid: Laid::default(), whole: Label::Empty }.label(label)
    }

    fn part(at: usize) -> PartRef {
        CRUMB.at(at as u64)
    }

    fn is_last(&self, at: usize) -> bool {
        at + 1 == self.crumbs.len()
    }

    /// The face crumb `at` wears: a link for an ancestor, strong text for the current crumb.
    fn crumb_face(&self, at: usize, label: &Label) -> CrumbFace {
        if self.is_last(at) {
            CrumbFace::Current(Text::new(label.clone()).role(TypeRole::BodyStrong).elide(Elide::End))
        } else {
            CrumbFace::Ancestor(Link::new(label.clone()))
        }
    }

    fn natural(&self, at: usize, cx: &MeasureCx<'_, Self>) -> f32 {
        match self.crumb_face(at, self.crumbs[at].label()) {
            CrumbFace::Current(text) => cx.face(text).x,
            CrumbFace::Ancestor(link) => cx.face(link).x,
        }
    }
}

enum CrumbFace {
    Ancestor(Link),
    Current(Text),
}

/// Widths for crumbs that want `natural` points each in `room` points: the shortest keep what they want and the rest
/// share what is left equally.
fn fair_widths(natural: &[f32], room: f32) -> Vec<f32> {
    let mut order: Vec<usize> = (0..natural.len()).collect();
    order.sort_by(|&a, &b| natural[a].total_cmp(&natural[b]));
    let mut widths = vec![0.0; natural.len()];
    let mut left = room;
    for (taken, &at) in order.iter().enumerate() {
        let share = left / (natural.len() - taken) as f32;
        widths[at] = natural[at].min(share).floor();
        left -= widths[at];
    }
    widths
}

/// The segments of a file path as crumbs, each valued by the path that opens it, spelled as in `path`: its own
/// separators (`/` or `\`), a leading `/` or a UNC `\\server\share` kept in the values, empty parts skipped. A bare
/// drive (`C:`) as the first crumb gets its separator back in its value, since on its own it means the current folder
/// on that drive.
pub fn path_crumbs(path: &str) -> Vec<Entry<String>> {
    let mut out: Vec<Entry<String>> = Vec::new();
    let mut start = 0;
    for (at, c) in path.char_indices().chain(std::iter::once((path.len(), '/'))) {
        if c != '/' && c != '\\' {
            continue;
        }
        if let Some(part) = path.get(start..at).filter(|part| !part.is_empty()) {
            let mut key = path.get(..at).unwrap_or(path).to_owned();
            if out.is_empty() && part.ends_with(':') && key == part {
                key.push(if path.contains('\\') { '\\' } else { '/' });
            }
            out.push(Entry::new(key, part.to_owned()));
        }
        start = at + c.len_utf8();
    }
    out
}

impl<K: Value + 'static> Component for BreadcrumbBar<'_, K> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::of(crate::contract::Input::Pointer).with_activate(ActivateKeys::Both);
    type Style = BreadcrumbBarStyle;
    type Output = BreadcrumbOutput<K>;

    fn style(cx: &StyleCx<'_>, _: BreadcrumbBarStyleKey<K>) -> BreadcrumbBarStyle {
        let control = cx.control();
        let muted = cx.color(cx.tokens.color.text.muted);
        BreadcrumbBarStyle {
            metrics: Metrics::new().min_size(Vec2::new(0.0, control.height)),
            focus: cx.focus_look(),
            separator: IconLook::new(control.icon, Stateful::new(muted)),
            gap: cx.tokens.space.gap.tight,
            min_crumb: cx.tokens.size.length.short,
            height: control.height,
        }
    }

    /// The width of the whole path with every crumb and separator; the bar fills the room it is given.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let crumbs: f32 = (0..self.crumbs.len()).map(|at| self.natural(at, cx)).sum();
        let joints = self.crumbs.len().saturating_sub(1) as f32 * (cx.style.separator.side + 2.0 * cx.style.gap);
        Vec2::new(crumbs + joints, cx.style.height)
    }

    fn fills(&self) -> Option<Axis> {
        Some(Axis::Horizontal)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let (gap, joint) = (cx.style.gap, cx.style.separator.side);
        let total = self.crumbs.len();
        let shown = ((rect.width() / cx.style.min_crumb) as usize).clamp(1, MOST_SHOWN).min(total);
        let hidden = total - shown;
        let labels: Vec<String> = self.crumbs.iter().map(|crumb| cx.env.resolve(crumb.label()).as_str().to_owned()).collect();
        self.whole = Label::from(labels.join(PATH_JOINT));
        let mut laid = Laid { hidden, ..Laid::default() };
        let mut flow = Flow::new(rect, cx.flow.is_rtl());
        if hidden > 0 {
            let width = cx.face(Text::new("\u{2026}")).x + 2.0 * gap;
            let (cell, rest) = flow.take_start(width);
            laid.ellipsis = Some(cell);
            let (_, rest) = rest.take_start(gap);
            let (chevron, rest) = rest.take_start(joint);
            laid.separators.push(chevron);
            flow = rest.take_start(gap).1;
        }
        let natural: Vec<f32> = (hidden..total).map(|at| self.natural(at, cx)).collect();
        let joints = (shown - 1) as f32 * (joint + 2.0 * gap);
        let room = (flow.rect().width() - joints).max(0.0);
        let widths = fair_widths(&natural, room);
        for (at, (width, wanted)) in (hidden..total).zip(widths.into_iter().zip(&natural)) {
            let (cell, rest) = flow.take_start(width);
            laid.cells.push((cell, width < *wanted));
            flow = rest;
            if !self.is_last(at) {
                let (_, rest) = flow.take_start(gap);
                let (chevron, rest) = rest.take_start(joint);
                laid.separators.push(chevron);
                flow = rest.take_start(gap).1;
            }
        }
        if let Some(cell) = laid.ellipsis {
            cx.part(HIDDEN, cell).sense(Sense::CLICK | Sense::FOCUS).focusable().tooltip(self.whole.clone()).toggles("hidden");
        }
        for (at, (cell, cut)) in (hidden..total).zip(laid.cells.iter().copied()) {
            let part = cx.part(Self::part(at), cell);
            if self.is_last(at) {
                let part = part.sense(Sense::HOVER);
                if cut {
                    part.tooltip(self.crumbs[at].label().clone());
                }
            } else {
                let tip = if cut { self.crumbs[at].label().clone() } else { self.whole.clone() };
                part.sense(Sense::CLICK | Sense::FOCUS).focusable().tooltip(tip);
            }
        }
        self.laid = laid;
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        for at in self.laid.hidden..self.crumbs.len().saturating_sub(1) {
            let part = Self::part(at);
            if cx.part(part).press.phase.is_click() || by_key == Some(part) {
                self.picked = self.crumbs[at].value().cloned();
            }
        }
    }

    /// The menu of the hidden crumbs, the nearest last. A pick comes back from the list as Enter or a click on a row.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("hidden");
        let hidden = &self.crumbs[..self.laid.hidden];
        let labels: Vec<Label> = hidden.iter().map(|crumb| cx.env.resolve(crumb.label())).collect();
        let name = cx.words::<BreadcrumbWords>().hidden.clone();
        let events = ui.show(popup, Overlay::menu(), |ui, _| {
            let rows = Hidden { labels: &labels };
            List::<usize>::new(name.clone())
                .select(SelectMode::Single)
                .activate(ActivateOn::Click)
                .lead_on_open(LeadOnOpen::First)
                .lead_follows_hover(true)
                .listbox()
                .selection(Bind::given(Selection::new(Vec::new(), None, None)))
                .show_items(ui, &rows, &mut |at, _| ListRow::new(labels[*at].clone()).option())
                .events
        });
        let mut chosen = None;
        for event in events.into_iter().flatten() {
            if let CollectionEvent::Activate(at) = event {
                chosen = Some(at);
            }
        }
        if let Some(value) = chosen.and_then(|at| hidden.get(at)).and_then(Entry::value) {
            self.picked = Some(value.clone());
            popup.close(cx);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        for (at, crumb) in self.crumbs.iter().enumerate().skip(self.laid.hidden) {
            let role = if self.is_last(at) { Role::LABEL } else { Role::LINK };
            access.node(Self::part(at), role).name = Name::Text(cx.env.resolve(crumb.label()).as_str().to_owned());
        }
        if self.laid.ellipsis.is_some() {
            access.node(HIDDEN, Role::BUTTON).name = Name::Text(cx.words::<BreadcrumbWords>().hidden.as_str().to_owned());
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        if self.laid.ellipsis.is_some() {
            ui.face(HIDDEN, Text::new("\u{2026}").shade(Shade::Muted));
        }
        for (at, _) in (self.laid.hidden..self.crumbs.len()).zip(&self.laid.cells) {
            match self.crumb_face(at, self.crumbs[at].label()) {
                CrumbFace::Current(text) => ui.face(Self::part(at), text),
                CrumbFace::Ancestor(link) => ui.face(Self::part(at), link),
            }
        }
        let side = cx.style.separator.side;
        let color = cx.resolve(&cx.style.separator.color);
        for slot in &self.laid.separators {
            let glyph = Rect::from_center_size(slot.center(), Vec2::splat(side));
            ui.draw().icon(glyph, &IconSpec::svg(IconId::new("chevron_right")).color(color));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> BreadcrumbOutput<K> {
        BreadcrumbOutput { response: cx.response.clone(), picked: self.picked }
    }
}

/// The hidden crumbs as a collection source, keyed by position.
struct Hidden<'a> {
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Hidden<'_> {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.labels.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Hidden<'_> {}
