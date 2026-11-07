//! `MenuItem` (11.4, 12.3): one row of a menu, and the level it shares with the rows around it.

use crate::contract::{
    Access, Common, Component, Cx, Focus as _, MeasureCx, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props, Remember as _, Role, Styled as _,
};
use crate::display::Kbd;
use crate::primitives::{IconSpec, PaintShape, StrokeWidth, TextRun};
use crate::runtime::access::PopupKind;
use crate::runtime::commands::{self, CommandRef, FireSource, ResolvedCommand};
use crate::runtime::env::{AppValues, EnvValue, Label};
use crate::runtime::frame;
use crate::style::{ComponentStyle, Flags, FocusLook, IconLook, StyleCx, SurfaceLook, TextLook, TypeRole, UiTokens};
use craft_behavior::{Flow, IconId, Metrics, NavMove, PartKey, Sense};
use craft_keymap::mnemonic;
use craft_theme::{Rgba, Stateful, States};
use egui::{Id, Pos2, Rect, Response, Vec2, pos2};
use std::borrow::Cow;
use std::sync::{Arc, Mutex, PoisonError};

/// The part a row's shortcut text shows in, as the face of a `Kbd`.
const SHORTCUT: PartKey = PartKey::new("shortcut");

/// The seconds the pointer rests on a submenu row before the row asks for its submenu.
const SUBMENU_DELAY: f64 = 0.3;

/// The points of a tick in a unit square, as a path.
const TICK: [(f32, f32); 3] = [(0.2, 0.5), (0.42, 0.72), (0.8, 0.28)];

/// The radius of a radio mark, as a share of the icon side.
const DOT_RADIUS: f32 = 0.2;

/// The label of a row as the level and the paint read it: the words row's `&` marker names the letter.
fn marker_at(label: &str) -> Option<usize> {
    let mut chars = label.chars().peekable();
    let mut at = 0;
    while let Some(c) = chars.next() {
        if c != '&' {
            at += 1;
            continue;
        }
        match chars.peek() {
            Some('&') => {
                chars.next();
                at += 1;
            }
            Some(next) if next.is_ascii_alphanumeric() => return Some(at),
            _ => {}
        }
    }
    None
}

/// `label` with a `&` marker before the first character that is `letter`, so the level's assignment honours a row's
/// own mnemonic; a label without that letter is left as it is.
fn with_marker(label: &str, letter: char) -> String {
    match label.char_indices().find(|(_, c)| c.eq_ignore_ascii_case(&letter)) {
        Some((at, _)) => format!("{}&{}", &label[..at], &label[at..]),
        None => label.to_owned(),
    }
}

/// What a row shows as text and the character index its mnemonic underlines: a label with no `letter` of its own
/// shows the letter after it in parentheses.
fn shown(label: &str, letter: Option<char>) -> (String, Option<usize>) {
    let (text, _) = mnemonic::marked(label);
    let Some(letter) = letter else { return (text, None) };
    let at = marker_at(label).or_else(|| text.chars().position(|c| c.eq_ignore_ascii_case(&letter)));
    match at {
        Some(at) => (text, Some(at)),
        None => {
            let at = text.chars().count() + 2;
            (format!("{text} ({letter})"), Some(at))
        }
    }
}

/// A row as it registered with its level in one pass.
struct Row {
    id: Id,
    text: String,
    enabled: bool,
    mark: bool,
    icon: bool,
}

/// A row's mnemonic as the level assigned it from the last pass's registrations.
struct Assigned {
    id: Id,
    letter: Option<char>,
    enabled: bool,
}

#[derive(Default)]
struct LevelState {
    now: Vec<Row>,
    shown: Vec<Assigned>,
    marks: bool,
    icons: bool,
    keyboard: bool,
    keyboard_now: bool,
    pending: Option<Id>,
    close: bool,
    submenu: bool,
}

/// The rows of one menu level: each registers its label, and the level gives every row its mnemonic from the
/// registrations of the pass before. Rows outside any menu get an inert level.
#[derive(Clone)]
pub(crate) struct MenuLevel {
    state: Option<Arc<Mutex<LevelState>>>,
}

