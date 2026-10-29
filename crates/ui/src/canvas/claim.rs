//! [`Claim`]: which overlay owns the current press on a canvas.

use egui::{Context, Event, Id, InputState, PointerButton, PointerState, Pos2};

use crate::edit::order::{self, Turn};

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
///
/// A press that arrives while another control's typing run has the focus waits for the run's
/// ending event (`edit::order`): [`take`](Claim::take) registers it with the order queue and the
/// owner asks [`turn`](Claim::turn) in every pass until its press runs. The claim holds through
/// the wait, also past the release, so a press released while it waited still runs, as a click.
///
/// ```ignore
/// // The owner's pass, after it took the press:
/// match Claim::turn(ctx, canvas.id, id) {
///     Turn::Waits => return, // keep the press; begin nothing, change nothing
///     Turn::Runs => start = *self.state, // the host's value now, not the one at the press
///     Turn::Now => {}
/// }
/// // ... begin the gesture from `Claim::owned_press` ...
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Claim {
    owner: Id,
    press: PrimaryPress,
    /// The press waits for its turn; the pass the owner last asked.
    waiting: Option<u64>,
}

impl Claim {
    fn key(canvas: Id) -> Id {
        canvas.with("craft_ui::canvas::Claim")
    }

    /// The claim on `canvas` while it lasts: its press is down or released in this pass, or it
    /// waits for its turn and its owner asked in the last pass.
    fn current(ctx: &Context, canvas: Id) -> Option<Claim> {
        let key = Claim::key(canvas);
        let claim = ctx.data(|d| d.get_temp::<Claim>(key))?;
        let pass = ctx.cumulative_pass_nr();
        if claim.press.holds(ctx) || (claim.waiting.is_some_and(|polled| polled + 1 >= pass) && !claim.press.superseded(ctx)) {
            Some(claim)
        } else {
            ctx.data_mut(|d| d.remove::<Claim>(key));
            None
        }
    }

    /// The overlay that owns the current press on `canvas`, if one took it.
    pub fn owner(ctx: &Context, canvas: Id) -> Option<Id> {
        Claim::current(ctx, canvas).map(|claim| claim.owner)
    }

    /// Takes the current press on `canvas` for `overlay`. True when no other overlay owns it (and
    /// again for the overlay that does); false while no primary press is in progress. The press
    /// is registered with the order queue: when another control's typing run has the focus the
    /// run is asked to end and the owner's [`turn`](Claim::turn) waits for its ending event.
    pub fn take(ctx: &Context, canvas: Id, overlay: Id) -> bool {
        match Claim::owner(ctx, canvas) {
            Some(owner) => owner == overlay,
            None => match PrimaryPress::current(ctx) {
                Some(press) => {
                    let waiting = (order::turn(ctx, overlay, None) == Turn::Waits).then(|| ctx.cumulative_pass_nr());
                    ctx.data_mut(|d| d.insert_temp(Claim::key(canvas), Claim { owner: overlay, press, waiting }));
                    true
                }
                None => false,
            },
        }
    }

    /// When the press `overlay` took on `canvas` runs. Ask in every pass while the press waits:
    /// the owner begins its gesture on [`Now`](Turn::Now) (the press's own pass, or no wait) or
    /// [`Runs`](Turn::Runs) (once, in the pass its turn comes; values it took at the press are
    /// the host's value then), and does nothing on [`Waits`](Turn::Waits). `Now` also when
    /// `overlay` owns no claim.
    pub(crate) fn turn(ctx: &Context, canvas: Id, overlay: Id) -> Turn {
        let Some(mut claim) = Claim::current(ctx, canvas).filter(|c| c.owner == overlay && c.waiting.is_some()) else {
            return Turn::Now;
        };
        let turn = order::turn(ctx, overlay, None);
        claim.waiting = (turn == Turn::Waits).then(|| ctx.cumulative_pass_nr());
        ctx.data_mut(|d| d.insert_temp(Claim::key(canvas), claim));
        turn
    }

    /// The current press on `canvas` when `overlay` owns it.
    pub(crate) fn owned_press(ctx: &Context, canvas: Id, overlay: Id) -> Option<PrimaryPress> {
        Claim::current(ctx, canvas).filter(|c| c.owner == overlay).map(|c| c.press)
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

    /// Whether a newer primary press has begun.
    fn superseded(self, ctx: &Context) -> bool {
        ctx.input(|i| match PrimaryPress::began(i) {
            Some(press) => press != self,
            None => !PrimaryPress::others_down(&i.pointer) && i.pointer.press_start_time().is_some_and(|t| t != self.time),
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
