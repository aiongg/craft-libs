//! [`Toast`] and [`ToastStack`]: transient messages in a corner of the window.

use std::sync::Arc;
use std::time::Duration;

use craft_theme::{ControlSize, Tone, TypeRole};
use egui::epaint::Shape;
use egui::text::LayoutJob;
use egui::{Align, AreaState, Context, Galley, Id, Label, LayerId, Layout, Order, Rect, Response, Ui, UiBuilder, vec2};

use super::live;
use super::notice::tone_icon;
use crate::controls::IconButton;
use crate::direction;
use crate::icons::{Icon, IconSize};
use crate::primitives::{Surface, SurfaceEdge, SurfaceKind};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, UiTokens, corner_radius, reshape_shadow, role_format};
use crate::typed_story;

/// Most cards shown at once.
const MAX_TOASTS: usize = 3;
/// Most bullet lines a card shows; the rest become one "…and n more" line.
const MAX_LINES: usize = 8;

/// A card's sizes, type roles and colours, from [`ToastStack::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToastStyle {
    /// Outer width: `toast.width`.
    pub width: f32,
    /// Between the card's edge and its content: `toast.pad`.
    pub pad: f32,
    /// From the host rect's edge at the reading end and from its bottom edge: `toast.inset`.
    pub inset: [f32; 2],
    /// Between cards: `toast.gap`.
    pub gap: f32,
    /// Corner radius: `toast.radius`.
    pub radius: f32,
    /// Card fill: `toast.fill`.
    pub fill: egui::Color32,
    /// Offset of the card's shadow: `toast.shadow.x` and `.y`. Its colour is the popup's.
    pub shadow_offset: [f32; 2],
    /// Blur of the card's shadow: `toast.shadow.blur`.
    pub shadow_blur: f32,
    /// Outline of a flat card: `toast.<tone>.edge`.
    pub edge: egui::Color32,
    /// Title: `toast.<tone>.title`.
    pub title: egui::Color32,
    /// Bullet lines: `toast.line`.
    pub line: egui::Color32,
    /// The "…and n more" line: `toast.more`.
    pub more: egui::Color32,
    /// Side of the tone icon: `icon.md`.
    pub icon: f32,
    pub title_role: TypeRole,
    pub line_role: TypeRole,
}

/// One message for [`ToastStack::show`]. The host keeps its list of toasts and passes one of
/// these per frame.
#[derive(Clone, Debug)]
#[must_use = "pass the toast to `ToastStack::show`"]
pub struct Toast {
    id: u64,
    title: String,
    tone: Tone,
    lines: Vec<String>,
    sticky: bool,
}

impl Toast {
    /// A neutral toast. `id` is the host's own, reported back when the toast ends.
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self { id, title: title.into(), tone: Tone::Neutral, lines: Vec::new(), sticky: false }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Bullet lines under the title.
    pub fn lines<S: Into<String>>(mut self, lines: impl IntoIterator<Item = S>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    /// Marks a toast the user asked to keep (PhotoCraft's "don't show again" ones): when there are
    /// too many, the oldest toast that is not sticky is dropped first.
    pub fn sticky(mut self, sticky: bool) -> Self {
        self.sticky = sticky;
        self
    }
}

/// Why a toast ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEnd {
    /// The user pressed "Dismiss".
    Closed,
    /// The auto-hide time passed.
    Expired,
    /// Too many toasts were passed; this was the oldest.
    Dropped,
}

/// What the stack remembers between frames.
#[derive(Clone, Debug, Default)]
struct State {
    /// The time each toast was first drawn, by id.
    shown_at: Vec<(u64, f64)>,
    /// The cards' rect on the last frame.
    rect: Option<Rect>,
    /// The "Dismiss" buttons of the last frame, by toast id, oldest first.
    dismiss: Vec<(u64, Id)>,
    /// The last widget outside the stack that had keyboard focus.
    focus_before: Option<Id>,
    /// Whether the timers were paused on the last frame.
    paused: bool,
}

/// A card's texts, laid out for its width, and its height.
struct Laid {
    title: Arc<Galley>,
    bullet: Arc<Galley>,
    lines: Vec<Arc<Galley>>,
    more: Option<(String, Arc<Galley>)>,
    /// Height of the row of icon, title and "Dismiss".
    row: f32,
    height: f32,
}