impl PartialEq for MenuLevel {
    fn eq(&self, other: &Self) -> bool {
        match (&self.state, &other.state) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl EnvValue for MenuLevel {
    fn root() -> Self {
        MenuLevel { state: None }
    }
}

/// The level's registrations, kept in the menu's shown memory so the rows of the last pass are readable now.
#[derive(Clone, Default)]
pub(crate) struct LevelMemory {
    live: Arc<Mutex<LevelState>>,
}

impl MenuLevel {
    pub(crate) fn of(memory: &LevelMemory) -> Self {
        MenuLevel { state: Some(Arc::clone(&memory.live)) }
    }

    fn with<R: Default>(&self, f: impl FnOnce(&mut LevelState) -> R) -> R {
        match &self.state {
            Some(state) => f(&mut state.lock().unwrap_or_else(PoisonError::into_inner)),
            None => R::default(),
        }
    }

    /// Starts a pass over the level: the rows that registered last pass get their letters, and the rows of this pass
    /// register again.
    fn begin(&self, submenu: bool) {
        self.with(|state| {
            let letters = {
                let labels: Vec<&str> = state.now.iter().map(|row| row.text.as_str()).collect();
                mnemonic::assign(&labels)
            };
            state.shown = state.now.iter().zip(letters).map(|(row, letter)| Assigned { id: row.id, letter, enabled: row.enabled }).collect();
            state.marks = state.now.iter().any(|row| row.mark);
            state.icons = state.now.iter().any(|row| row.icon);
            state.keyboard = state.keyboard_now;
            state.keyboard_now = false;
            state.now.clear();
            state.submenu = submenu;
        });
    }

    fn register(&self, row: Row) {
        self.with(|state| {
            match state.now.iter_mut().find(|known| known.id == row.id) {
                Some(known) => *known = row,
                None => state.now.push(row),
            };
        });
    }

    fn letter_of(&self, id: Id) -> Option<char> {
        self.with(|state| state.shown.iter().find(|row| row.id == id).and_then(|row| row.letter))
    }

    /// The enabled row `typed` runs.
    pub(crate) fn target(&self, typed: char) -> Option<Id> {
        let typed = typed.to_ascii_uppercase();
        self.with(|state| state.shown.iter().find(|row| row.enabled && row.letter == Some(typed)).map(|row| row.id))
    }

    fn marks(&self) -> bool {
        self.with(|state| state.marks)
    }

    fn icons(&self) -> bool {
        self.with(|state| state.icons)
    }

    /// Whether a row of the level held keyboard focus in the last pass: the mnemonics show.
    fn keyboard(&self) -> bool {
        self.with(|state| state.keyboard)
    }

    fn note_keyboard(&self) {
        self.with(|state| state.keyboard_now = true);
    }

    pub(crate) fn set_pending(&self, id: Id) {
        self.with(|state| state.pending = Some(id));
    }

    fn take_pending(&self, id: Id) -> bool {
        self.with(|state| {
            let mine = state.pending == Some(id);
            if mine {
                state.pending = None;
            }
            mine
        })
    }

    fn is_submenu(&self) -> bool {
        self.with(|state| state.submenu)
    }

    fn request_close(&self) {
        self.with(|state| state.close = true);
    }

    /// Whether a row asked its submenu level to close.
    pub(crate) fn take_close(&self) -> bool {
        self.with(|state| std::mem::take(&mut state.close))
    }
}

/// The level a menu container puts into its body's env: the rows of the body register with it.
pub(crate) fn level_env<C: Component>(cx: &Cx<'_, C>, values: &mut AppValues, submenu: bool) {
    let level = MenuLevel::of(&cx.recall::<LevelMemory>());
    level.begin(submenu);
    values.set(level);
}

/// The style of a menu row: the hover fill, the label, the row's icon and its detail.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct MenuItemStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The label, the shortcut's ink and the marks.
    #[style(text)]
    text: TextLook,
    /// The row's icon, the check and radio marks and the submenu arrow.
    #[style(icon)]
    icon: IconLook,
    /// The detail after the label.
    #[style(text)]
    detail: TextLook,
}

/// What a row reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct MenuItemOutput {
    /// The row's response.
    pub response: Response,
    /// The row ran this pass: a click, Enter or Space on it, or its mnemonic letter. A submenu row never runs; it
    /// asks for its submenu.
    pub ran: bool,
    /// A submenu row asks for its submenu to open: the pointer rested on it, Forward (Right, mirrored under
    /// right-to-left) was pressed on it, or it was activated.
    pub opens_submenu: bool,
}

