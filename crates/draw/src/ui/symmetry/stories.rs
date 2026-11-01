//! Stories of painting symmetry (task ST-TO2; task RG2 registers them). A fake page stands for the
//! document, and the axes, the Transform Symmetry session and the menu run on it as an app would
//! run them. Sample words and the status lines are story words.
//!
//! - `primitives/symmetry/modes`: the four modes at the page centre and off-centre, with the
//!   handles of Transform Symmetry on request; the axes clip to the page.
//! - `primitives/symmetry/transform`: Transform Symmetry in a `ToolHost`: move the centre, turn the
//!   axes (⇧ in 15° steps), Enter and Esc, the keyboard parts. Commit stores the symmetry and
//!   reopens the session on it; Cancel reopens it on the stored one.
//! - `patterns/symmetry-menu/menu`: the menu with paths, Off, and Transform Symmetry disabled and
//!   enabled.

use egui::Ui;
use kurbo::Vec2;

use super::{
    Symmetry, SymmetryChoice, SymmetryCommand, SymmetryEvent, SymmetryInput, SymmetryLabels, SymmetryMenu, SymmetryMode, SymmetryOverlay,
    SymmetryPathRow, SymmetryStyle, SymmetryTransform,
};
use craft_ui::canvas::{RulerUnits, ToolHost};
use craft_ui::controls::Button;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Symmetry", "SymmetryOverlay", "SymmetryTransform", "SymmetryMenu"];

craft_ui::words! {
    /// The status lines and sample path names of the symmetry stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SymmetryStoryLabels {
        axes: "symmetryStory.axes" = "{n} axes on the page";
        centre: "symmetryStory.centre" = "Centre {x}, {y} · {angle}°";
        stored: "symmetryStory.stored" = "Stored";
        cancelled: "symmetryStory.cancelled" = "Cancelled";
        wave: "symmetryStory.wave" = "Wave";
        star: "symmetryStory.star" = "Star";
    }
}

/// How the last session ended.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ended {
    Stored,
    Cancelled,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/symmetry/modes",
            Level::Primitives,
            "Symmetry",
            "Modes",
            ModesArgs { view: ViewArgs::default(), mode: ModeArg::Vertical, x: 0.0, y: 0.0, angle: 0.0, handles: false },
            show_modes
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/symmetry/transform",
            Level::Primitives,
            "SymmetryTransform",
            "Transform",
            TransformArgs { view: ViewArgs::default(), mode: ModeArg::Vertical, x: 0.0, y: 0.0, angle: 0.0 },
            show_transform
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/symmetry-menu/menu",
            Level::Patterns,
            "SymmetryMenu",
            "Menu",
            MenuArgs { choice: ChoiceArg::Off, paths: true, can_transform: true },
            show_menu
        ),
    ]
}

// --- shared parts ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Vertical,
    Horizontal,
    #[choice(name = "Dual Axis")]
    Dual,
    Diagonal,
}

impl ModeArg {
    fn mode(self) -> SymmetryMode {
        match self {
            ModeArg::Vertical => SymmetryMode::Vertical,
            ModeArg::Horizontal => SymmetryMode::Horizontal,
            ModeArg::Dual => SymmetryMode::Dual,
            ModeArg::Diagonal => SymmetryMode::Diagonal,
        }
    }
}

/// The symmetry the args describe: the centre is the page centre moved by `x` and `y`.
fn symmetry(mode: ModeArg, x: f64, y: f64, rotation: f64) -> Option<Symmetry> {
    Symmetry::new(mode.mode(), DOC.center() + Vec2::new(x, y), rotation)
}

/// Remembers the story's state under a name of this module's id space.
fn state_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_draw::symmetry::stories", name))
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

// --- modes ---

#[derive(StoryArgs, Debug, PartialEq)]
struct ModesArgs {
    #[arg(flatten)]
    view: ViewArgs,
    mode: ModeArg,
    /// The centre's offset from the page centre, in document units.
    #[arg(range = -500.0..=500.0)]
    x: f64,
    #[arg(range = -400.0..=400.0)]
    y: f64,
    /// Degrees clockwise.
    #[arg(range = 0.0..=360.0)]
    angle: f64,
    /// Shows the centre and the rotation handle, as while transforming.
    handles: bool,
}

