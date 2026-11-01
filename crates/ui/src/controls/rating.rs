//! [`Rating`]: one to five stars the user clicks, and [`paint_rating`] for stars drawn on
//! thumbnails and captions.

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, EventFilter, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Sense, Ui, UiBuilder, vec2};

use super::ControlSize;
use crate::behavior::{ClaimedKey, KeyClaim};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::icons::{self, Icon, IconSize};
use crate::localized::words::counts;
use crate::localized::{Labels as _, Words};
use crate::primitives::{Corner, Interactive};
use crate::theme::{ToEgui, Tokens, TokensOf as _};

crate::words! {
    /// The rating's words, English by default.
    #[non_exhaustive]
    pub struct RatingLabels {
        /// The row's accessible name.
        rating: "rating.rating" = "Rating";
        /// A star's accessible name: `{n}` its rating. A
        /// [`Counted`](crate::localized::words::Counted) word ("{n} star", "{n} stars"),
        /// written with [`Words::counted`](crate::localized::Words::counted); its forms are in
        /// the kernel's [`COUNTS`](crate::localized::words::COUNTS).
        stars: "rating.stars" = "{n} stars";
        /// The value read for no rating.
        unrated: "rating.unrated" = "Unrated";
    }
}

/// The most stars a rating has.
pub const MAX_STARS: u8 = 5;

/// How big painted stars are, for [`paint_rating`]: LightCraft's three renderers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StarSize {
    /// Film-strip badges: 8-point stars every 9 points.
    Small,
    /// Grid badges and compare captions: 12-point stars every 13.
    #[default]
    Medium,
}

impl StarSize {
    /// One star's side.
    pub fn side(self) -> f32 {
        match self {
            StarSize::Small => 8.0,
            StarSize::Medium => 12.0,
        }
    }

    /// From one star to the next.
    pub fn step(self) -> f32 {
        match self {
            StarSize::Small => 9.0,
            StarSize::Medium => 13.0,
        }
    }
}

/// A row of five stars, after LightCraft's `Rating`.
///
/// ```ignore
/// if let Some(EditEvent::Set { value, .. }) = Rating::new("rating", "Rating", &mut photo.rating).show(ui).event { app.rate(value) }
/// ```
///
/// - **Look.** Five [`Icon::Star`](crate::icons::Icon::Star) outlines, the first `rating` of them
///   [`Icon::StarFilled`](crate::icons::Icon::StarFilled), in `rating.star` (LightCraft
///   draws them in its accent), in 14-point stars on 19-point cells (Md) or 12 on 15 (Sm). In
///   right-to-left the first star is on the right.
/// - **Pointer.** Hovering star `n` previews `n` filled stars; a click sets `n`, and a click on
///   the current rating's star clears it to 0 (LightCraft's `clicked_rating`).
/// - **Keys.** One Tab stop: the star of the current rating, the first when unrated. ← and →
///   (mirrored) lower and raise the rating from 0 to 5, Home and End go to 0 and 5, the digits 0
///   to 5 set it (Lightroom's). The focused row claims the digits
///   ([`KeyClaim`](crate::behavior::KeyClaim)): a focused widget otherwise leaves digits to the
///   app's tool and opacity keys ([`focus::key_owner`](crate::focus::key_owner)).
/// - **Report.** Each click or key that changes the rating is a one-shot
///   [`Set`](crate::edit::EditEvent::Set) of the new rating in the [`EditResponse`], in the pass
///   it happened; the value is already set. Hovering previews and reports nothing.
/// - **AccessKit.** A `RadioGroup` named by its label; each star a `RadioButton` named
///   [`RatingLabels::stars`], `checked` for the current rating's.
pub struct Rating<'a> {
    id_salt: IdSalt,
    label: String,
    rating: &'a mut u8,
    size: ControlSize,
    enabled: bool,
    labels: Option<RatingLabels>,
}

