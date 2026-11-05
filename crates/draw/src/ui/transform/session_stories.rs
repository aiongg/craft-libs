//! Stories of the Free Transform session. A rectangle
//! stands for the layer over the story canvas, and the session, its overlay and its bar run on it
//! as an app would run them. Sample words, result notes and phase names are story words.
//!
//! - `composites/transform-session/session`: the overlay and the bar over a fake picture: modes,
//!   the scale rule, the reference point shown or hidden, the readout, the interpolation. Apply
//!   and Discard close the session and reopen a box on the applied result.
//! - `patterns/transform-bar/bar`: the bar alone in its phases: Free, the Content-Aware
//!   subset (`BarGroups`, no switch) and Warp with a stub warp group.

use egui::{Shape, Stroke, Ui};
use kurbo::{Rect, Size};

use super::{
    BarGroups, Interpolation, TransformBar, TransformBarOutput, TransformEdit, TransformLabels, TransformNames, TransformOverlay, TransformPhase,
    TransformRequest, TransformSession, TransformSettings,
};
use craft_ui::canvas::RulerUnits;
use craft_ui::canvas::quad::{BoxState, TransformMode};
use craft_ui::controls::Button;
use craft_ui::localized::units::UnitContext;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, lock, shared};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TransformSession", "TransformBar"];

craft_ui::words! {
    /// The status words and the bar's group name of the transform session stories, English by default.
    #[non_exhaustive]
    pub(crate) struct TransformSessionStoryLabels {
        applied: "transformSessionStory.applied" = "Applied";
        closed: "transformSessionStory.closed" = "Closed without changes";
        discarded: "transformSessionStory.discarded" = "Discarded";
        warp: "transformSessionStory.warp" = "Warp";
        box_phase: "transformSessionStory.box" = "Box";
        warp_options: "transformSessionStory.warpOptions" = "Warp options";
        phase_note: "transformSessionStory.phaseNote" = "{phase} · {note}";
    }
}

/// What the app did with the last request.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Note {
    Applied,
    Closed,
    Discarded,
}

impl Note {
    fn text(self, labels: &TransformSessionStoryLabels) -> &str {
        match self {
            Note::Applied => &labels.applied,
            Note::Closed => &labels.closed,
            Note::Discarded => &labels.discarded,
        }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/transform-session/session",
            Level::Composites,
            "TransformSession",
            "Session",
            SessionArgs {
                view: ViewArgs::default(),
                mode: ModeArg::Free,
                rule: RuleArg::Proportional,
                reference_point: true,
                readout: true,
                interpolation: InterpolationArg::Bicubic,
                width: 240.0,
                height: 160.0,
            },
            show_session
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/transform-bar/bar",
            Level::Patterns,
            "TransformBar",
            "Bar",
            BarArgs { phase: PhaseArg::Free, size_menu: true, width: 240.0, height: 160.0 },
            show_bar
        ),
    ]
}

// --- args ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Free,
    Rotate,
    Skew,
    Distort,
    Perspective,
}

impl ModeArg {
    fn mode(self) -> TransformMode {
        match self {
            ModeArg::Free => TransformMode::Free,
            ModeArg::Rotate => TransformMode::Rotate,
            ModeArg::Skew => TransformMode::Skew,
            ModeArg::Distort => TransformMode::Distort,
            ModeArg::Perspective => TransformMode::Perspective,
        }
    }
}

/// Corners keep the proportions and ⇧ frees them, or the legacy rule the other way round.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RuleArg {
    Proportional,
    Free,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum InterpolationArg {
    Bicubic,
    Bilinear,
    #[choice(name = "Nearest Neighbor")]
    NearestNeighbor,
}