/// A stack of toast cards in the bottom corner at the reading end of a rect: bottom right, or
/// bottom left in a right-to-left interface. Newest at the bottom, nearest the corner, at most
/// three; each card is a popup surface with the tone's icon, title and edge, an optional list of
/// bullet lines and a "Dismiss" button.
///
/// ```
/// use std::time::Duration;
/// use craft_ui::composites::{Toast, ToastEnd, ToastStack, Tone};
/// # egui::__run_test_ui(|ui| {
/// let body = ui.max_rect();
/// let ended = ToastStack::new("toasts")
///     .auto_hide(Some(Duration::from_secs(6)))
///     .show(ui.ctx(), body, [Toast::new(1, "Couldn't save").tone(Tone::Bad).lines(["Disk is full"])]);
/// for (id, end) in ended { /* remove toast `id`; `ToastEnd::Closed` is the user's own choice */ }
/// # });
/// ```
///
/// - **Data stays with the host.** The stack keeps only timers, in egui memory by stack id and
///   toast id. It returns the toasts that ended this frame; the host removes them from its list.
/// - **Placement:** the cards are laid out from the corner upwards in the frame they appear,
///   each measured before it is drawn, so a new card is in place at once.
/// - **Auto-hide:** [`auto_hide`](ToastStack::auto_hide), six seconds by default, applies to every
///   toast from the first frame it is drawn; `None` keeps them until closed. While the pointer is
///   over the stack, or keyboard focus is inside it, every timer restarts each frame, and once
///   more when the pointer leaves, so the time spent there never counts.
/// - **Keyboard:** "Dismiss" is an ordinary Tab stop with the focus ring, oldest card first. The
///   host calls [`show`](ToastStack::show) after the rest of its interface, so the toasts come
///   last in the Tab order (egui orders Tab stops as they are drawn, whatever the layer). Focus
///   never moves to a toast on its own, and Esc is not bound. When a toast whose "Dismiss" has
///   focus ends, focus moves to the next card's "Dismiss" (the previous card's, after the newest),
///   or back to the widget that had it before the stack, after the last card.
/// - **AccessKit:** each card is a live region: `Status` and polite, or for [`Tone::Bad`] `Alert`
///   and assertive, named "title, line, line", so a new toast is announced once. Its node id
///   comes from the stack id and the toast id, so a card that moves when another ends is not
///   announced again. Everything inside it is silent. The tone icon and the bullets have no node.
/// - The text can be selected and copied. The card is on a foreground layer, so a drag on it
///   never reaches the canvas.
#[must_use = "show the stack with `show`"]
pub struct ToastStack {
    id: Id,
    auto_hide: Option<Duration>,
    dismiss_label: String,
    more_label: Box<dyn Fn(usize) -> String>,
}

impl ToastStack {
    /// A stack with this id salt.
    pub fn new(id_salt: impl egui::AsIdSalt) -> Self {
        Self {
            id: Id::unique("craft_ui::composites::ToastStack").with(id_salt),
            auto_hide: Some(Duration::from_secs(6)),
            dismiss_label: "Dismiss".to_owned(),
            more_label: Box::new(|n| format!("…and {n} more")),
        }
    }

    /// How long a toast stays before it ends; `None` keeps it until it is closed.
    pub fn auto_hide(mut self, duration: Option<Duration>) -> Self {
        self.auto_hide = duration;
        self
    }

    /// The accessible name and tooltip of the close button; the host passes the translation.
    pub fn dismiss_label(mut self, label: impl Into<String>) -> Self {
        self.dismiss_label = label.into();
        self
    }

    /// The "…and n more" line, given the number of lines left out.
    pub fn more_label(mut self, label: impl Fn(usize) -> String + 'static) -> Self {
        self.more_label = Box::new(label);
        self
    }

    pub fn style(tokens: &UiTokens, tone: Tone) -> ToastStyle {
        let t = &tokens.component.toast;
        ToastStyle {
            width: t.width,
            pad: t.pad,
            inset: t.inset,
            gap: t.gap,
            radius: t.radius,
            fill: t.fill.to_egui(),
            shadow_offset: t.shadow_offset,
            shadow_blur: t.shadow_blur,
            edge: t.edge(tone).to_egui(),
            title: t.title(tone).to_egui(),
            line: t.line.to_egui(),
            more: t.more.to_egui(),
            icon: IconSize::Md.points(tokens),
            title_role: TypeRole::BodyStrong,
            line_role: TypeRole::Body,
        }
    }

