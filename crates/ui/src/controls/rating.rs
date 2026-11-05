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
use crate::theme::{ToEgui, Tokens, TokensOf as _, UiTokens};

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

/// The stars' sizes and colours at one size, from [`Rating::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RatingStyle {
    /// One star's cell: `rating.cell` at the size.
    pub cell: f32,
    /// A star's side in the control: `rating.star_size` at the size.
    pub star: f32,
    /// A star's side on a badge ([`paint_rating`]): `rating.badge_star` at the size.
    pub badge_star: f32,
    /// Between the left edges of badge stars: `rating.badge_step` at the size.
    pub badge_step: f32,
    /// Lit stars: `rating.star`.
    pub lit: Color32,
    /// Unlit stars: `color.text.muted`.
    pub unlit: Color32,
    /// Both kinds of stars in a disabled rating: `color.text.disabled`.
    pub disabled: Color32,
}

/// A row of five stars, after LightCraft's `Rating`.
///
/// ```ignore
/// if let Some(EditEvent::Set { value, .. }) = Rating::new("rating", "Rating", &mut photo.rating).show(ui).event { app.rate(value) }
/// ```
///
/// - **Look.** Five [`Icon::Star`](crate::icons::Icon::Star) outlines in the muted text colour
///   (LightCraft's icon colour), the first `rating` of them
///   [`Icon::StarFilled`](crate::icons::Icon::StarFilled) in `rating.star`, the text colour
///   (LightCraft's near-text star), on cells of `rating.cell` with stars of `rating.star`'s sizes.
///   In right-to-left the first star is on the right.
/// - **Pointer.** Hovering star `n` previews `n` filled stars and fills the hovered cell through
///   the state layer; a click sets `n`, and a click on the current rating's star clears it to 0
///   (LightCraft's `clicked_rating`).
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

    /// Sets the control size (default [`ControlSize::Md`]).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the control takes input (default `true`).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The stars' sizes and colours at `size`.
    pub fn style(tokens: &UiTokens, size: ControlSize) -> RatingStyle {
        let rating = &tokens.component.rating;
        RatingStyle {
            cell: rating.cell.get(size),
            star: rating.star_size.get(size),
            badge_star: rating.badge_star.get(size),
            badge_step: rating.badge_step.get(size),
            lit: rating.star.to_egui(),
            unlit: tokens.color.text.muted.to_egui(),
            disabled: tokens.color.text.disabled.to_egui(),
        }
    }

    // lightcraft: crates/ui-egui/src/widgets/rating.rs:129 show
    /// Draws the stars and reports the edited rating.
    pub fn show(self, ui: &mut Ui) -> EditResponse<u8> {
        let Self { id_salt, label, rating, size, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| RatingLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let enabled = enabled && ui.is_enabled();
        let style = Self::style(&tokens, size);
        let (cell, side) = (style.cell, style.star);
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
        let focus = ids.iter().copied().find(|&id| enabled && ui.memory(|m| m.has_focus(id)));
        if let Some(owner) = focus {
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
            let mut star =
                Interactive::with_id(id, Role::RadioButton, stars_name(i as u8 + 1)).position_in_set(i).corner(Corner::Control).enabled(enabled);
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
        } else if focus.is_some_and(|id| id != stop(value)) {
            ui.memory_mut(|m| m.request_focus(stop(value)));
        }

        let shown = hovered.map_or(value, |i| i as u8 + 1);
        let (lit, unlit) = if enabled { (style.lit, style.unlit) } else { (style.disabled, style.disabled) };
        let mut responses = Vec::with_capacity(sensed.len());
        for (i, (star, rect)) in sensed.into_iter().zip(&rects).enumerate() {
            let response = star.checked(i as u8 + 1 == value).finish(&group);
            if group.is_rect_visible(*rect) {
                let (icon, color) = if (i as u8) < shown { (Icon::StarFilled, lit) } else { (Icon::Star, unlit) };
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
/// in `color`, with the badge sizes of `tokens` for `size`. Returns the row's width (0 for no
/// stars), so the caller lays out what follows.
// lightcraft: crates/ui-egui/src/widgets/rating.rs:67 paint_rating
pub fn paint_rating(painter: &Painter, tokens: &UiTokens, pos: Pos2, rating: u8, size: ControlSize, color: Color32) -> f32 {
    let rating = rating.min(MAX_STARS);
    let style = Rating::style(tokens, size);
    let (side, step) = (style.badge_star, style.badge_step);
    for i in 0..rating {
        let center = egui::pos2(pos.x + step * f32::from(i) + side / 2.0, pos.y);
        paint_star(painter, Rect::from_center_size(center, vec2(side, side)), Icon::StarFilled, side, color);
    }
    step * f32::from(rating)
}
