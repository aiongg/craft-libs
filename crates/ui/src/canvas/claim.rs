//! [`Claim`]: which overlay owns the current press on a canvas.

use egui::{Context, Id};

/// The overlay that took the current press on a canvas, kept in egui temp memory under the
/// canvas's id until the press is released.
///
/// Overlays sense through the host's canvas `Response`, so one press reaches all of them and the
/// tool. The host calls its overlays before its tool input, in priority order (handles, guides,
/// then the tool); at the press each overlay that wants it calls [`take`](Claim::take), and only
/// the first succeeds. The host skips its tool input while [`owner`](Claim::owner) names an
/// overlay. The claim holds through the pass of the release, so the owner can finish its gesture
/// there and the tool still doesn't see the release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    owner: Id,
    /// The pass the press was taken in.
    pass: u64,
}

impl Claim {
    fn key(canvas: Id) -> Id {
        canvas.with("craft_ui::canvas::Claim")
    }

    /// Whether a claim made in pass `pass` still holds: the same pass, or a later one in which the
    /// press is held or released and no new press began.
    fn holds(ctx: &Context, pass: u64) -> bool {
        if pass == ctx.cumulative_pass_nr() {
            return true;
        }
        ctx.input(|i| !i.pointer.any_pressed() && (i.pointer.any_down() || i.pointer.any_released()))
    }

    /// The overlay that owns the current press on `canvas`, if one took it.
    pub fn owner(ctx: &Context, canvas: Id) -> Option<Id> {
        let key = Claim::key(canvas);
        let claim = ctx.data(|d| d.get_temp::<Claim>(key))?;
        if Claim::holds(ctx, claim.pass) {
            Some(claim.owner)
        } else {
            ctx.data_mut(|d| d.remove::<Claim>(key));
            None
        }
    }

    /// Takes the current press on `canvas` for `overlay`. True when no other overlay owns it (and
    /// again for the overlay that does); false while no press is in progress.
    pub fn take(ctx: &Context, canvas: Id, overlay: Id) -> bool {
        match Claim::owner(ctx, canvas) {
            Some(owner) => owner == overlay,
            None => {
                let pressing = ctx.input(|i| i.pointer.any_pressed() || i.pointer.any_down() || i.pointer.any_released());
                if pressing {
                    let claim = Claim { owner: overlay, pass: ctx.cumulative_pass_nr() };
                    ctx.data_mut(|d| d.insert_temp(Claim::key(canvas), claim));
                }
                pressing
            }
        }
    }

    /// Whether `overlay` owns the current press on `canvas`.
    pub fn is_owner(ctx: &Context, canvas: Id, overlay: Id) -> bool {
        Claim::owner(ctx, canvas) == Some(overlay)
    }
}