    /// Shows `toasts`, oldest first, above the bottom edge of `rect` (the host's body area, above
    /// its status bar). Call it after the rest of the interface. Returns the toasts that ended
    /// this frame.
    pub fn show(self, ctx: &Context, rect: Rect, toasts: impl IntoIterator<Item = Toast>) -> Vec<(u64, ToastEnd)> {
        let mut ended = Vec::new();
        let mut toasts: Vec<Toast> = toasts.into_iter().collect();
        while toasts.len() > MAX_TOASTS {
            let at = toasts.iter().position(|t| !t.sticky).unwrap_or(0);
            ended.push((toasts.remove(at).id, ToastEnd::Dropped));
        }
        let mut state = ctx.data(|d| d.get_temp::<State>(self.id)).unwrap_or_default();
        let now = ctx.input(|i| i.time);

        let focus = ctx.memory(|m| m.focused());
        let focused = focus.and_then(|f| state.dismiss.iter().position(|(_, id)| *id == f));
        // Tab clears the focus for the rest of the frame in which it leaves a widget, so only
        // a focused widget is remembered.
        if focused.is_none() && focus.is_some() {
            state.focus_before = focus;
        }
        let hovered = state.rect.is_some_and(|r| ctx.pointer_hover_pos().is_some_and(|p| r.contains(p)));
        let paused = hovered || focused.is_some();
        state.shown_at.retain(|(id, _)| toasts.iter().any(|t| t.id == *id));
        for toast in &toasts {
            if !state.shown_at.iter().any(|(id, _)| *id == toast.id) {
                state.shown_at.push((toast.id, now));
            }
        }
        if paused || state.paused {
            state.shown_at.iter_mut().for_each(|(_, at)| *at = now);
        }
        state.paused = paused;
        if let Some(limit) = self.auto_hide {
            let expired = |at: f64| now - at >= limit.as_secs_f64();
            for toast in &toasts {
                if state.shown_at.iter().any(|(id, at)| *id == toast.id && expired(*at)) {
                    ended.push((toast.id, ToastEnd::Expired));
                }
            }
            toasts.retain(|t| !ended.iter().any(|(id, _)| *id == t.id));
            let next = state.shown_at.iter().map(|(_, at)| limit.as_secs_f64() - (now - at)).fold(f64::INFINITY, f64::min);
            if next.is_finite() {
                ctx.request_repaint_after(Duration::from_secs_f64(next.max(0.0)));
            }
        }

        let tokens = crate::theme::current(ctx);
        let rtl = direction::is_rtl(ctx);
        let base = ToastStack::style(&tokens, Tone::Neutral);
        let width = base.width.min((rect.width() - 2.0 * base.inset[0]).max(0.0));
        let x = if rtl { rect.left() + base.inset[0] } else { rect.right() - base.inset[0] - width };
        let column = Rect::from_x_y_ranges(x..=x + width, rect.top()..=rect.bottom() - base.inset[1]);

        // A layer of its own rather than an `Area`, which would place the cards with the size
        // they had on the last frame.
        let layer = LayerId::new(Order::Foreground, self.id);
        // Bottom up, so the layer's own rect starts at the corner and covers the cards alone, and a
        // click above them reaches what is under the column.
        let layout = Layout::bottom_up(Align::Min);
        let mut ui = Ui::new(ctx.clone(), self.id.with("stack"), UiBuilder::new().layer_id(layer).max_rect(column).layout(layout));
        ui.set_clip_rect(ctx.content_rect());
        let laid: Vec<Laid> = toasts.iter().map(|toast| self.lay_out(&ui, &tokens, toast, width)).collect();
        let mut bottom = column.bottom();
        let mut cards = vec![Rect::NOTHING; toasts.len()];
        for (card, laid) in cards.iter_mut().zip(&laid).rev() {
            *card = Rect::from_x_y_ranges(column.x_range(), bottom - laid.height..=bottom);
            bottom = card.top() - base.gap;
        }
        let mut dismiss = Vec::new();
        for ((toast, laid), card) in toasts.iter().zip(&laid).zip(&cards) {
            let (response, closed) = self.card(&mut ui, &tokens, toast, laid, *card);
            if closed {
                ended.push((toast.id, ToastEnd::Closed));
            }
            dismiss.push((toast.id, response.id));
        }
        let shown = cards.iter().fold(Rect::NOTHING, |all, card| all.union(*card));
        state.rect = (!toasts.is_empty()).then_some(shown);
        if !toasts.is_empty() {
            ctx.memory_mut(|m| {
                let areas = m.areas_mut();
                if !areas.visible_last_frame(&layer) {
                    areas.move_to_top(layer);
                }
                // Registered like an `Area`, so the pointer over a card hovers and scrolls the
                // card, not what is under it.
                areas.set_state(layer, AreaState { pivot_pos: Some(shown.min), size: Some(shown.size()), ..AreaState::default() });
            });
        }

        if let Some(at) = focused {
            let is_kept = |id: &u64| dismiss.iter().any(|(t, _)| t == id) && !ended.iter().any(|(t, _)| t == id);
            if !is_kept(&state.dismiss[at].0) {
                let next = state.dismiss[at + 1..].iter().chain(state.dismiss[..at].iter().rev()).find(|(t, _)| is_kept(t));
                let target = next.and_then(|(t, _)| dismiss.iter().find(|(d, _)| d == t)).map(|(_, id)| *id);
                match target.or(state.focus_before) {
                    Some(id) => ctx.memory_mut(|m| m.request_focus(id)),
                    None => {
                        if let Some(id) = focus {
                            ctx.memory_mut(|m| m.surrender_focus(id));
                        }
                    }
                }
            }
        }
        state.dismiss = dismiss;
        ctx.data_mut(|d| d.insert_temp(self.id, state));
        if !ended.is_empty() {
            ctx.request_repaint();
        }
        ended
    }

