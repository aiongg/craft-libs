//! [`Tour`]: coach marks that walk the user through the interface, one step at a time, each
//! pointing at a part of the window.

use std::time::Duration;

use craft_theme::Stroke as ThemeStroke;
use egui::{Area, AsIdSalt, Context, Id, LayerId, Order, Pos2, Rect, StrokeKind, Ui, Vec2, pos2, vec2};

use crate::controls::{Button, ButtonVariant, ControlSize, IconButton};
use crate::direction;
use crate::icons::Icon;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::popover::{CloseReason, Popover, PopoverAnchor, PopoverClose, SCREEN_MARGIN};
use crate::primitives::{Content, Corner, Gap, Stack, Text};
use crate::text::text_width;
use crate::theme::{ToEgui, UiTokens, current};
use craft_theme::TypeRole;

/// Pulse speed in radians per second.
const PULSE_RATE: f64 = 4.0;
/// Share of the window's width the card takes at most.
const MAX_CARD_SHARE: f32 = 0.3;
/// How far the highlight outline grows at the top of its pulse.
const HIGHLIGHT_PULSE: f32 = 2.0;
const REPAINT: Duration = Duration::from_millis(33);

/// One step: what it points at and what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct TourStep {
    /// The automation id of the step (`layers-panel`).
    pub id: String,
    /// The card's heading. Translate it in the app.
    pub title: String,
    /// The card's text. Translate it in the app.
    pub body: String,
    /// The rect of the part it points at this frame; `None` shows the card centred with no
    /// highlight (a welcome or closing step).
    pub target: Option<Rect>,
}

/// What the user did with the tour this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TourEvent {
    /// Next, or Enter: the host moves to the following step.
    Next,
    /// Back: the host moves to the previous step.
    Back,
    /// Done on the last step.
    Finished,
    /// Skip Tour, the close button or Esc.
    Skipped,
}

/// Coach marks over the window: the target's outline pulses, and a card beside it (on the side
/// with room, else bottom right) shows the step's title and body, "2 of 6", Back, Next (Done on
/// the last step) and Skip Tour.
///
/// ```no_run
/// use craft_ui::patterns::{Tour, TourEvent, TourStep};
/// # fn demo(ctx: &egui::Context, steps: &[TourStep], step: &mut usize, running: &mut bool) {
/// match Tour::new("welcome").show(ctx, steps, *step) {
///     Some(TourEvent::Next) => *step += 1,
///     Some(TourEvent::Back) => *step -= 1,
///     Some(TourEvent::Finished | TourEvent::Skipped) => *running = false,
///     _ => {}
/// }
/// # }
/// ```
///
/// - **Data stays with the host:** the steps, the current index and whether the tour runs (and
///   was finished, for a "don't show again" preference).
/// - **Pointer:** the rest of the window stays usable; the card and highlight take no clicks
///   outside the card. A step whose target is gone (panel closed) shows centred.
/// - **Keyboard:** the card takes focus when a step shows (Next focused); Enter is Next,
///   Esc skips, Tab moves between the card's buttons. Focus returns where it was when the tour
///   ends.
/// - **Motion:** the highlight's pulse stops under reduced motion (`motion.reduced`), leaving a
///   steady outline.
/// - **AccessKit:** the card is a `Dialog` (not modal) named by the step's title, its body the
///   description; each new step is announced.
#[must_use = "show the tour with `show`"]
pub struct Tour {
    id: Id,
    labels: Option<TourLabels>,
}

/// Where the keyboard focus was when the tour began, to give it back when the tour ends.
#[derive(Clone, Copy, Default)]
struct Visit {
    frame: u64,
    previous: Option<Id>,
}

/// The card's inner width: the widest title or body line of any step, so the card keeps its width
/// from step to step, at least what the footer's buttons need and at most [`MAX_CARD_SHARE`] of a
/// `window` wide window (the footer's width wins over the share).
fn card_width(ui: &Ui, tokens: &UiTokens, labels: &TourLabels, steps: &[TourStep], window: f32) -> f32 {
    let (title, body) = (tokens.font(TypeRole::Title), tokens.font(TypeRole::Body));
    let text = steps.iter().map(|s| text_width(ui, &s.title, &title).max(text_width(ui, &s.body, &body))).fold(0.0, f32::max);
    let label = tokens.font(TypeRole::Label);
    let button = |name: &str| text_width(ui, name, &label) + tokens.size.control.md;
    let footer = button(&labels.skip) + button(&labels.back) + button(&labels.next).max(button(&labels.done)) + 2.0 * tokens.space.gap.inline;
    text.min(window * MAX_CARD_SHARE).max(footer).min((window - 2.0 * SCREEN_MARGIN).max(footer))
}

