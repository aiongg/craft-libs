//! What opens a free-standing layer: triggers, openers, and the anchor and least width an opener gives.

use super::place::{Anchor, Place};
use super::props::Overlay;
use crate::contract::Response;
use crate::primitives::Id;
use craft_ui_rt::Rect;

/// How a free-standing popup's trigger fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum Trigger {
    /// A click on the opener toggles. A root opener (`Overlay::opened_by`) toggles on the primary press inside it, as
    /// pop-up buttons and dropdowns do, so a press held past the click time still opens it; the release that
    /// completes the click is part of that opening.
    Click,
    /// A primary press on the opener toggles, and a secondary click inside it toggles too, hanging the layer from the
    /// opener as a press does (a pop-up menu button). Keyboard and accessibility activation toggle it too.
    Press,
    /// A context request (right click, the Menu key, AccessKit `ShowContextMenu`) opens.
    Context,
    /// The pointer resting on the opener opens, after the tooltip delay.
    Hover,
    /// A press held for `HOLD` seconds opens.
    Hold,
    /// Only `PopupHandle::open` and the binding open it.
    Manual,
}

/// A component that opens a free-standing popup, and how. A layer with several openers keeps the first as its
/// `Overlay.opener`, which keys the layer's open state, and the others in `others`.
#[derive(Clone, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct Opener {
    /// The opener's id.
    pub id: Id,
    /// The opener's rect in this frame, in window coordinates.
    pub rect: Rect,
    /// The trigger.
    pub trigger: Trigger,
    /// Whether the opener was enabled in the run that recorded it; a disabled opener closes the layer.
    pub enabled: bool,
    /// Where the layer hangs when this opener opened it. `None` hangs it at the opener's rect, or, for a
    /// `Trigger::Context` opener, at the pointer a secondary click opened it at.
    pub anchor: Option<Anchor>,
    /// The other openers of the same layer.
    pub others: Vec<Opener>,
    /// Whether the opener's run fired `trigger`. A container with no `open` binding applies it to the layer's open
    /// slot.
    pub(crate) fired: bool,
    /// How the opener fired, for where the layer hangs.
    pub(crate) opening: Opening,
    /// The opener is pressed with the primary button, for `Trigger::Hold`.
    pub(crate) pressed: bool,
    /// A `Trigger::Click` opener's primary click completed in this run: the release of the press that opened the
    /// layer, whose activation is that opening and no pick.
    pub(crate) clicked: bool,
}

impl Opener {
    pub(super) fn new(anchor: &Response, trigger: Trigger) -> Opener {
        // The opener's run decided its facts: a primary press that went down on it, a secondary click released on
        // it, a context request by key or assistive technology, a keyboard or assistive click.
        let facts = anchor.input();
        let primary = facts.pressed;
        let by_pointer = facts.secondary_clicked;
        let fired = match trigger {
            // A root opener opens on the primary press inside it, as a pop-up button does, however long the press is
            // then held; a `Click` opener's release completes the click that press began.
            Trigger::Click => primary,
            Trigger::Press => primary || by_pointer,
            Trigger::Context => by_pointer || facts.context,
            Trigger::Hover | Trigger::Hold | Trigger::Manual => false,
        };
        // Keyboard and accessibility clicks have no pointer press; their activation must still open the layer.
        let fired = fired || (facts.activated && matches!(trigger, Trigger::Click | Trigger::Press));
        // A secondary click hangs a context layer at the pointer it was released at.
        let opening = match (trigger, facts.pointer) {
            (Trigger::Context, Some(at)) if by_pointer => Opening::At(Rect::from_min_size(at, craft_ui_rt::Vec2::ZERO)),
            _ => Opening::Plain,
        };
        // Only a hold reads whether the press is held: the read makes the reading scope run when it flips.
        let pressed = trigger == Trigger::Hold && anchor.is_pointer_button_down_on();
        let clicked = trigger == Trigger::Click && facts.clicked && !facts.activated;
        Opener {
            id: anchor.id,
            rect: anchor.rect,
            trigger,
            enabled: !facts.disabled,
            anchor: None,
            others: Vec::new(),
            fired,
            opening,
            pressed,
            clicked,
        }
    }

    /// This opener and the others, first first.
    pub(super) fn all(&self) -> impl Iterator<Item = &Opener> {
        std::iter::once(self).chain(&self.others)
    }

    /// The opener the layer's open slot says opened it (`by`: its id, trigger and opening), else the first with its
    /// own opening.
    pub(super) fn active(&self, by: Option<(Id, Trigger, Opening)>) -> (&Opener, Opening) {
        let found = by.and_then(|(id, trigger, opening)| {
            self.all().find(|opener| opener.id == id && opener.trigger == trigger).map(|opener| (opener, opening))
        });
        found.unwrap_or((self, self.opening))
    }

    /// Whether an opener's run fired its trigger or completed the click that opened the layer: that activation opened
    /// the layer and closes no layer chain it sits in ([`claim_activation`](super::dismissal::claim_activation)).
    pub(crate) fn claims_activation(&self) -> bool {
        self.all().any(|opener| opener.fired || opener.clicked)
    }
}

/// How an opener opened its layer: what decides where a `Trigger::Context` layer hangs.
#[derive(Clone, Copy, Debug, Default, PartialEq, crate::contract::MemoProps)]
pub(crate) enum Opening {
    /// A press, click, hold, key or the binding: the layer hangs from the opener.
    #[default]
    Plain,
    /// A secondary click at this point (an empty rect), in window coordinates: the layer hangs there.
    At(Rect),
}

/// The least width a layer asks of its content: its `min_width` rule applied to the width of the opener it hangs
/// from, else of the rect its place hangs it from; 0 for an anchor that is not a rect.
pub(crate) fn anchor_floor(overlay: &Overlay) -> f32 {
    let width = match &overlay.opener {
        Some(opener) => match &opener.anchor {
            Some(Anchor::Rect(rect)) => rect.width(),
            Some(Anchor::Response) | None => opener.rect.width(),
            Some(_) => return 0.0,
        },
        None => match &overlay.place {
            Place::Anchored { anchor: Anchor::Rect(rect), .. } => rect.width(),
            _ => return 0.0,
        },
    };
    overlay.least_width(width)
}

/// The least width of the layer the content runs in ([`anchor_floor`]), which a layer's container gives its regions:
/// a content-sized collection in the layer is at least this wide, so its rows fill the layer. 0 outside a layer, and
/// inside a row or grid, whose children share the layer's width.
#[derive(Clone, Copy, Debug, Default, PartialEq, crate::contract::MemoProps)]
pub(crate) struct LayerFloor(pub(crate) f32);

impl crate::runtime::env::EnvValue for LayerFloor {
    fn root() -> Self {
        LayerFloor(0.0)
    }
}

/// The anchor a layer opened by `opener` hangs from: the opener's own, else the layer's `Pointer` or `Corner`
/// place, else the point a secondary click opened it at, else the opener's rect.
pub(crate) fn opener_anchor(opener: &Opener, opening: Opening, place: &Place) -> Anchor {
    if let Some(anchor) = &opener.anchor {
        return anchor.clone();
    }
    match (place, opening) {
        (Place::Anchored { anchor: anchor @ (Anchor::Pointer | Anchor::Corner(..) | Anchor::Parent), .. }, _) => anchor.clone(),
        (_, Opening::At(rect)) => Anchor::Rect(rect),
        (_, Opening::Plain) => Anchor::Response,
    }
}
