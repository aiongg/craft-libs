//! Dialog droppers (F04.02): a dialog's pipette that samples the canvas. One dropper at a time is
//! armed; while it is, the app's canvas samples for it instead of running its tool, and the
//! dropper takes the colour on its next draw.
//!
//! ```no_run
//! use craft_color::ui::{DropperButton, DropperKind, DropperState};
//! # fn dialog(ui: &mut egui::Ui) {
//! // In a Levels dialog: the black-point dropper.
//! if let Some(color) = DropperButton::new("levels-black", "Sample in image to set black point").kind(DropperKind::Black).show(ui).sampled {
//!     // set the black point from `color`
//! }
//! # }
//! # fn canvas(ctx: &egui::Context, color_under_pointer: craft_theme::Srgba) {
//! // In the app's canvas, on a click while a dropper is armed:
//! if DropperState::armed(ctx).is_some() {
//!     DropperState::deliver(ctx, color_under_pointer);
//! }
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: DropperTool is S4C-F13's")]

use craft_theme::{ControlSize, Srgba};
use craft_ui::controls::IconButton;
use craft_ui::icons::Icon;
use egui::{AsIdSalt, Context, Id, IdSalt, Response, Ui};

/// The armed dropper, kept in egui memory: who armed it, the pass it was last drawn in, and a
/// sample waiting for it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Armed {
    owner: Id,
    pass: u64,
    sample: Option<Srgba>,
}

/// The app-wide dropper state, in egui memory. A dropper [`arm`](Self::arm)s itself; the canvas
/// asks whether one is [`armed`](Self::armed) and [`deliver`](Self::deliver)s the colour under a
/// click; the dropper [`take`](Self::take)s it, which disarms it.
///
/// A dropper that is not drawn for a pass (its dialog closed, its popup hidden) is disarmed:
/// [`armed`](Self::armed) returns `None` once the owner has missed a pass, so a closed dialog
/// never leaves the canvas sampling for nobody. Esc on the canvas while armed disarms
/// ([`disarm`](Self::disarm)).
pub struct DropperState;

fn state_key() -> Id {
    Id::unique("craft_color::ui::droppers::armed")
}

/// The armed record, if its owner was drawn in this pass or the one before.
fn record(ctx: &Context) -> Option<Armed> {
    let armed: Option<Armed> = ctx.data(|d| d.get_temp(state_key()));
    let pass = ctx.cumulative_pass_nr();
    armed.filter(|a| a.pass.saturating_add(1) >= pass)
}

impl DropperState {
    /// Arms `owner`'s dropper (the one armed before is disarmed). Called each pass the owner is
    /// drawn and wants samples.
    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:163 `arm`, :175 `refresh_armed`.
    pub fn arm(ctx: &Context, owner: Id) {
        let sample = record(ctx).filter(|a| a.owner == owner).and_then(|a| a.sample);
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| d.insert_temp(state_key(), Armed { owner, pass, sample }));
    }

    /// Disarms `owner`'s dropper, if it is the armed one.
    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:169 `disarm`.
    pub fn disarm(ctx: &Context, owner: Id) {
        if record(ctx).is_some_and(|a| a.owner == owner) {
            ctx.data_mut(|d| d.remove::<Armed>(state_key()));
        }
    }

    /// The armed dropper's owner, if one is armed and was drawn in the last pass.
    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:159 `eyedropper_armed` (and the pass check
    // in `armed_record`).
    pub fn armed(ctx: &Context) -> Option<Id> {
        record(ctx).map(|a| a.owner)
    }

    /// The canvas sampled `color` for the armed dropper; it takes it on its next draw. Nothing
    /// happens when none is armed.
    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:193 `deliver_sample`.
    pub fn deliver(ctx: &Context, color: Srgba) {
        if let Some(armed) = record(ctx) {
            ctx.data_mut(|d| d.insert_temp(state_key(), Armed { sample: Some(color), ..armed }));
            ctx.request_repaint();
        }
    }

    /// The colour delivered for `owner`, once; taking it disarms the dropper unless the owner
    /// [`arm`](Self::arm)s it again (the Color Picker dialog stays armed while it is on top).
    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:185 `take_sample`.
    pub fn take(ctx: &Context, owner: Id) -> Option<Srgba> {
        let armed = record(ctx).filter(|a| a.owner == owner)?;
        let sample = armed.sample?;
        ctx.data_mut(|d| d.remove::<Armed>(state_key()));
        Some(sample)
    }
}

/// Which pipette a [`DropperButton`] shows: Photoshop's plain eyedropper, or the black, grey and
/// white point droppers of Levels and Curves (the pipette's bulb filled with the colour).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DropperKind {
    #[default]
    Sample,
    Black,
    Gray,
    White,
}