impl Outcome for MenuItemOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        crate::testing::StoryOutput { id: Some(self.response.id), clicked: self.ran, ..crate::testing::StoryOutput::default() }
    }
}

/// The widths of a row's columns and its trailing items, in points; zero for a column the row does not show.
struct Columns {
    mark: f32,
    icon: f32,
    detail: f32,
    trail: f32,
}

/// Where the parts of a row lie.
struct Placed {
    mark: Rect,
    icon: Rect,
    label: Rect,
    detail: Rect,
    trail: Rect,
}

/// A row of a menu: a label, an optional icon, a check or radio mark, shortcut text and a detail. A row bound to a
/// command takes its label, shortcut, enabled and checked state from the command and fires it as a menu row.
///
/// - Running a row (click, Enter or Space, its mnemonic letter) closes the whole chain of menus it sits in.
/// - A submenu row opens a submenu instead: the output's `opens_submenu` asks the caller to open the `Menu` it shows
///   after the row.
/// - The mnemonic is the `&` letter of the label, else the first letter no earlier row of the level uses.
///   Underlines show while the keyboard drives the menu, never on macOS.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct MenuItem<'a> {
    #[common]
    common: Common,
    command_ref: Option<CommandRef>,
    submenu: bool,
    /// An icon before the label.
    #[prop(into)]
    icon: Option<IconSpec<'a>>,
    /// A check mark, shown while the value is true; the row reserves the mark column either way.
    #[prop]
    check: Option<bool>,
    /// A radio mark, shown while the value is true; the row reserves the mark column either way.
    #[prop]
    radio_mark: Option<bool>,
    /// The letter that runs the row, in place of the level's own pick.
    #[prop]
    mnemonic: Option<char>,
    /// A muted text after the label.
    #[prop(into)]
    detail: Option<Label>,
    /// Shortcut text at the end of the row, written by the caller; a command row shows its chord.
    #[prop(into)]
    shortcut: Option<Label>,
    ran: bool,
    opens: bool,
}

impl<'a> MenuItem<'a> {
    /// A row showing `label`, which is also its accessible name.
    pub fn new(label: impl Into<Label>) -> Self {
        MenuItem {
            common: Common::new(),
            command_ref: None,
            submenu: false,
            icon: None,
            check: None,
            radio_mark: None,
            mnemonic: None,
            detail: None,
            shortcut: None,
            ran: false,
            opens: false,
        }
        .label(label)
    }