/// The axes over the page, clipped to it.
fn show_modes(ui: &mut Ui, args: &mut ModesArgs) {
    let words = StoryLabels::of(ui);
    let own = SymmetryStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let Some(symmetry) = symmetry(args.mode, args.x, args.y, args.angle) else {
        return;
    };
    let style = SymmetryStyle::of(&Tokens::of(ui));
    SymmetryOverlay::new(&symmetry, DOC.size()).editing(args.handles).paint(&ui.painter_at(canvas.view.rect), &canvas.view, &style);
    status(ui, craft_i18n::fmt(&own.axes, &[("n", &symmetry.segments(DOC.size()).len().to_string())]));
}

// --- transform ---

#[derive(StoryArgs, Debug, PartialEq)]
struct TransformArgs {
    #[arg(flatten)]
    view: ViewArgs,
    mode: ModeArg,
    /// The centre's offset from the page centre, in document units.
    #[arg(range = -500.0..=500.0)]
    x: f64,
    #[arg(range = -400.0..=400.0)]
    y: f64,
    /// Degrees clockwise.
    #[arg(range = 0.0..=360.0)]
    angle: f64,
}

/// What the transform story keeps: the open session, the stored symmetry, the last result and the
/// args the symmetry was made from.
#[derive(Clone, Debug)]
struct Kept {
    session: SymmetryTransform,
    stored: Symmetry,
    note: Option<Ended>,
    seed: [f64; 4],
}

impl Kept {
    fn new(stored: Symmetry, seed: [f64; 4]) -> Self {
        Self { session: SymmetryTransform::begin(stored), stored, note: None, seed }
    }

    /// Answers an event as the app does: Commit stores the symmetry, and either end drops the
    /// session, which the story reopens on the stored symmetry.
    fn apply(&mut self, event: SymmetryEvent) {
        match event {
            SymmetryEvent::Preview(_) => return,
            SymmetryEvent::Commit(symmetry) => {
                self.stored = symmetry;
                self.note = Some(Ended::Stored);
            }
            SymmetryEvent::Cancel => self.note = Some(Ended::Cancelled),
        }
        self.session = SymmetryTransform::begin(self.stored);
    }
}

/// The session over the page, with its bar (title, ✓ and ⊘) above the status line. Click the page
/// to give it the keyboard: Tab picks a part, the arrows step it, Enter commits and Esc cancels.
fn show_transform(ui: &mut Ui, args: &mut TransformArgs) {
    let words = StoryLabels::of(ui);
    let labels = SymmetryLabels::of(ui);
    let own = SymmetryStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, "transform");
    let seed = [f64::from(args.mode as u8), args.x, args.y, args.angle];
    let Some(initial) = symmetry(args.mode, args.x, args.y, args.angle) else {
        return;
    };
    let mut kept: Kept = ctx.data(|d| d.get_temp(key)).filter(|kept: &Kept| kept.seed == seed).unwrap_or_else(|| Kept::new(initial, seed));

    let mut clicked = Vec::new();
    OptionsBar::new("symmetry-transform").label(labels.transform.clone()).show(ui, |bar| {
        let session = &kept.session;
        bar.trailing(|ui| {
            let pressed = OptionsBar::commit_cancel(ui, &labels.commit, &labels.cancel);
            if pressed.cancel {
                clicked.push(session.cancel());
            }
            if pressed.commit {
                clicked.push(session.commit());
            }
        });
    });
    for event in clicked {
        kept.apply(event);
    }

    let canvas = document(ui, args.view, &words.document);
    let pixels = RulerUnits::pixels(LocaleNumbers::of(ui).as_ref().clone());
    let mut pass = kept.session.pass(SymmetryInput::new(DOC.size(), true).units(&pixels, &pixels).labels(labels));
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut pass);
    let events = pass.finish().events;
    for event in events {
        kept.apply(event);
    }

    let style = SymmetryStyle::of(&Tokens::of(ui));
    let preview = kept.session.preview();
    SymmetryOverlay::new(&preview, DOC.size()).editing(true).ring(kept.session.picked()).paint(
        &ui.painter_at(canvas.view.rect),
        &canvas.view,
        &style,
    );
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = Kept::new(initial, seed);
        }
        let numbers = LocaleNumbers::of(ui);
        let text = craft_i18n::fmt(
            &own.centre,
            &[
                ("x", &numbers.format(preview.center.x, 0..=1)),
                ("y", &numbers.format(preview.center.y, 0..=1)),
                ("angle", &numbers.format(preview.rotation, 0..=1)),
            ],
        );
        let ended = kept.note.map(|ended| match ended {
            Ended::Stored => own.stored.as_str(),
            Ended::Cancelled => own.cancelled.as_str(),
        });
        status(ui, ended.map_or_else(|| text.clone(), |ended| format!("{text} · {ended}")));
    });
    ctx.data_mut(|d| d.insert_temp(key, kept));
}