impl DropperKind {
    /// The colour of the point dropper's bulb; `None` for the plain pipette.
    fn fill(self) -> Option<Srgba> {
        match self {
            DropperKind::Sample => None,
            DropperKind::Black => Some(Srgba::rgb(0.0, 0.0, 0.0)),
            DropperKind::Gray => Some(Srgba::rgb(0.5, 0.5, 0.5)),
            DropperKind::White => Some(Srgba::rgb(1.0, 1.0, 1.0)),
        }
    }
}

/// What [`DropperButton::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the sample"]
pub struct DropperResponse {
    pub response: Response,
    /// The colour the canvas sampled for this dropper this pass.
    pub sampled: Option<Srgba>,
    /// The dropper is armed after this pass.
    pub armed: bool,
}

/// A dialog's pipette button: a toggle that arms the canvas to sample for it.
///
/// - Click, Enter or Space arms it (it looks pressed) and disarms any other; again disarms it.
///   While it is armed the canvas cursor is the eyedropper, and a click on the canvas delivers
///   the colour under it (averaged over the Eyedropper's [`SampleSize`](super::SampleSize), as
///   Photoshop's dialog droppers do); the button takes it and stays armed for the next click
///   (Levels and Curves sample repeatedly), until the user presses it again, presses Esc on the
///   canvas or the dialog closes.
/// - AccessKit: a `ToggleButton` named by `label`, pressed while armed. The tooltip is `label`.
pub struct DropperButton {
    id_salt: IdSalt,
    label: String,
    kind: DropperKind,
    enabled: bool,
}

impl DropperButton {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>) -> DropperButton {
        DropperButton { id_salt: IdSalt::new(id_salt), label: label.into(), kind: DropperKind::Sample, enabled: true }
    }

    pub fn kind(mut self, kind: DropperKind) -> DropperButton {
        self.kind = kind;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> DropperButton {
        self.enabled = enabled;
        self
    }

    // inv-effectcraft: crates/ui-egui/src/color_picker.rs:163 `arm` and :185 `take_sample` around
    // an `IconButton` (`Icon::Eyedropper`, the Black/Gray/White variants' bulb filled);
    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:289 `sample_at` (the dialog side).
    pub fn show(self, ui: &mut Ui) -> DropperResponse {
        let ctx = ui.ctx().clone();
        let owner = ui.make_persistent_id(self.id_salt);
        let was_armed = DropperState::armed(&ctx) == Some(owner);
        let sampled = DropperState::take(&ctx, owner);
        let mut on = self.enabled && (was_armed || sampled.is_some());
        let mut button =
            IconButton::new(Icon::Eyedropper, self.label.as_str()).size(ControlSize::Sm).toggle(&mut on).enabled(self.enabled).with_id(owner);
        if let Some(fill) = self.kind.fill() {
            button = button.badge(fill);
        }
        let response = ui.add(button);
        if on {
            DropperState::arm(&ctx, owner);
        } else {
            DropperState::disarm(&ctx, owner);
        }
        DropperResponse { response, sampled, armed: on }
    }
}

/// The canvas side: the tool the app hands to its tool host while a dropper is armed. A press
/// on the canvas samples through the app's `sample` function at the document point (`None` off
/// the image or over transparency) and [`deliver`](DropperState::deliver)s it; a drag samples
/// at every move (Photoshop updates the dialog while the button is held); Esc disarms. The
/// cursor is the eyedropper with its tip on the sampled pixel.
// needs: S3-canvas tool_host — `impl CanvasTool for DropperTool` (`press` returns
// `ToolAction::Claimed`, `drag` and `release` sample at `ToolPointer::doc`, `cancel` disarms,
// `cursor` is the eyedropper `ToolCursor`).
pub struct DropperTool<'a> {
    sample: Box<dyn FnMut([f64; 2]) -> Option<Srgba> + 'a>,
}

impl<'a> DropperTool<'a> {
    /// A dropper tool sampling through `sample` (document point to colour).
    pub fn new(sample: impl FnMut([f64; 2]) -> Option<Srgba> + 'a) -> DropperTool<'a> {
        DropperTool { sample: Box::new(sample) }
    }

    /// Samples at document point `doc` and delivers the colour to the armed dropper; what the
    /// tool's press and drag do.
    // pc: crates/ui-egui/src/canvas/cursors.rs:207 `composite_color` (the app's `sample`), then
    // `DropperState::deliver`.
    pub fn sample_at(&mut self, ctx: &Context, doc: [f64; 2]) {
        todo!()
    }
}