    /// A row bound to `command`: its label, chord, enabled state and checked state follow the command, and running it
    /// fires the command.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        let command = command.into();
        let mut item = MenuItem::new(Label::Empty);
        item.command_ref = Some(command.clone());
        <MenuItem as Props>::command(item, command)
    }

    /// A row that opens a submenu, with an arrow at its end.
    pub fn submenu(label: impl Into<Label>) -> Self {
        MenuItem { submenu: true, ..MenuItem::new(label) }
    }

    /// The icon a bound command names, when it names one with a static name.
    fn command_icon(command: Option<&ResolvedCommand>) -> Option<IconSpec<'static>> {
        match command?.icon.as_ref()? {
            Cow::Borrowed(name) => Some(IconSpec::svg(IconId::new(name))),
            Cow::Owned(_) => None,
        }
    }

    fn lead_icon(&self, command: Option<&ResolvedCommand>) -> Option<IconSpec<'_>> {
        self.icon.clone().or_else(|| MenuItem::command_icon(command))
    }

    fn marked_row(&self, command: Option<&ResolvedCommand>) -> bool {
        self.check.is_some() || self.radio_mark.is_some() || command.is_some_and(|command| command.checked.is_some())
    }

    /// The shortcut as the `Kbd` that shows it.
    fn keys(&self, command: Option<&ResolvedCommand>) -> Option<Kbd> {
        match (&self.command_ref, command) {
            (Some(reference), Some(resolved)) if resolved.chord.is_some() => Some(Kbd::command(reference.clone())),
            _ => self.shortcut.clone().map(Kbd::new),
        }
    }

    fn place(&self, columns: &Columns, area: Rect, rtl: bool, gap: f32) -> Placed {
        let step = |width: f32| if width > 0.0 { width + gap } else { 0.0 };
        let (mark, flow) = Flow::new(area, rtl).take_start(step(columns.mark));
        let (icon, flow) = flow.take_start(step(columns.icon));
        let (trail, flow) = flow.take_end(step(columns.trail));
        let (detail, flow) = flow.take_end(step(columns.detail));
        Placed { mark, icon, label: flow.rect(), detail, trail }
    }

    fn columns(&self, cx: &MeasureCx<'_, Self>, level: &MenuLevel) -> Columns {
        let side = cx.icon();
        let command = cx.command();
        let marked = self.marked_row(command) || level.marks();
        let iconed = self.lead_icon(command).is_some() || level.icons();
        let detail = self.detail.as_ref().map_or(0.0, |detail| {
            let detail = cx.env.resolve(detail);
            cx.layout_at(&TextRun::new(detail.as_str()).role(cx.style.detail.role), f32::INFINITY).size().x
        });
        let trail = if self.submenu { side } else { self.keys(command).map_or(0.0, |keys| cx.face(keys).x) };
        Columns { mark: if marked { side } else { 0.0 }, icon: if iconed { side } else { 0.0 }, detail, trail }
    }
}