// --- menu ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ChoiceArg {
    Off,
    Vertical,
    Horizontal,
    #[choice(name = "Dual Axis")]
    Dual,
    Diagonal,
    Path,
}

impl ChoiceArg {
    fn choice(self) -> SymmetryChoice {
        match self {
            ChoiceArg::Off => SymmetryChoice::Off,
            ChoiceArg::Vertical => SymmetryChoice::Mode(SymmetryMode::Vertical),
            ChoiceArg::Horizontal => SymmetryChoice::Mode(SymmetryMode::Horizontal),
            ChoiceArg::Dual => SymmetryChoice::Mode(SymmetryMode::Dual),
            ChoiceArg::Diagonal => SymmetryChoice::Mode(SymmetryMode::Diagonal),
            ChoiceArg::Path => SymmetryChoice::Path("wave".to_owned()),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MenuArgs {
    /// The symmetry in force when the story starts, or after a change of this arg.
    choice: ChoiceArg,
    /// Lists the Work Path and two saved paths.
    paths: bool,
    /// Enables Transform Symmetry; the app enables it with a straight symmetry and a painting
    /// tool.
    #[arg(name = "can transform")]
    can_transform: bool,
}

/// The symmetry in force and the last row picked, with the choice the args started from.
#[derive(Clone, Debug)]
struct MenuKept {
    choice: SymmetryChoice,
    note: String,
    seed: ChoiceArg,
}

/// The button and its menu; a row sets the choice, as the app's command would, and Transform
/// Symmetry only reports.
fn show_menu(ui: &mut Ui, args: &mut MenuArgs) {
    let words = StoryLabels::of(ui);
    let labels = SymmetryLabels::of(ui);
    let own = SymmetryStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, "menu");
    let mut kept: MenuKept = ctx.data(|d| d.get_temp(key)).filter(|kept: &MenuKept| kept.seed == args.choice).unwrap_or_else(|| MenuKept {
        choice: args.choice.choice(),
        note: String::new(),
        seed: args.choice,
    });
    let rows = [
        SymmetryPathRow { key: "work".to_owned(), label: labels.work_path.clone() },
        SymmetryPathRow { key: "wave".to_owned(), label: own.wave.clone() },
        SymmetryPathRow { key: "star".to_owned(), label: own.star.clone() },
    ];
    let paths: &[SymmetryPathRow] = if args.paths { &rows } else { &[] };
    ui.horizontal(|ui| {
        let shown = SymmetryMenu::new("symmetry-menu").current(&kept.choice).paths(paths).can_transform(args.can_transform).labels(&labels).show(ui);
        if let Some(command) = shown.command {
            let name = match &command {
                SymmetryCommand::Off => labels.off.clone(),
                SymmetryCommand::Mode(mode) => match mode {
                    SymmetryMode::Vertical => labels.vertical.clone(),
                    SymmetryMode::Horizontal => labels.horizontal.clone(),
                    SymmetryMode::Dual => labels.dual.clone(),
                    SymmetryMode::Diagonal => labels.diagonal.clone(),
                },
                SymmetryCommand::FromPath(path) => path.clone(),
                SymmetryCommand::Transform => labels.transform.clone(),
            };
            kept.note = craft_i18n::fmt(&words.picked, &[("action", &name)]);
            match command {
                SymmetryCommand::Off => kept.choice = SymmetryChoice::Off,
                SymmetryCommand::Mode(mode) => kept.choice = SymmetryChoice::Mode(mode),
                SymmetryCommand::FromPath(path) => kept.choice = SymmetryChoice::Path(path),
                SymmetryCommand::Transform => {}
            }
        }
    });
    if !kept.note.is_empty() {
        status(ui, kept.note.clone());
    }
    ctx.data_mut(|d| d.insert_temp(key, kept));
}