impl<'a> Rating<'a> {
    /// A row named `label` editing `rating` (read as at most 5).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, rating: &'a mut u8) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), rating, size: ControlSize::Md, enabled: true, labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: RatingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // lightcraft: crates/ui-egui/src/widgets/rating.rs:129 show
    pub fn show(self, ui: &mut Ui) -> EditResponse<u8> {
        let Self { id_salt, label, rating, size, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| RatingLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let enabled = enabled && ui.is_enabled();
        let (cell, side) = match size {
            ControlSize::Sm => (tokens.component.rating.cell_sm, tokens.component.rating.star_sm),
            ControlSize::Md => (tokens.component.rating.cell_md, tokens.component.rating.star_md),
        };
        let (rtl, forward) = (direction::is_rtl(ui), direction::forward(ui));
        let current = (*rating).min(MAX_STARS);
        let words = Words::of_ctx(&ctx);
        let stars_name = |n: u8| words.counted(&ctx, counts::STARS, &labels.stars, u64::from(n));

        let (_, strip) = ui.allocate_space(vec2(cell * f32::from(MAX_STARS), cell));
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt(id_salt)
                .max_rect(strip)
                .layout(direction::row(ui))
                .accessibility_role(Role::RadioGroup)
                .accessibility_label(label),
        );
        let ids: Vec<_> = (0..MAX_STARS).map(|i| Interactive::id_in(&group, i)).collect();
        let stop = |value: u8| ids[usize::from(value.max(1) - 1)];

        // The row is one Tab stop, on the current rating's star (the first when unrated).
        let mut value = current;
        let focused = enabled && ids.iter().any(|&id| ui.memory(|m| m.has_focus(id)));
        if focused {
            let owner = stop(current);
            ui.memory_mut(|m| m.set_focus_lock_filter(owner, EventFilter { horizontal_arrows: true, ..EventFilter::default() }));
            let (raise, lower) = if rtl { (Key::ArrowLeft, Key::ArrowRight) } else { (Key::ArrowRight, Key::ArrowLeft) };
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, raise) {
                    value = (value + 1).min(MAX_STARS);
                }
                if i.consume_key(Modifiers::NONE, lower) {
                    value = value.saturating_sub(1);
                }
                if i.consume_key(Modifiers::NONE, Key::Home) {
                    value = 0;
                }
                if i.consume_key(Modifiers::NONE, Key::End) {
                    value = MAX_STARS;
                }
                for (n, key) in DIGITS.into_iter().enumerate() {
                    if i.consume_key(Modifiers::NONE, key) {
                        value = n as u8;
                    }
                }
            });
            KeyClaim::claim(&ctx, owner, &DIGITS.map(|key| ClaimedKey::exact(Modifiers::NONE, key)));
        }

        let modifiers = ui.input(|i| i.modifiers);
        let stop_now = stop(value);
        let mut sensed = Vec::with_capacity(ids.len());
        let mut rects = Vec::with_capacity(ids.len());
        for (i, &id) in ids.iter().enumerate() {
            let x = direction::start_x(ui, strip) + forward * cell * i as f32;
            let rect = Rect::from_min_max(egui::pos2(x.min(x + forward * cell), strip.top()), egui::pos2(x.max(x + forward * cell), strip.bottom()));
            let mut star = Interactive::with_id(id, Role::RadioButton, stars_name(i as u8 + 1))
                .position_in_set(i)
                .corner(Corner::Control)
                .state_layer(false)
                .enabled(enabled);
            if id != stop_now {
                star = star.sense(Sense::CLICK);
            }
            sensed.push(star.interact_at(&group, rect));
            rects.push(rect);
        }
        let hovered = sensed.iter().position(|s| s.response().hovered()).filter(|_| enabled);
        if let Some(i) = sensed.iter().position(|s| s.response().clicked()) {
            let n = i as u8 + 1;
            value = if n == current { 0 } else { n };
            ui.memory_mut(|m| m.request_focus(stop(value)));
        } else if value != current && focused {
            ui.memory_mut(|m| m.request_focus(stop(value)));
        }

        let shown = hovered.map_or(value, |i| i as u8 + 1);
        let color = if enabled { tokens.component.rating.star.to_egui() } else { tokens.color.text.disabled.to_egui() };
        let mut responses = Vec::with_capacity(sensed.len());
        for (i, (star, rect)) in sensed.into_iter().zip(&rects).enumerate() {
            let response = star.checked(i as u8 + 1 == value).finish(&group);
            if group.is_rect_visible(*rect) {
                let icon = if (i as u8) < shown { Icon::StarFilled } else { Icon::Star };
                paint_star(group.painter(), *rect, icon, side, color);
            }
            responses.push(response);
        }
        let value_text = if value == 0 { labels.unrated.clone() } else { stars_name(value) };
        ctx.accesskit_node_builder(group.unique_id(), |node| node.set_value(value_text));

        let mut response = responses
            .into_iter()
            .reduce(|all, r| all.union(r))
            .map(|all| all.with_new_rect(strip))
            .unwrap_or_else(|| ui.interact(strip, group.unique_id(), Sense::hover()));
        let event = (value != current).then(|| {
            *rating = value;
            response.mark_changed();
            EditEvent::set(value, modifiers)
        });
        EditResponse::new(response, event)
    }
}

/// Number keys 0 to 5, the ratings.
const DIGITS: [Key; 6] = [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];

/// `icon` `points` wide, centred in `rect`.
fn paint_star(painter: &Painter, rect: Rect, icon: Icon, points: f32, color: Color32) {
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let px = icons::pixels(points, ppp);
    let texture = icons::texture_in(ctx, icon, icons::active(ctx), px, IconSize::Sm.stroke() * ppp);
    let side = px as f32 / ppp;
    let min = egui::emath::GuiRounding::round_to_pixels(rect.center() - vec2(side, side) / 2.0, ppp);
    painter.image(texture, Rect::from_min_size(min, vec2(side, side)), Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), color);
}

/// `rating` filled stars in a row, the first one's left edge at `pos.x` and its middle at `pos.y`,
/// in `color`. Returns the row's width (0 for no stars), so the caller lays out what follows.
// lightcraft: crates/ui-egui/src/widgets/rating.rs:67 paint_rating
pub fn paint_rating(painter: &Painter, pos: Pos2, rating: u8, size: StarSize, color: Color32) -> f32 {
    let rating = rating.min(MAX_STARS);
    for i in 0..rating {
        let center = egui::pos2(pos.x + size.step() * f32::from(i) + size.side() / 2.0, pos.y);
        paint_star(painter, Rect::from_center_size(center, vec2(size.side(), size.side())), Icon::StarFilled, size.side(), color);
    }
    size.step() * f32::from(rating)
}
