//! Pure layout and rules of the tab strip.
//!
//! Positions are physical points. "Reading order" is the display order of the strip: left to right,
//! or right to left in a mirrored interface.

use egui::{Key, Modifiers, Pos2, Rect, os::OperatingSystem, pos2};

use crate::behavior::StackKey;
use crate::tabs::{TabFit, fit, fit_whole};

/// The kind of a strip. It sets the defaults below; the host picks one per strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabKind {
    /// Document tabs: whole titles, a × on every closable tab, every document in the overflow menu.
    Document,
    /// Panel tabs: tabs shrink before they overflow, no ×, only the hidden tabs in the overflow menu.
    Panel,
    /// Text tabs: only the labels, the selected one in the text colour and the others dimmed, no
    /// tab surface, divider or ×. Whole titles, every tab in the overflow menu (a row of
    /// categories in a dialog).
    Text,
}

/// How tab widths are fitted to the strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitMode {
    /// [`fit_whole`]: whole widths; only an active tab wider than the strip is cut.
    Whole,
    /// [`fit`]: the widest tabs shrink to the minimum width, then the rest overflow.
    Shrink,
}

/// Where the overflow button sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPlace {
    /// Pinned to the reading end of the tab area.
    StripEnd,
    /// Right after the last shown tab.
    AfterTabs,
}

/// Which tabs the overflow menu lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowMenu {
    /// Every tab, the active one checked.
    AllTabs,
    /// The hidden tabs only.
    HiddenOnly,
}

impl TabKind {
    pub fn fit_mode(self) -> FitMode {
        match self {
            Self::Document | Self::Text => FitMode::Whole,
            Self::Panel => FitMode::Shrink,
        }
    }

    pub fn overflow_place(self) -> OverflowPlace {
        match self {
            Self::Document | Self::Text => OverflowPlace::StripEnd,
            Self::Panel => OverflowPlace::AfterTabs,
        }
    }

    pub fn overflow_menu(self) -> OverflowMenu {
        match self {
            Self::Document | Self::Text => OverflowMenu::AllTabs,
            Self::Panel => OverflowMenu::HiddenOnly,
        }
    }

    /// Whether closable tabs show a × on the tab; a panel's Close is in the host's context menu.
    pub fn shows_close(self) -> bool {
        self == Self::Document
    }

    /// Whether tabs draw only their label, with no surface, indicator or strip fill.
    pub fn text_only(self) -> bool {
        self == Self::Text
    }

    /// The default of `middle_click_closes`.
    pub fn middle_click_closes(self) -> bool {
        self == Self::Document
    }

    /// Whether the compact size draws a divider at each tab's trailing edge.
    pub fn compact_dividers(self) -> bool {
        self == Self::Document
    }
}

/// The side of a tab its × is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseSide {
    Left,
    Right,
}

/// The side of the tab the × is on: before the title on macOS, after it elsewhere, in reading
/// order, so a mirrored strip puts it on the other physical side.
pub fn close_side(os: OperatingSystem, rtl: bool) -> CloseSide {
    let before_title = os == OperatingSystem::Mac;
    if before_title != rtl { CloseSide::Left } else { CloseSide::Right }
}

/// What [`StripPlan::new`] lays out.
#[derive(Clone, Debug)]
pub struct StripPlanInput<'a> {
    pub kind: TabKind,
    /// The strip's rect.
    pub rect: Rect,
    pub rtl: bool,
    /// Width of the host's area at the reading start.
    pub leading: f32,
    /// Width of the host's area at the reading end.
    pub trailing: f32,
    /// Each tab's natural width, in the host's order.
    pub natural: &'a [f32],
    /// The index of the shown tab; out of range or `None` keeps the last tab.
    pub selected: Option<usize>,
    /// The narrowest a panel tab shrinks to.
    pub min_width: f32,
    /// The overflow button's width.
    pub overflow_width: f32,
}

/// A shown tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedTab {
    /// The tab's index in the host's order.
    pub index: usize,
    pub rect: Rect,
}

/// Where everything of a strip goes on one pass.
#[derive(Clone, Debug, PartialEq)]
pub struct StripPlan {
    /// The shown tabs in reading order.
    pub shown: Vec<PlacedTab>,
    /// The hidden tabs' indices, in the host's order.
    pub hidden: Vec<usize>,
    /// The overflow button; `None` when every tab is shown.
    pub overflow: Option<Rect>,
    /// The host's areas; zero width when the host has none.
    pub leading: Rect,
    pub trailing: Rect,
    /// The space between the host's areas, where the tabs and the button go.
    pub tabs_area: Rect,
}

impl StripPlan {
    pub fn new(input: &StripPlanInput<'_>) -> Self {
        let StripPlanInput { kind, rect, rtl, leading, trailing, natural, selected, min_width, overflow_width } = *input;
        let total = rect.width().max(0.0);
        let leading = leading.clamp(0.0, total);
        let trailing = trailing.clamp(0.0, total - leading);
        // Distances from the reading start, mapped to physical x.
        let at = |from_start: f32| if rtl { rect.right() - from_start } else { rect.left() + from_start };
        let span = |from: f32, to: f32| {
            let (a, b) = (at(from), at(to));
            Rect::from_min_max(pos2(a.min(b), rect.top()), pos2(a.max(b), rect.bottom()))
        };
        let area = total - leading - trailing;
        let selected = selected.unwrap_or(usize::MAX);
        let TabFit { shown, overflow, overflow_width } = match kind.fit_mode() {
            FitMode::Whole => fit_whole(natural, selected, area, overflow_width),
            FitMode::Shrink => fit(natural, selected, area, min_width, overflow_width),
        };
        let mut x = leading;
        let shown: Vec<PlacedTab> = shown
            .into_iter()
            .map(|(index, width)| {
                let tab = PlacedTab { index, rect: span(x, x + width) };
                x += width;
                tab
            })
            .collect();
        let button = (!overflow.is_empty()).then(|| match kind.overflow_place() {
            OverflowPlace::AfterTabs => span(x, x + overflow_width),
            OverflowPlace::StripEnd => span(total - trailing - overflow_width, total - trailing),
        });
        Self {
            shown,
            hidden: overflow,
            overflow: button,
            leading: span(0.0, leading),
            trailing: span(total - trailing, total),
            tabs_area: span(leading, total - trailing),
        }
    }

