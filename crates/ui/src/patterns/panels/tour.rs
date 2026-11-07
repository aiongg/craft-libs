//! `Tour` (24.4): a guided walk through the interface, one step at a time, a card beside the part it explains.

use super::words::PanelWords;
use crate::buttons::Button;
use crate::contract::overlay::{Anchor, FloatPlace, OverlayFit, Place, ViewportShare};
use crate::contract::{Bind, Overlay, Pattern, Props, Show as _, ShowContainer as _, StopProps as _};
use crate::display::Text;
use crate::layout_components::{Row, Stack};
use crate::notices::{Notice, NoticeLayout};
use crate::popover::Popover;
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::Initial;
use crate::runtime::overlay::{Backdrop, CloseReason, Rank};
use craft_behavior::overlay::{Align, Side};
use craft_behavior::{EditEvent, GestureId, Mixed};
use craft_theme::variant::Tone;
use egui::{Rect, Ui};
use std::sync::Arc;

#[cfg(feature = "testing")]
mod stories;

/// The card's widest share of the window.
const CARD_SHARE: ViewportShare = ViewportShare::new(0.3, 0.6);

/// One step of a tour: what it says and the part of the window it points at.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TourStep {
    /// The part the step points at, in window coordinates. The scrim leaves it clear and the card sits beside it; a
    /// step without one shows its card in the middle of the window.
    pub target: Option<Rect>,
    /// The step's heading.
    pub title: Label,
    /// The step's text.
    pub text: Label,
}

impl TourStep {
    /// A step titled `title` saying `text`, pointing at nothing.
    pub fn new(title: impl Into<Label>, text: impl Into<Label>) -> Self {
        TourStep { target: None, title: title.into(), text: text.into() }
    }

    /// The part of the window the step points at.
    #[must_use]
    pub fn target(mut self, target: Rect) -> Self {
        self.target = Some(target);
        self
    }
}

/// What the user did to a tour in this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TourEvent {
    /// Went on to the next step; the index binding holds it.
    Next,
    /// Went back a step; the index binding holds it.
    Back,
    /// Finished the last step. The app stops showing the tour.
    Finished,
    /// Left the tour before the end (Skip or Escape). The app stops showing the tour.
    Skipped,
}

/// What a tour reports: what the user did in this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TourOutput {
    /// What the user did, if anything.
    pub event: Option<TourEvent>,
}

/// A guided tour of the interface: a card with the step's heading and text, Skip, Back and Next (Done on the last
/// step) and the step count, beside the part the step points at, with the rest of the window dimmed.
///
/// - Show it while the tour runs; the app stops showing it on `Finished` and `Skipped`.
/// - The index binding holds the step; Next and Back change it and report the event. An index past the last step
///   shows the last step.
/// - Keys: Next has the focus when the step opens, so Enter goes on; Escape skips; Tab reaches Back and Skip.
#[derive(Pattern, Props)]
pub struct Tour<'a> {
    label: Label,
    steps: &'a [TourStep],
    index: Bind<'a, usize>,
}

impl<'a> Tour<'a> {
    /// A tour named `label` over `steps`, at the step `index`.
    pub fn new(label: impl Into<Label>, steps: &'a [TourStep], index: Bind<'a, usize>) -> Self {
        Tour { label: label.into(), steps, index }
    }

    /// Shows the current step; a tour with no steps shows nothing.
    pub fn show(self, ui: &mut Ui) -> TourOutput {
        let Tour { label, steps, mut index } = self;
        let Some(last) = steps.len().checked_sub(1) else { return TourOutput::default() };
        let held = index.reborrow().get().and_then(|held| held.same().copied()).unwrap_or(0);
        let at = held.min(last);
        let step = &steps[at];
        let env = Env::of(ui);
        let words = env.words::<PanelWords>();
        let count = words.tour_count.fill(&[("n", &env.format().integer(at as u64 + 1)), ("total", &env.format().integer(steps.len() as u64))]);
        let layer = layer(step.target);
        let mut clicked = (false, false, false);
        let shown = Popover::new(step.title.clone(), Bind::given(true)).overlay(|_| layer).initial_focus(Initial::Default).close_on_enter(true).show(
            ui,
            |ui, _| {
                Stack::new().show(ui, |ui, _| {
                    Notice::new(Tone::Neutral, step.text.clone()).title(step.title.clone()).layout(NoticeLayout::Inline).show(ui);
                    Text::new(count.clone()).show(ui);
                    Row::new().show(ui, |ui, _| {
                        let skip = if at < last { Some(Button::new(words.tour_skip.clone()).show(ui)) } else { None };
                        let back = if at > 0 { Some(Button::new(words.tour_back.clone()).show(ui)) } else { None };
                        let next = if at < last { words.tour_next.clone() } else { words.tour_done.clone() };
                        let next = Button::new(next).default_stop().show(ui);
                        clicked = (
                            skip.is_some_and(|skip| crate::contract::clicked(&skip)),
                            back.is_some_and(|back| crate::contract::clicked(&back)),
                            crate::contract::clicked(&next),
                        );
                    });
                });
            },
        );
        let (skip, back, next) = clicked;
        let forward = if at < last { TourEvent::Next } else { TourEvent::Finished };
        let event = if next {
            Some(forward)
        } else if back {
            Some(TourEvent::Back)
        } else if skip {
            Some(TourEvent::Skipped)
        } else {
            match shown.output.closed {
                Some(CloseReason::Enter) => Some(forward),
                Some(_) => Some(TourEvent::Skipped),
                None => None,
            }
        };
        let moved = match event {
            Some(TourEvent::Next) => Some(at + 1),
            Some(TourEvent::Back) => Some(at - 1),
            _ => None,
        };
        if let Some(to) = moved {
            let name: Arc<str> = Arc::from(label.as_str());
            let _ = index.reborrow().apply(&EditEvent::set(GestureId::default(), Mixed::Same(held), to, name, None));
        }
        TourOutput { event }
    }
}

/// The card's layer: modal, with the scrim leaving `target` clear and the card beside it, else a plain scrim and the
/// card in the middle.
fn layer(target: Option<Rect>) -> Overlay {
    let mut overlay = Overlay::modal();
    overlay.fit = OverlayFit::ContentCapped(CARD_SHARE);
    match target {
        Some(target) => {
            overlay.rank = Rank::modal(Backdrop::ScrimWithHole(target));
            overlay.place = Place::anchored(Anchor::Rect(target), Side::End, Align::Center);
        }
        None => {
            overlay.rank = Rank::modal(Backdrop::Scrim);
            overlay.place = Place::Float(FloatPlace::centred());
        }
    }
    overlay
}