    /// Lays out `toast`'s texts for a card `width` wide and measures the card, so the stack can
    /// place it before it is drawn. [`card`](Self::card) draws these galleys in this geometry.
    fn lay_out(&self, ui: &Ui, tokens: &UiTokens, toast: &Toast, width: f32) -> Laid {
        let style = ToastStack::style(tokens, toast.tone);
        let geometry = Geometry::new(tokens, &style, toast.tone, width);
        let start = direction::start_align(ui);
        let galley = |role, color, text: &str, wrap: f32| {
            let mut job = LayoutJob::single_section(text.to_owned(), role_format(tokens, role, color));
            job.wrap.max_width = wrap;
            job.halign = start;
            job.language = ui.language();
            ui.painter().layout_job(job)
        };
        let title = galley(style.title_role, style.title, &toast.title, geometry.room);
        let mut bullet = LayoutJob::single_section(BULLET.to_owned(), role_format(tokens, style.line_role, style.line));
        bullet.language = ui.language();
        let bullet = ui.painter().layout_job(bullet);
        let line_width = (geometry.inner - bullet.size().x - geometry.gap).max(0.0);
        let lines: Vec<_> = toast.lines.iter().take(MAX_LINES).map(|line| galley(style.line_role, style.line, line, line_width)).collect();
        let more = (toast.lines.len() > MAX_LINES).then(|| {
            let text = (self.more_label)(toast.lines.len() - MAX_LINES);
            let laid = galley(style.line_role, style.more, &text, geometry.inner);
            (text, laid)
        });
        let row = geometry.first_line.max(geometry.title_offset + title.size().y).max(geometry.icon_height);
        let below: f32 =
            lines.iter().map(|l| l.size().y.max(bullet.size().y)).chain(more.iter().map(|(_, m)| m.size().y)).map(|h| style.gap + h).sum();
        Laid { title, bullet, lines, more, row, height: 2.0 * style.pad + row + below }
    }

    /// Draws one card in `rect`. Returns the "Dismiss" button's response and whether it was
    /// pressed.
    fn card(&self, ui: &mut Ui, tokens: &UiTokens, toast: &Toast, laid: &Laid, rect: Rect) -> (Response, bool) {
        let style = ToastStack::style(tokens, toast.tone);
        let geometry = Geometry::new(tokens, &style, toast.tone, rect.width());
        let slot = ui.painter().add(Shape::Noop);
        let (role, politeness) = live::for_tone(toast.tone);
        let content = UiBuilder::new().max_rect(rect.shrink(style.pad)).layout(direction::column(ui));
        let button = ui
            .scope_builder(content, |ui| {
                live::region(ui, self.id.with(toast.id), role, politeness, |ui, labels| {
                    ui.spacing_mut().item_spacing = vec2(geometry.gap, style.gap);
                    let mut row = direction::row(ui);
                    row.cross_align = Align::Min;
                    let button = ui.allocate_ui_with_layout(vec2(geometry.inner, laid.row), row, |ui| {
                        tone_icon(ui, toast.tone, IconSize::Md, geometry.first_line, style.title);
                        ui.allocate_ui_with_layout(vec2(geometry.room, laid.row), direction::column(ui), |ui| {
                            ui.set_min_width(geometry.room);
                            ui.add_space(geometry.title_offset);
                            ui.add(Label::new(laid.title.clone()));
                            labels.add(toast.title.as_str());
                        });
                        let (_, square) = ui.allocate_space(vec2(geometry.side, geometry.side));
                        IconButton::new(Icon::Close, self.dismiss_label.as_str()).size(ControlSize::Sm).id_salt("dismiss").show_at(ui, square)
                    });
                    for (line, galley) in toast.lines.iter().zip(&laid.lines) {
                        let height = galley.size().y.max(laid.bullet.size().y);
                        ui.allocate_ui_with_layout(vec2(geometry.inner, height), row, |ui| {
                            // Painted, so the line's text node is the line alone.
                            let (_, mark) = ui.allocate_space(laid.bullet.size());
                            ui.painter().galley(mark.min, laid.bullet.clone(), style.line);
                            ui.add(Label::new(galley.clone()));
                        });
                        labels.add(line.as_str());
                    }
                    if let Some((text, galley)) = &laid.more {
                        ui.add(Label::new(galley.clone()));
                        labels.add(text.as_str());
                    }
                    button.inner
                })
                .inner
            })
            .inner;
        let mut surface = Surface::style(tokens, SurfaceKind::Popup);
        surface.fill = style.fill;
        surface.radius = corner_radius(style.radius);
        surface.shadow = surface.shadow.map(|shadow| reshape_shadow(shadow, style.shadow_offset, style.shadow_blur));
        if let SurfaceEdge::Border(stroke) = &mut surface.edge {
            stroke.color = style.edge;
        }
        ui.painter().set(slot, Shape::Vec(surface.shapes(rect, ui.pixels_per_point())));
        let closed = button.clicked();
        (button, closed)
    }
}

