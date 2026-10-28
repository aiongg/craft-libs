//! [`Stack`] and [`Row`]: layouts with token gaps and padding.

use craft_theme::Tokens as ThemeTokens;
use egui::{Frame, InnerResponse, Ui, vec2};

use crate::direction;
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, margin};

/// The space between neighbouring items: `space.gap.*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Gap {
    /// Between items on one line.
    Inline,
    /// Between stacked rows.
    Stack,
    /// Between groups.
    Section,
}

impl Gap {
    /// Every gap, smallest first.
    pub const ALL: &'static [Gap] = &[Gap::Inline, Gap::Stack, Gap::Section];

    /// The name in story args and dumps.
    pub const fn name(self) -> &'static str {
        match self {
            Gap::Inline => "inline",
            Gap::Stack => "stack",
            Gap::Section => "section",
        }
    }

    /// The gap in points.
    pub fn points(self, tokens: &ThemeTokens) -> f32 {
        let gap = &tokens.space.gap;
        match self {
            Gap::Inline => gap.inline,
            Gap::Stack => gap.stack,
            Gap::Section => gap.section,
        }
    }
}

/// Padding inside a container: `space.pad.*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Pad {
    /// Inside controls.
    Control,
    /// Inside panels.
    Panel,
    /// Inside dialogs.
    Dialog,
}

impl Pad {
    /// Every padding, smallest first.
    pub const ALL: &'static [Pad] = &[Pad::Control, Pad::Panel, Pad::Dialog];

    /// The name in story args and dumps.
    pub const fn name(self) -> &'static str {
        match self {
            Pad::Control => "control",
            Pad::Panel => "panel",
            Pad::Dialog => "dialog",
        }
    }

    /// The padding in points.
    pub fn points(self, tokens: &ThemeTokens) -> f32 {
        let pad = &tokens.space.pad;
        match self {
            Pad::Control => pad.control,
            Pad::Panel => pad.panel,
            Pad::Dialog => pad.dialog,
        }
    }
}

/// Items top to bottom, against the reading start, `gap` apart.
#[derive(Clone, Copy, Debug)]
pub struct Stack {
    gap: Gap,
    pad: Option<Pad>,
}

impl Stack {
    pub fn new(gap: Gap) -> Self {
        Self { gap, pad: None }
    }

    /// Padding on every side.
    pub fn pad(mut self, pad: Pad) -> Self {
        self.pad = Some(pad);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let tokens = Tokens::of(ui);
        let gap = self.gap.points(&tokens);
        padded(ui, self.pad, |ui| {
            ui.with_layout(direction::column(ui), |ui| {
                ui.spacing_mut().item_spacing.y = gap;
                add_contents(ui)
            })
            .inner
        })
    }
}

/// Items in reading order (from the right in a right-to-left interface), vertically centred,
/// `gap` apart.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    gap: Gap,
    pad: Option<Pad>,
}

impl Row {
    pub fn new(gap: Gap) -> Self {
        Self { gap, pad: None }
    }

    /// Padding on every side.
    pub fn pad(mut self, pad: Pad) -> Self {
        self.pad = Some(pad);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let tokens = Tokens::of(ui);
        let gap = self.gap.points(&tokens);
        padded(ui, self.pad, |ui| {
            // As `ui.horizontal`: one control high to start with, so items centre on the row.
            let size = vec2(ui.available_size_before_wrap().x, ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(size, direction::row(ui), |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                add_contents(ui)
            })
            .inner
        })
    }
}

fn padded<R>(ui: &mut Ui, pad: Option<Pad>, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    match pad {
        Some(pad) => Frame::new().inner_margin(margin(pad.points(&Tokens::of(ui)))).show(ui, add_contents),
        None => ui.scope(add_contents),
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Stack", "Row"];

const GAP_NAMES: &[&str] = &["inline", "stack", "section"];
const PAD_NAMES: &[&str] = &["none", "control", "panel", "dialog"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "primitives/stack/gaps",
            level: Level::Primitives,
            component: "Stack",
            name: "Gaps",
            args: || Args::new().with_choice("gap", GAP_NAMES, 1).with_choice("pad", PAD_NAMES, 2),
            show: show_stack,
        },
        Story {
            id: "primitives/row/gaps",
            level: Level::Primitives,
            component: "Row",
            name: "Gaps",
            args: || Args::new().with_choice("gap", GAP_NAMES, 0).with_choice("pad", PAD_NAMES, 2),
            show: show_row,
        },
    ]
}

fn gap_arg(args: &Args) -> Gap {
    let name = args.choice("gap");
    Gap::ALL.iter().copied().find(|g| g.name() == name).unwrap_or(Gap::Stack)
}

fn pad_arg(args: &Args) -> Option<Pad> {
    let name = args.choice("pad");
    Pad::ALL.iter().copied().find(|p| p.name() == name)
}

fn show_stack(ui: &mut Ui, args: &mut Args) {
    let stack = Stack::new(gap_arg(args));
    let stack = match pad_arg(args) {
        Some(pad) => stack.pad(pad),
        None => stack,
    };
    outlined(ui, |ui| {
        stack.show(ui, |ui| {
            for name in ["First", "Second", "Third"] {
                ui.add(super::Text::new(name));
            }
        });
    });
}

fn show_row(ui: &mut Ui, args: &mut Args) {
    let row = Row::new(gap_arg(args));
    let row = match pad_arg(args) {
        Some(pad) => row.pad(pad),
        None => row,
    };
    outlined(ui, |ui| {
        row.show(ui, |ui| {
            for name in ["First", "Second", "Third"] {
                ui.add(super::Text::new(name));
            }
        });
    });
}

/// The layout on a sunken surface, so its padding shows.
fn outlined(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    super::Surface::new(super::SurfaceKind::Sunken).show(ui, add);
}
