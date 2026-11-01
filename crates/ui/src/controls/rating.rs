#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`Rating`]: one to five stars the user clicks, and [`paint_rating`] for stars drawn on
//! thumbnails and captions.

use egui::{AsIdSalt, Color32, IdSalt, Painter, Pos2, Ui};

use super::ControlSize;
use crate::edit::EditResponse;

crate::words! {
    /// The rating's words, English by default.
    #[non_exhaustive]
    pub struct RatingLabels {
        /// The row's accessible name.
        rating: "rating.rating" = "Rating";
        /// A star's accessible name: `{n}` its rating. A
        /// [`Counted`](crate::localized::words::Counted) word ("{n} star", "{n} stars"),
        /// written with [`Words::counted`](crate::localized::Words::counted); S5C-F9 adds it to
        /// the kernel's `counts` and [`COUNTS`](crate::localized::words::COUNTS).
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
///   [`Icon::StarFilled`](crate::icons::Icon::StarFilled), in `rating.star` (proposed; LightCraft
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
        todo!()
    }
}

/// `rating` filled stars in a row, the first one's left edge at `pos.x` and its middle at `pos.y`,
/// in `color`. Returns the row's width (0 for no stars), so the caller lays out what follows.
// lightcraft: crates/ui-egui/src/widgets/rating.rs:67 paint_rating
pub fn paint_rating(painter: &Painter, pos: Pos2, rating: u8, size: StarSize, color: Color32) -> f32 {
    todo!()
}