/// The size the card with `step` takes before it has been laid out once: the stacked rows at
/// `width`, inside the popover's margin.
fn card_estimate(ui: &Ui, tokens: &UiTokens, step: &TourStep, width: f32) -> Vec2 {
    let line = |role: TypeRole| tokens.typography.get(role).line_height;
    let body = ui.painter().layout(step.body.clone(), tokens.font(TypeRole::Body), egui::Color32::PLACEHOLDER, width).size().y;
    let rows = [line(TypeRole::Caption).max(tokens.size.control.sm), line(TypeRole::Title), body, tokens.size.control.md];
    let margin = 2.0 * tokens.space.gap.stack;
    vec2(width + margin, rows.iter().sum::<f32>() + (rows.len() - 1) as f32 * tokens.space.gap.stack + margin)
}

/// The card's top left corner. Beside the target on the first side with room (right, left, below,
/// above), else at the bottom right; centred without a target. `margin` is kept to the screen's
/// edge.
pub(super) fn card_position(target: Option<Rect>, card: Vec2, screen: Rect, margin: f32, gap: f32) -> Pos2 {
    let room = screen.shrink(margin);
    let clamp_x = |x: f32| x.clamp(room.min.x, (room.max.x - card.x).max(room.min.x));
    let clamp_y = |y: f32| y.clamp(room.min.y, (room.max.y - card.y).max(room.min.y));
    let Some(t) = target else {
        return pos2(clamp_x(room.center().x - card.x / 2.0), clamp_y(room.center().y - card.y / 2.0));
    };
    let beside = [
        pos2(t.max.x + gap, clamp_y(t.center().y - card.y / 2.0)),
        pos2(t.min.x - gap - card.x, clamp_y(t.center().y - card.y / 2.0)),
        pos2(clamp_x(t.center().x - card.x / 2.0), t.max.y + gap),
        pos2(clamp_x(t.center().x - card.x / 2.0), t.min.y - gap - card.y),
    ];
    beside
        .into_iter()
        .find(|&at| room.contains_rect(Rect::from_min_size(at, card)))
        .unwrap_or_else(|| pos2(clamp_x(room.max.x - card.x), clamp_y(room.max.y - card.y)))
}