impl Component for MenuItem<'_> {
    const ROLE: Role = Role::MENU_ITEM;
    const ALT_ROLES: &'static [Role] = &[Role::MENU_ITEM_CHECK_BOX, Role::MENU_ITEM_RADIO];
    const OVERLAYS: bool = false;
    type Style = MenuItemStyle;
    type Output = MenuItemOutput;

    fn style(cx: &StyleCx<'_>, _: MenuItemStyleKey) -> MenuItemStyle {
        let tokens = cx.tokens;
        let (state, text) = (&tokens.color.state, &tokens.color.text);
        let c = |rgba: Rgba| cx.color(rgba);
        let clear = Rgba::TRANSPARENT;
        let fill = Stateful::new(c(clear)).off(States::new(c(clear), c(state.hover), c(state.pressed), c(clear)));
        let surface = SurfaceLook::new(fill).radius(UiTokens::corner(tokens.shape.radius.sm));
        let ink = |base: Rgba| Stateful::new(c(base)).off(States::new(c(base), c(base), c(base), c(text.disabled)));
        let control = cx.control();
        MenuItemStyle {
            surface,
            metrics: control.metrics(),
            focus: cx.focus_look(),
            text: TextLook::new(TypeRole::Body, ink(text.normal)),
            icon: IconLook::new(control.icon, ink(text.normal)),
            detail: TextLook::new(TypeRole::Caption, ink(text.faint)),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let level = cx.env.get::<MenuLevel>();
        let columns = self.columns(cx, &level);
        let (text, _) = shown(cx.resolved.label, level.letter_of(cx.id));
        let run = TextRun::new(text.as_str()).role(cx.style.text.role);
        let label = if text.is_empty() { Vec2::ZERO } else { cx.layout_at(&run, f32::INFINITY).size() };
        let gap = cx.gap();
        let step = |width: f32| if width > 0.0 { width + gap } else { 0.0 };
        let width = step(columns.mark) + step(columns.icon) + label.x + step(columns.detail) + step(columns.trail);
        Vec2::new(width, label.y.max(cx.icon()))
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    /// The shortcut's slot, which paints as the face of a `Kbd`.
    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let level = cx.env.get::<MenuLevel>();
        let columns = self.columns(cx, &level);
        if columns.trail == 0.0 || self.submenu {
            return;
        }
        let rtl = cx.flow.is_rtl();
        let placed = self.place(&columns, cx.content_rect, rtl, cx.gap());
        let (rect, _) = Flow::new(placed.trail, rtl).take_end(columns.trail);
        cx.part(SHORTCUT, rect);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
        let ctx = cx.core.ctx.clone();
        let level = cx.env.get::<MenuLevel>();
        let enabled = cx.resolved.enabled;
        let command = cx.command.clone();
        let label = match self.mnemonic {
            Some(letter) => with_marker(cx.resolved.label, letter),
            None => cx.resolved.label.to_owned(),
        };
        level.register(Row {
            id: cx.id,
            text: label,
            enabled,
            mark: self.marked_row(command.as_deref()),
            icon: self.lead_icon(command.as_deref()).is_some(),
        });
        if cx.focus_visible() {
            level.note_keyboard();
        }
        let mut ran = enabled && cx.activation().is_some();
        let (mut forward, mut back) = (false, false);
        while let Some((step, _)) = cx.intents().take_move() {
            match step {
                NavMove::Forward => forward = true,
                NavMove::Back => back = true,
                _ => {}
            }
        }
        if enabled && level.take_pending(cx.id) {
            ran = true;
        }
        if back && level.is_submenu() {
            level.request_close();
        }
        if self.submenu {
            let hovered = enabled && cx.response.hovered();
            let now = frame::clock(&ctx);
            let waited = {
                let dwell = cx.memory::<Dwell>();
                if hovered {
                    now - *dwell.since.get_or_insert(now)
                } else {
                    dwell.since = None;
                    0.0
                }
            };
            if hovered && waited < SUBMENU_DELAY {
                frame::request_repaint(&ctx);
            }
            self.opens = ran || (enabled && forward) || (hovered && waited >= SUBMENU_DELAY);
            return;
        }
        if ran {
            if let Some(reference) = self.command_ref.clone() {
                commands::fire(&ctx, reference, FireSource::Menu);
            }
            crate::runtime::overlay::note_activated(&ctx, cx.id);
            self.ran = true;
        }
    }

    /// Running a row is reported through the output and closes the menus here, so the pipeline's own click and
    /// command firing stay off.
    fn activated(&self, _: &Cx<'_, Self>) -> bool {
        false
    }

    fn flags(&self) -> Flags {
        let mut flags = Flags::empty();
        flags.set(Flags::CHECKED, self.check == Some(true) || self.radio_mark == Some(true));
        flags
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let (rtl, side, gap) = (cx.flow.is_rtl(), cx.style.icon.side, cx.style.metrics.gap);
        let level = cx.env.get::<MenuLevel>();
        let command = cx.command.as_deref();
        let marked = self.marked_row(command) || level.marks();
        let iconed = self.lead_icon(command).is_some() || level.icons();
        let (text, underline) = shown(cx.resolved.label, level.letter_of(cx.id));
        let draw = ui.draw();
        let detail = self.detail.as_ref().map(|detail| cx.env.resolve(detail));
        let detail_run =
            detail.as_ref().map(|detail| TextRun::new(detail.as_str()).role(cx.style.detail.role).color(cx.resolve(&cx.style.detail.color)));
        let detail_width = detail_run.as_ref().map_or(0.0, |run| draw.layout(run, f32::INFINITY).size().x);
        let trail = if self.submenu { side } else { cx.part_if_declared(SHORTCUT).map_or(0.0, |part| part.rect.width()) };
        let columns = Columns { mark: if marked { side } else { 0.0 }, icon: if iconed { side } else { 0.0 }, detail: detail_width, trail };
        let placed = self.place(&columns, cx.content_rect, rtl, gap);
        let ink = cx.resolve(&cx.style.icon.color);
        let glyph = |slot: Rect| Rect::from_center_size(slot.center(), Vec2::splat(side));
        if columns.mark > 0.0 {
            let square = Rect::from_center_size(Flow::new(placed.mark, rtl).take_start(side).0.center(), Vec2::splat(side));
            let checked = self.check.or(self.radio_mark).or_else(|| command.and_then(|command| command.checked)) == Some(true);
            if checked {
                if self.radio_mark.is_some() {
                    draw.fill(&PaintShape::Circle(square.center(), side * DOT_RADIUS), ink);
                } else {
                    let at = |(x, y): (f32, f32)| -> Pos2 { square.min + Vec2::new(x, y) * side };
                    draw.stroke(&PaintShape::Path(TICK.map(at).to_vec()), StrokeWidth::Medium, ink);
                }
            }
        }
        if columns.icon > 0.0
            && let Some(icon) = self.lead_icon(command)
        {
            let slot = Flow::new(placed.icon, rtl).take_start(side).0;
            let icon = if icon.color.is_some() { icon } else { icon.color(ink) };
            draw.icon(glyph(slot), &icon);
        }
        let run = TextRun::new(text.as_str()).elide().role(cx.style.text.role).color(cx.resolve(&cx.style.text.color));
        let layout = draw.layout(&run, placed.label.width());
        let size = layout.size();
        let x = if rtl { placed.label.max.x - size.x } else { placed.label.min.x };
        let y = placed.label.center().y - size.y / 2.0;
        draw.galley(pos2(x, y), &layout);
        if let Some(at) = underline.filter(|_| !rtl && level.keyboard() && !cx.env.shell().os.is_mac()) {
            let before: String = text.chars().take(at).collect();
            let letter: String = text.chars().skip(at).take(1).collect();
            let measure = |s: &str| if s.is_empty() { 0.0 } else { draw.layout(&TextRun::new(s).role(cx.style.text.role), f32::INFINITY).size().x };
            let (from, width) = (x + measure(&before), measure(&letter));
            let line = y + size.y;
            draw.stroke(&PaintShape::Path(vec![pos2(from, line), pos2(from + width, line)]), StrokeWidth::Thin, cx.resolve(&cx.style.text.color));
        }
        if let Some(run) = &detail_run {
            let layout = draw.layout(run, f32::INFINITY);
            let size = layout.size();
            let x = if rtl { placed.detail.min.x } else { placed.detail.max.x - size.x };
            draw.galley(pos2(x, placed.detail.center().y - size.y / 2.0), &layout);
        }
        if self.submenu {
            let slot = Flow::new(placed.trail, rtl).take_end(side).0;
            draw.icon(glyph(slot), &IconSpec::svg(IconId::new("chevron_right")).color(ink));
        } else if cx.part_if_declared(SHORTCUT).is_some()
            && let Some(keys) = self.keys(command)
        {
            ui.face(SHORTCUT, keys);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if self.radio_mark.is_some() {
            access.role(Role::MENU_ITEM_RADIO);
        } else if self.check.is_some() || cx.command.as_deref().is_some_and(|command| command.checked.is_some()) {
            access.role(Role::MENU_ITEM_CHECK_BOX);
        }
        if self.submenu {
            access.has_popup(PopupKind::Menu);
        }
        if let Some(letter) = cx.env.get::<MenuLevel>().letter_of(cx.id) {
            access.access_key(letter);
        }
        if let Some(text) = &self.shortcut {
            access.shortcut(cx.env.resolve(text).as_str().to_owned());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> MenuItemOutput {
        MenuItemOutput { response: cx.response.clone(), ran: self.ran, opens_submenu: self.opens }
    }
}

/// How long the pointer has rested on a submenu row.
#[derive(Clone, Default)]
struct Dwell {
    since: Option<f64>,
}
