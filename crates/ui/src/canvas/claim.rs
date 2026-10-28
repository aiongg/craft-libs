//! [`Claim`]: which overlay owns the current press on a canvas.

use egui::{Context, Event, Id, InputState, PointerButton, PointerState, Pos2};

/// The overlay that took the current press on a canvas, kept in egui temp memory under the
/// canvas's id until the press is released.
///
/// Overlays sense through the host's canvas `Response`, so one press reaches all of them and the
/// tool. The host calls its overlays before its tool input, in priority order (handles, guides,
/// then the tool); at the press each overlay that wants it calls [`take`](Claim::take), and only
/// the first succeeds. The host skips its tool input while [`owner`](Claim::owner) names an
/// overlay. The claim belongs to one press of the primary button: it holds while that press is
/// down and through the pass of its release, so the owner can finish its gesture there and the
/// tool still doesn't see the release. Other buttons pressed meanwhile and further egui passes of
/// a frame leave it alone; the next primary press ends it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Claim {
    owner: Id,
    press: PrimaryPress,
}

impl Claim {
    fn key(canvas: Id) -> Id {
        canvas.with("craft_ui::canvas::Claim")
    }

    /// The overlay that owns the current press on `canvas`, if one took it.
    pub fn owner(ctx: &Context, canvas: Id) -> Option<Id> {
        let key = Claim::key(canvas);
        let claim = ctx.data(|d| d.get_temp::<Claim>(key))?;
        if claim.press.holds(ctx) {
            Some(claim.owner)
        } else {
            ctx.data_mut(|d| d.remove::<Claim>(key));
            None
        }
    }

    /// Takes the current press on `canvas` for `overlay`. True when no other overlay owns it (and
    /// again for the overlay that does); false while no primary press is in progress.
    pub fn take(ctx: &Context, canvas: Id, overlay: Id) -> bool {
        match Claim::owner(ctx, canvas) {
            Some(owner) => owner == overlay,
            None => match PrimaryPress::current(ctx) {
                Some(press) => {
                    ctx.data_mut(|d| d.insert_temp(Claim::key(canvas), Claim { owner: overlay, press }));
                    true
                }
                None => false,
            },
        }
    }

    /// Whether `overlay` owns the current press on `canvas`.
    pub fn is_owner(ctx: &Context, canvas: Id, overlay: Id) -> bool {
        Claim::owner(ctx, canvas) == Some(overlay)
    }
}

/// The buttons other than the primary one.
const OTHER_BUTTONS: [PointerButton; 4] = [PointerButton::Secondary, PointerButton::Middle, PointerButton::Extra1, PointerButton::Extra2];

/// One press of the primary button: where and when it went down. Overlays keep it with a gesture
/// to tell their press from a later one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PrimaryPress {
    pub origin: Pos2,
    time: f64,
}

impl PrimaryPress {
    /// The primary press whose button-down event is in this pass, the last one if there are
    /// several.
    fn began(i: &InputState) -> Option<PrimaryPress> {
        i.events.iter().rev().find_map(|e| match *e {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, .. } => Some(PrimaryPress { origin: pos, time: i.time }),
            _ => None,
        })
    }

    /// egui keeps one press origin and time for all buttons: they are the primary button's only
    /// while no other button is down.
    fn others_down(p: &PointerState) -> bool {
        OTHER_BUTTONS.iter().any(|&b| p.button_down(b))
    }

    /// The primary press in progress: the one that began in this pass, else the one held down.
    pub fn current(ctx: &Context) -> Option<PrimaryPress> {
        ctx.input(|i| {
            PrimaryPress::began(i).or_else(|| {
                let p = &i.pointer;
                if !p.primary_down() || PrimaryPress::others_down(p) {
                    return None;
                }
                Some(PrimaryPress { origin: p.press_origin()?, time: p.press_start_time()? })
            })
        })
    }

    /// Whether this is still the current press: no other primary press began since, and the
    /// button is down or was released in this pass. A pass without input events (egui's further
    /// passes of a frame) sees no new press.
    pub fn holds(self, ctx: &Context) -> bool {
        ctx.input(|i| match PrimaryPress::began(i) {
            Some(press) => press == self,
            None => {
                let p = &i.pointer;
                // A press that began in a pass nobody looked at.
                let newer = !PrimaryPress::others_down(p) && p.press_start_time().is_some_and(|t| t != self.time);
                !newer && (p.primary_down() || p.primary_released())
            }
        })
    }
}