impl Tour {
    /// A tour with English labels. `id_salt` keys the card's state.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id: Id::unique(id_salt), labels: None }
    }

    /// The texts, translated. Without them the installed words are used.
    pub fn labels(mut self, labels: TourLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows step `index` of `steps` on the foreground layer; returns the user's move this frame.
    // effectcraft: crates/ui-egui/src/panels/learn.rs:80 `coach` (card bottom right, target pulse
    // `sin(time * 4)`, 2 pt stroke, repaint every 33 ms, hidden under the Home screen).
    pub fn show(self, ctx: &Context, steps: &[TourStep], index: usize) -> Option<TourEvent> {
        let step = steps.get(index)?;
        let last = index + 1 == steps.len();
        let labels = self.labels.clone().unwrap_or_else(|| TourLabels::of_ctx(ctx));
        let numbers = LocaleNumbers::of_ctx(ctx);
        let tokens = current(ctx);
        let frame = ctx.cumulative_frame_nr();
        let visit_id = self.id.with("visit");
        let began = ctx.data(|d| d.get_temp::<Visit>(visit_id)).is_none_or(|v| v.frame + 1 < frame);
        let previous = if began { ctx.memory(|m| m.focused()) } else { ctx.data(|d| d.get_temp::<Visit>(visit_id)).and_then(|v| v.previous) };
        ctx.data_mut(|d| d.insert_temp(visit_id, Visit { frame, previous }));

        if let Some(target) = step.target {
            let still = tokens.appearance.reduced_motion;
            let pulse = if still { 1.0 } else { (0.5 + 0.5 * (ctx.input(|i| i.time) * PULSE_RATE).sin()) as f32 };
            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, self.id.with("highlight")));
            let stroke = egui::Stroke::new(
                ThemeStroke::W2.width(ctx.pixels_per_point()),
                tokens.color.accent.solid.to_egui().gamma_multiply(0.6 + 0.4 * pulse),
            );
            let rect = target.expand(tokens.space.gap.tight + HIGHLIGHT_PULSE * pulse);
            painter.rect_stroke(rect, Corner::Control.radius(&tokens), stroke, StrokeKind::Outside);
            if !still {
                ctx.request_repaint_after(REPAINT);
            }
        }

        let mut event = None;
        let host = Area::new(self.id.with("host")).order(Order::Foreground).fixed_pos(Pos2::ZERO).interactable(false);
        host.show(ctx, |ui| {
            let salt = ("card", index);
            let card_id = ui.make_persistent_id(salt);
            let card_width = card_width(ui, &tokens, &labels, steps, ctx.content_rect().width());
            let estimate = card_estimate(ui, &tokens, step, card_width);
            let size = ctx.memory(|m| m.area_rect(card_id)).map_or(estimate, |r| r.size());
            let at = card_position(step.target, size, ctx.content_rect(), SCREEN_MARGIN, tokens.space.pad.panel);
            let mut open = true;
            let shown = Popover::new(salt, PopoverAnchor::Point(at))
                .close(PopoverClose::default().outside(false).focus_left(false).enter(true))
                .label(step.title.clone())
                .show(ui, &mut open, |ui, pop| {
                    ui.set_width(card_width);
                    let mut clicked = None;
                    Stack::new(Gap::Row).show(ui, |ui| {
                        direction::horizontal(ui, |ui| {
                            let progress = craft_i18n::fmt(
                                &labels.progress,
                                &[("n", &numbers.count(index as u64 + 1)), ("total", &numbers.count(steps.len() as u64))],
                            );
                            ui.add(Text::new(progress).role(TypeRole::Caption).content(Content::Muted));
                            ui.with_layout(direction::trailing(ui), |ui| {
                                let close = IconButton::new(Icon::Close, labels.skip.clone()).size(ControlSize::Sm).id_salt("close");
                                if ui.add(close).clicked() {
                                    clicked = Some(TourEvent::Skipped);
                                }
                            });
                        });
                        ui.add(Text::new(step.title.clone()).role(TypeRole::Title));
                        ui.add(Text::new(step.body.clone()));
                        direction::horizontal(ui, |ui| {
                            if ui.add(Button::new(labels.skip.clone()).variant(ButtonVariant::Ghost).id_salt("skip")).clicked() {
                                clicked = Some(TourEvent::Skipped);
                            }
                            ui.with_layout(direction::trailing(ui), |ui| {
                                let (name, done) = if last { (&labels.done, TourEvent::Finished) } else { (&labels.next, TourEvent::Next) };
                                let next = ui.add(Button::new(name.clone()).variant(ButtonVariant::Primary).id_salt("next"));
                                pop.initial_focus(next.id);
                                if next.clicked() {
                                    clicked = Some(done);
                                }
                                if ui.add(Button::new(labels.back.clone()).enabled(index > 0).id_salt("back")).clicked() {
                                    clicked = Some(TourEvent::Back);
                                }
                            });
                        });
                    });
                    clicked
                });
            ctx.accesskit_node_builder(card_id, |node| node.set_description(step.body.clone()));
            if let Some(shown) = shown {
                event = shown.inner.or(match shown.closed_by {
                    Some(CloseReason::Escape) => Some(TourEvent::Skipped),
                    Some(CloseReason::Enter) if last => Some(TourEvent::Finished),
                    Some(CloseReason::Enter) => Some(TourEvent::Next),
                    _ => None,
                });
            }
        });

        if matches!(event, Some(TourEvent::Finished | TourEvent::Skipped)) {
            ctx.data_mut(|d| d.remove_temp::<Visit>(visit_id));
            if let Some(previous) = previous {
                ctx.memory_mut(|m| m.request_focus(previous));
            }
        }
        event
    }
}

crate::words! {
    /// The tour card's labels, English by default. `{n}` and `{total}` are the step number and count.
    #[non_exhaustive]
    pub struct TourLabels {
        back: "tour.back" = "Back";
        next: "tour.next" = "Next";
        done: "tour.done" = "Done";
        skip: "tour.skip" = "Skip Tour";
        progress: "tour.progress" = "{n} of {total}";
    }
}