/// The bullet before each line, painted without a node.
const BULLET: &str = "•";

/// Where a card's parts go across its width.
struct Geometry {
    /// Width inside the padding.
    inner: f32,
    /// Between the icon, the title and "Dismiss", and between a bullet and its line.
    gap: f32,
    /// Side of the "Dismiss" button.
    side: f32,
    /// Width of the title's column.
    room: f32,
    /// Height of the first line: the taller of "Dismiss" and the title's line.
    first_line: f32,
    /// Above the title, to centre its first line on the first line.
    title_offset: f32,
    /// Height of the tone icon's box; 0 for Neutral.
    icon_height: f32,
}

impl Geometry {
    fn new(tokens: &UiTokens, style: &ToastStyle, tone: Tone, width: f32) -> Self {
        let inner = (width - 2.0 * style.pad).max(0.0);
        let gap = tokens.space.gap.inline;
        let side = IconButton::style(tokens, ControlSize::Sm).side;
        let title_line = tokens.typography.get(style.title_role).line_height;
        let first_line = side.max(title_line);
        let icon = (tone != Tone::Neutral).then_some(style.icon);
        let room = (inner - icon.map_or(0.0, |i| i + gap) - side - gap).max(0.0);
        let icon_height = icon.map_or(0.0, |i| i.max(first_line));
        Self { inner, gap, side, room, first_line, title_offset: (first_line - title_line) / 2.0, icon_height }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Toast"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToneArg {
    Neutral,
    Info,
    Good,
    Warn,
    Bad,
}

impl ToneArg {
    fn tone(self) -> Tone {
        match self {
            ToneArg::Neutral => Tone::Neutral,
            ToneArg::Info => Tone::Info,
            ToneArg::Good => Tone::Good,
            ToneArg::Warn => Tone::Warn,
            ToneArg::Bad => Tone::Bad,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct StackArgs {
    tone: ToneArg,
    auto_hide: bool,
    #[arg(range = 1.0..=30.0)]
    seconds: f64,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/toast/stack",
        Level::Composites,
        "Toast",
        "Stack",
        StackArgs { tone: ToneArg::Neutral, auto_hide: false, seconds: 6.0 },
        show_stack
    )]
}

/// A neutral notice, a warning with ten lines and an error in the bottom corner of the story's
/// area; closed ones stay gone.
pub(super) fn show_stack(ui: &mut Ui, args: &mut StackArgs) {
    let tone = args.tone.tone();
    let auto_hide = args.auto_hide.then(|| Duration::from_secs_f64(args.seconds));
    let id = ui.scope_id().with("toast-story");
    let mut closed: Vec<u64> = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let area = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(area.min, vec2(area.width(), area.height().min(420.0)));
    ui.allocate_rect(rect, egui::Sense::hover());
    let lines: Vec<String> = (1..=10).map(|n| format!("Warning {n}")).collect();
    let all = [
        Toast::new(1, "Pasted from the clipboard").tone(tone),
        Toast::new(2, "Opened with 10 warnings").tone(Tone::Warn).lines(lines),
        Toast::new(3, "Couldn't save the document").tone(Tone::Bad).lines(["The disk is full."]),
    ];
    let toasts = all.into_iter().filter(|t| !closed.contains(&t.id));
    for (ended, _) in ToastStack::new("toast-story").auto_hide(auto_hide).show(ui.ctx(), rect, toasts) {
        closed.push(ended);
    }
    ui.data_mut(|d| d.insert_temp(id, closed));
}
