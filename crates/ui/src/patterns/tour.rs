//! [`Tour`]: coach marks that walk the user through the interface, one step at a time, each
//! pointing at a part of the window.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context, Id, Rect};

/// One step: what it points at and what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct TourStep {
    /// The automation id of the step (`layers-panel`).
    pub id: String,
    pub title: String,
    pub body: String,
    /// The rect of the part it points at this frame; `None` shows the card centred with no
    /// highlight (a welcome or closing step).
    pub target: Option<Rect>,
}

/// What the user did with the tour this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TourEvent {
    Next,
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

impl Tour {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        todo!()
    }

    pub fn labels(mut self, labels: TourLabels) -> Self {
        todo!()
    }

    /// Shows step `index` of `steps` on the foreground layer; returns the user's move this frame.
    // effectcraft: crates/ui-egui/src/panels/learn.rs:80 `coach` (card bottom right, target pulse
    // `sin(time * 4)`, 2 pt stroke, repaint every 33 ms, hidden under the Home screen).
    pub fn show(self, ctx: &Context, steps: &[TourStep], index: usize) -> Option<TourEvent> {
        todo!()
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
