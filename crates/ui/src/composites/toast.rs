//! [`Toast`] and [`ToastStack`]: transient messages in a corner of the window.

use std::time::Duration;

use craft_theme::{ControlSize, Tone, TypeRole};
use egui::epaint::Shape;
use egui::{Align, Align2, Area, Context, Frame, Id, Order, Rect, Response, Ui, UiBuilder, pos2, vec2};

use super::live;
use super::notice::{text, tone_icon};
use crate::controls::IconButton;
use crate::direction;
use crate::icons::{Icon, IconSize};
use crate::primitives::{Surface, SurfaceEdge, SurfaceKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, UiTokens, corner_radius, margin};

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
    /// The stack's rect on the last frame.
    rect: Option<Rect>,
    /// The "Dismiss" buttons of the last frame.
    dismiss: Vec<Id>,
    /// Whether the timers were paused on the last frame.
    paused: bool,
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
/// - **Auto-hide:** [`auto_hide`](ToastStack::auto_hide), six seconds by default, applies to every
///   toast from the first frame it is drawn; `None` keeps them until closed. While the pointer is
///   over the stack, or keyboard focus is inside it, every timer restarts each frame, and once
///   more when the pointer leaves, so the time spent there never counts.
/// - **Keyboard:** "Dismiss" is an ordinary Tab stop with the focus ring, last in the order since
///   the stack is drawn last. Focus never moves to a toast on its own, and Esc is not bound.
/// - **AccessKit:** each card is a live region: `Status` and polite, or for [`Tone::Bad`] `Alert`
///   and assertive, named by its title and lines, so a new toast is announced once. Everything
///   inside it is silent. The tone icon has no node.
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
    /// its status bar). Returns the toasts that ended this frame.
    pub fn show(self, ctx: &Context, rect: Rect, toasts: impl IntoIterator<Item = Toast>) -> Vec<(u64, ToastEnd)> {
        let mut ended = Vec::new();
        let mut toasts: Vec<Toast> = toasts.into_iter().collect();
        while toasts.len() > MAX_TOASTS {
            let at = toasts.iter().position(|t| !t.sticky).unwrap_or(0);
            ended.push((toasts.remove(at).id, ToastEnd::Dropped));
        }
        let mut state = ctx.data(|d| d.get_temp::<State>(self.id)).unwrap_or_default();
        let now = ctx.input(|i| i.time);

        let hovered = state.rect.is_some_and(|r| ctx.pointer_hover_pos().is_some_and(|p| r.contains(p)));
        let focused = ctx.memory(|m| m.focused()).is_some_and(|f| state.dismiss.contains(&f));
        let paused = hovered || focused;
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
        let (pivot, x) = if rtl { (Align2::LEFT_BOTTOM, rect.left() + base.inset[0]) } else { (Align2::RIGHT_BOTTOM, rect.right() - base.inset[0]) };
        let pos = pos2(x, rect.bottom() - base.inset[1]);

        let mut dismiss = Vec::new();
        let area = Area::new(self.id.with("area")).order(Order::Foreground).pivot(pivot).fixed_pos(pos).show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = base.gap;
            for toast in &toasts {
                let (response, closed) = self.card(ui, &tokens, toast, width);
                dismiss.push(response.id);
                if closed {
                    ended.push((toast.id, ToastEnd::Closed));
                }
            }
        });
        state.rect = (!toasts.is_empty()).then_some(area.response.rect);
        state.dismiss = dismiss;
        ctx.data_mut(|d| d.insert_temp(self.id, state));
        if !ended.is_empty() {
            ctx.request_repaint();
        }
        ended
    }

    /// One card `width` wide. Returns the "Dismiss" button's response and whether it was pressed.
    fn card(&self, ui: &mut Ui, tokens: &UiTokens, toast: &Toast, width: f32) -> (Response, bool) {
        let style = ToastStack::style(tokens, toast.tone);
        let slot = ui.painter().add(Shape::Noop);
        let (role, politeness) = live::for_tone(toast.tone);
        let mut button = None;
        let card = live::region(ui, role, politeness, |ui, labels| {
            ui.set_width(width);
            Frame::new().inner_margin(margin(style.pad)).show(ui, |ui| {
                ui.set_width((width - 2.0 * style.pad).max(0.0));
                let side = IconButton::style(tokens, ControlSize::Sm).side;
                let title_line = tokens.typography.get(style.title_role).line_height;
                let first_line = side.max(title_line);
                let mut row = direction::row(ui);
                row.cross_align = Align::Min;
                ui.scope_builder(UiBuilder::new().layout(row), |ui| {
                    ui.spacing_mut().item_spacing.x = tokens.space.gap.inline;
                    tone_icon(ui, toast.tone, IconSize::Md, first_line, style.title);
                    let room = (ui.available_width() - side - ui.spacing().item_spacing.x).max(0.0);
                    ui.allocate_ui_with_layout(vec2(room, 0.0), direction::column(ui), |ui| {
                        ui.set_min_width(room);
                        ui.add_space((first_line - title_line) / 2.0);
                        labels.add(&text(ui, tokens, style.title_role, style.title, &toast.title));
                    });
                    let (_, square) = ui.allocate_space(vec2(side, side));
                    button = Some(
                        IconButton::new(Icon::Close, self.dismiss_label.as_str())
                            .size(ControlSize::Sm)
                            .id_salt((toast.id, "dismiss"))
                            .show_at(ui, square),
                    );
                });
                for line in toast.lines.iter().take(MAX_LINES) {
                    labels.add(&text(ui, tokens, style.line_role, style.line, &format!("• {line}")));
                }
                if toast.lines.len() > MAX_LINES {
                    labels.add(&text(ui, tokens, style.line_role, style.more, &(self.more_label)(toast.lines.len() - MAX_LINES)));
                }
            });
        });
        let mut surface = Surface::style(tokens, SurfaceKind::Popup);
        surface.fill = style.fill;
        surface.radius = corner_radius(style.radius);
        if let SurfaceEdge::Border(stroke) = &mut surface.edge {
            stroke.color = style.edge;
        }
        ui.painter().set(slot, Shape::Vec(surface.shapes(card.response.rect, ui.pixels_per_point())));
        let button = button.unwrap_or_else(|| card.response.clone());
        let closed = button.clicked();
        (button, closed)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Toast"];

const TONE_NAMES: &[&str] = &["neutral", "info", "good", "warn", "bad"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "composites/toast/stack",
        level: Level::Composites,
        component: "Toast",
        name: "Stack",
        args: || Args::new().with_choice("tone", TONE_NAMES, 0).with_bool("auto_hide", false).with_number("seconds", 6.0, 1.0, 30.0),
        show: show_stack,
    }]
}

/// A neutral notice, a warning with ten lines and an error in the bottom corner of the story's
/// area; closed ones stay gone.
pub(super) fn show_stack(ui: &mut Ui, args: &mut Args) {
    let tone = Tone::from_name(args.choice("tone")).unwrap_or_default();
    let auto_hide = args.bool("auto_hide").then(|| Duration::from_secs_f64(args.number("seconds")));
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