impl InterpolationArg {
    fn interpolation(self) -> Interpolation {
        match self {
            InterpolationArg::Bicubic => Interpolation::Bicubic,
            InterpolationArg::Bilinear => Interpolation::Bilinear,
            InterpolationArg::NearestNeighbor => Interpolation::NearestNeighbor,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PhaseArg {
    Free,
    #[choice(name = "Content-Aware")]
    ContentAware,
    Warp,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SessionArgs {
    #[arg(flatten)]
    view: ViewArgs,
    mode: ModeArg,
    #[arg(name = "scale rule")]
    rule: RuleArg,
    #[arg(name = "reference point")]
    reference_point: bool,
    /// Shows the values beside the pointer during a drag.
    readout: bool,
    interpolation: InterpolationArg,
    /// The layer's size in document units.
    #[arg(range = 40.0..=600.0)]
    width: f64,
    #[arg(range = 40.0..=380.0)]
    height: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    phase: PhaseArg,
    /// Gives W and H the unit menu of a secondary click.
    #[arg(name = "size menu")]
    size_menu: bool,
    /// The layer's size in document units.
    #[arg(range = 40.0..=600.0)]
    width: f64,
    #[arg(range = 40.0..=380.0)]
    height: f64,
}

// --- the fake document ---

/// The args a session is built from; a change of one reaches the open session as the edit an app
/// would send.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Snap {
    mode: ModeArg,
    rule: RuleArg,
    reference_point: bool,
    interpolation: InterpolationArg,
    phase: PhaseArg,
    width: f64,
    height: f64,
}

/// What a story keeps: the open session, the persisted section, the box the last Apply left, and
/// the last result.
#[derive(Clone, Debug)]
struct Kept {
    session: TransformSession<()>,
    settings: TransformSettings,
    applied: BoxState,
    note: Option<Note>,
    last: Snap,
}

impl Kept {
    fn new(snap: Snap) -> Self {
        let settings = TransformSettings {
            legacy: snap.rule == RuleArg::Free,
            show_reference_point: snap.reference_point,
            interpolation: snap.interpolation.interpolation(),
            ..TransformSettings::default()
        };
        let applied = BoxState::from_rect(frame(snap));
        let session = open(snap, &settings, applied, 0.0);
        Self { session, settings, applied, note: None, last: snap }
    }

    /// Closes the session and opens a box on what was applied.
    fn reopen(&mut self, now: f64) {
        self.session = open(self.last, &self.settings, self.applied, now);
    }

    /// Sends `edit` to the session and answers what it asks for, as an app does.
    fn apply(&mut self, edit: TransformEdit<()>, now: f64) {
        let request = self.session.apply(edit, now);
        self.answer(request, now);
    }

    /// Ends the session as a widget's ✓ or ⊘ asks.
    fn finish(&mut self, apply: bool, discard: bool, now: f64) {
        let request = if apply {
            Some(self.session.commit())
        } else if discard {
            Some(self.session.discard())
        } else {
            None
        };
        self.answer(request, now);
    }

    fn answer(&mut self, request: Option<TransformRequest<()>>, now: f64) {
        match request {
            Some(TransformRequest::Apply(result)) => {
                self.applied = result.state;
                self.note = Some(Note::Applied);
                self.reopen(now);
            }
            Some(TransformRequest::CloseUntouched) => {
                self.note = Some(Note::Closed);
                self.reopen(now);
            }
            Some(TransformRequest::Discard) => {
                self.note = Some(Note::Discarded);
                self.reopen(now);
            }
            Some(TransformRequest::NeedWarp { .. }) => {
                self.session.apply(TransformEdit::Warp(()), now);
            }
            None => {}
        }
    }

    /// Carries a change of the args into the open session.
    fn sync(&mut self, snap: Snap, now: f64) {
        let last = self.last;
        if (snap.width, snap.height, snap.phase) != (last.width, last.height, last.phase) {
            *self = Kept::new(snap);
            return;
        }
        self.last = snap;
        if snap.mode != last.mode {
            self.apply(TransformEdit::Mode(snap.mode.mode()), now);
        }
        if snap.rule != last.rule {
            self.settings.legacy = snap.rule == RuleArg::Free;
        }
        if snap.reference_point != last.reference_point {
            self.settings.show_reference_point = snap.reference_point;
            self.apply(TransformEdit::ReferencePoint(snap.reference_point), now);
        }
        if snap.interpolation != last.interpolation {
            self.settings.interpolation = snap.interpolation.interpolation();
            self.apply(TransformEdit::Interpolation(snap.interpolation.interpolation()), now);
        }
    }

    /// Ends the pass: the session records its undo step once nothing is held.
    fn settle(&mut self, ui: &Ui) {
        let busy = ui.input(|i| i.pointer.any_down());
        self.session.settle(ui.input(|i| i.time), busy);
    }
}

/// The layer's source frame, centred on the story document.
fn frame(snap: Snap) -> Rect {
    Rect::from_center_size(DOC.center(), Size::new(snap.width, snap.height))
}

/// A session on the layer: the box starts at `state`, in the mode and phase of `snap`.
fn open(snap: Snap, settings: &TransformSettings, state: BoxState, now: f64) -> TransformSession<()> {
    let mut session = TransformSession::with_state(frame(snap), state).mode(snap.mode.mode()).settings(settings);
    if snap.phase == PhaseArg::Warp {
        session.apply(TransformEdit::Warp(()), now);
    }
    session
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// The bar of `kept` in an options bar, with the groups its phase shows; applies its edits.
fn show_options_bar(ui: &mut Ui, kept: &mut Kept, phase: PhaseArg, size_menu: bool, now: f64) {
    let label = TransformNames::of(ui).group;
    let groups = match phase {
        PhaseArg::ContentAware => BarGroups { reference: false, angle: false, skew: false, interpolation: false, ..BarGroups::ALL },
        PhaseArg::Free | PhaseArg::Warp => BarGroups::ALL,
    };
    let warp_label = TransformSessionStoryLabels::of(ui).warp_options.clone();
    let mut out = TransformBarOutput::default();
    {
        let Kept { session, settings, .. } = kept;
        OptionsBar::new("transform-session-options").label(label).show(ui, |slots| {
            let mut bar = TransformBar::new("transform-session-bar", session)
                .groups(groups)
                .mode_switch(phase != PhaseArg::ContentAware)
                .settings(settings)
                .warp_fields(|ui| status(ui, warp_label))
                .output(&mut out);
            if size_menu {
                bar = bar.unit_context(UnitContext::default());
            }
            bar.add_to(slots);
        });
    }
    for edit in out.edits {
        kept.apply(edit, now);
    }
    if out.reset_warp {
        kept.apply(TransformEdit::Warp(()), now);
    }
    kept.finish(out.apply, out.discard, now);
}

// --- session ---

/// The overlay over the fake layer, with the bar below. Apply (Enter or the check) keeps the
/// transformed box; Discard (Esc or the cross) puts back the last applied one.
fn show_session(ui: &mut Ui, args: &mut SessionArgs) {
    let words = StoryLabels::of(ui);
    let story = TransformSessionStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let snap = Snap {
        mode: args.mode,
        rule: args.rule,
        reference_point: args.reference_point,
        interpolation: args.interpolation,
        phase: PhaseArg::Free,
        width: args.width,
        height: args.height,
    };
    let slot = shared(ui, "session", || Kept::new(snap));
    let mut kept = lock(&slot).clone();
    kept.sync(snap, now);

    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let corners = kept.session.quad().map(|p| canvas.view.to_screen(p));
    let accent = tokens.color.accent.solid.to_egui();
    let stroke = Stroke::new(tokens.component.line_guide.width, accent);
    painter.add(Shape::convex_polygon(corners.to_vec(), accent.gamma_multiply(0.5), stroke));
    painter.add(Shape::line_segment([corners[0], corners[2]], stroke));

    let numbers = LocaleNumbers::of(ui).as_ref().clone();
    let pixels = RulerUnits::pixels(numbers);
    let mut overlay = TransformOverlay::new("transform-session-overlay", &kept.session).settings(&kept.settings).units(&pixels, &pixels);
    if args.readout {
        overlay = overlay.readout(TransformLabels::of(ui));
    }
    let shown = overlay.show(ui, &canvas.response, &canvas.view);
    for edit in shown.edits {
        kept.apply(edit, now);
    }
    kept.finish(shown.apply, shown.discard, now);

    show_options_bar(ui, &mut kept, PhaseArg::Free, true, now);
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = Kept::new(snap);
        }
        let phase = if kept.session.phase() == TransformPhase::Warp { &story.warp } else { &story.box_phase };
        status(
            ui,
            kept.note
                .map_or_else(|| phase.clone(), |note| craft_i18n::fmt(&story.phase_note, &[("phase", phase.as_str()), ("note", note.text(&story))])),
        );
    });
    kept.settle(ui);
    *lock(&slot) = kept;
}

// --- bar ---

/// The bar alone over a session nobody draws: the fields edit its box, the check and the select
/// change the session, commit and cancel reopen it.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = StoryLabels::of(ui);
    let story = TransformSessionStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let snap = Snap {
        mode: ModeArg::Free,
        rule: RuleArg::Proportional,
        reference_point: true,
        interpolation: InterpolationArg::Bicubic,
        phase: args.phase,
        width: args.width,
        height: args.height,
    };
    let slot = shared(ui, "bar", || Kept::new(snap));
    let mut kept = lock(&slot).clone();
    kept.sync(snap, now);
    show_options_bar(ui, &mut kept, args.phase, args.size_menu, now);
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = Kept::new(snap);
        }
        if let Some(note) = kept.note {
            status(ui, note.text(&story).to_owned());
        }
    });
    kept.settle(ui);
    *lock(&slot) = kept;
}