    /// The host-order index a drop at `pos` opens at: before the first shown tab whose centre is
    /// past `pos` in reading order, else after the last shown tab. Zero when nothing is shown.
    pub fn slot_at(&self, rtl: bool, pos: Pos2) -> usize {
        let past = |tab: &PlacedTab| if rtl { tab.rect.center().x < pos.x } else { tab.rect.center().x > pos.x };
        match self.shown.iter().find(|tab| past(tab)) {
            Some(tab) => tab.index,
            None => self.shown.last().map_or(0, |tab| tab.index + 1),
        }
    }

    /// The index of the shown tab under `pos`.
    pub fn tab_at(&self, pos: Pos2) -> Option<usize> {
        self.shown.iter().find(|tab| tab.rect.contains(pos)).map(|tab| tab.index)
    }

    /// The rect of the shown tab at host index `index`.
    pub fn rect_of(&self, index: usize) -> Option<Rect> {
        self.shown.iter().find(|tab| tab.index == index).map(|tab| tab.rect)
    }
}

/// Where a tab drag stands relative to its strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragZone {
    /// Targets resolve.
    Band,
    /// Past the band: no target, the tab is leaving the strip.
    Outside,
}

/// The zone of a drag at `pointer`. The band is the strip rect grown by `margin` on every side.
/// Without `tear_off` only the position along the strip counts, so the pointer is always in the band.
pub fn drag_zone(strip: Rect, pointer: Pos2, tear_off: bool, margin: f32) -> DragZone {
    if !tear_off || strip.expand(margin).contains(pointer) { DragZone::Band } else { DragZone::Outside }
}

/// The stacking key the reorder keys read as on a strip: `]` moves a tab one place later in
/// reading order and `[` one earlier, Shift to the ends. [`StackKey`] reads `]` as toward the
/// start, as a list's top; the strip's display order is the reading order, so the same swap
/// serves both directions of the interface.
pub fn strip_key(key: StackKey) -> StackKey {
    match key {
        StackKey::TowardStart => StackKey::TowardEnd,
        StackKey::TowardEnd => StackKey::TowardStart,
        StackKey::ToStart => StackKey::ToEnd,
        StackKey::ToEnd => StackKey::ToStart,
    }
}

/// The move the pressed `key` asks for on a strip, ready for `Reorder::key_move`.
pub fn reorder_key(key: Key, modifiers: Modifiers, mac: bool) -> Option<StackKey> {
    StackKey::from_key(key, modifiers, mac).map(strip_key)
}

/// A keyboard focus position of a strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusSpot {
    /// The shown tab at this position in reading order.
    Tab(usize),
    /// The overflow button.
    Overflow,
}

/// A focus move on the strip, in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusMove {
    Previous,
    Next,
    First,
    Last,
}

/// The focus move a key asks for: the arrows by reading direction, Home and End. Other keys, and
/// arrows with a modifier, are not the strip's.
pub fn focus_move(key: Key, modifiers: Modifiers, rtl: bool) -> Option<FocusMove> {
    if modifiers.any() {
        return None;
    }
    let (previous, next) = if rtl { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowLeft, Key::ArrowRight) };
    match key {
        k if k == previous => Some(FocusMove::Previous),
        k if k == next => Some(FocusMove::Next),
        Key::Home => Some(FocusMove::First),
        Key::End => Some(FocusMove::Last),
        _ => None,
    }
}

/// Where the focus goes after `mv`: along the shown tabs and then onto the overflow button, with
/// no wrap. Home and End stay on the tabs. `None` when the strip has nothing to focus or the move
/// stays where it is.
pub fn step_focus(from: FocusSpot, mv: FocusMove, shown: usize, has_overflow: bool) -> Option<FocusSpot> {
    let last = shown.checked_sub(1);
    let to = match (mv, from) {
        (FocusMove::First, _) => last.map(|_| FocusSpot::Tab(0)),
        (FocusMove::Last, _) => last.map(FocusSpot::Tab),
        (FocusMove::Previous, FocusSpot::Tab(at)) => at.checked_sub(1).map(FocusSpot::Tab),
        (FocusMove::Previous, FocusSpot::Overflow) => last.map(FocusSpot::Tab),
        (FocusMove::Next, FocusSpot::Tab(at)) if last.is_some_and(|last| at < last) => Some(FocusSpot::Tab(at + 1)),
        (FocusMove::Next, FocusSpot::Tab(_)) => has_overflow.then_some(FocusSpot::Overflow),
        (FocusMove::Next, FocusSpot::Overflow) => None,
    };
    to.filter(|&to| to != from)
}

/// The focus position of the tab at host index `index` on `plan`: its place among the shown tabs,
/// or the overflow button when it is hidden (a tab moved into the overflow). `None` when neither
/// exists.
pub fn focus_spot(plan: &StripPlan, index: usize) -> Option<FocusSpot> {
    match plan.shown.iter().position(|tab| tab.index == index) {
        Some(at) => Some(FocusSpot::Tab(at)),
        None => plan.overflow.map(|_| FocusSpot::Overflow),
    }
}
