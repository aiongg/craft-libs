//! [`Stack`] and [`Row`]: layouts with token gaps and padding.

use craft_theme::Tokens as ThemeTokens;
use egui::{Frame, InnerResponse, Margin, Ui, vec2};

use crate::direction;
use crate::inspect::{self, RecordKind};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{Tokens, TokensOf, margin};
use crate::typed_story;

/// The space between neighbouring items: `space.gap.*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Gap {
    /// Between items on one line.
    Inline,
    /// Between the rows of a panel body.
    Row,
    /// Between stacked rows.
    Stack,
    /// Between groups.
    Section,
}

impl Gap {
    /// Every gap, smallest first.
    pub const ALL: &'static [Gap] = &[Gap::Inline, Gap::Row, Gap::Stack, Gap::Section];

    /// The name in story args and dumps.
    pub const fn name(self) -> &'static str {
        match self {
            Gap::Inline => "inline",
            Gap::Row => "row",
            Gap::Stack => "stack",
            Gap::Section => "section",
        }
    }

    /// The gap in points.
    pub fn points(self, tokens: &ThemeTokens) -> f32 {
        let gap = &tokens.space.gap;
        match self {
            Gap::Inline => gap.inline,
            Gap::Row => gap.row,
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
        padded(ui, self.pad, RecordKind::Stack, |ui| {
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
        padded(ui, self.pad, RecordKind::Row, |ui| {
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

fn padded<R>(ui: &mut Ui, pad: Option<Pad>, kind: RecordKind, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let (inner, inset) = match pad {
        Some(pad) => {
            let inset = margin(pad.points(&Tokens::of(ui)));
            (Frame::new().inner_margin(inset).show(ui, add_contents), inset)
        }
        None => (ui.scope(add_contents), Margin::ZERO),
    };
    let rect = inner.response.rect;
    inspect::record(ui, inner.response.id, kind, rect, rect - inset);
    inner
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Stack", "Row"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum GapArg {
    Inline,
    Row,
    Stack,
    Section,
}

impl GapArg {
    fn gap(self) -> Gap {
        match self {
            GapArg::Inline => Gap::Inline,
            GapArg::Row => Gap::Row,
            GapArg::Stack => Gap::Stack,
            GapArg::Section => Gap::Section,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PadArg {
    None,
    Control,
    Panel,
    Dialog,
}

impl PadArg {
    fn pad(self) -> Option<Pad> {
        match self {
            PadArg::None => None,
            PadArg::Control => Some(Pad::Control),
            PadArg::Panel => Some(Pad::Panel),
            PadArg::Dialog => Some(Pad::Dialog),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct GapsArgs {
    gap: GapArg,
    pad: PadArg,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/stack/gaps", Level::Primitives, "Stack", "Gaps", GapsArgs { gap: GapArg::Stack, pad: PadArg::Panel }, show_stack),
        typed_story!("primitives/row/gaps", Level::Primitives, "Row", "Gaps", GapsArgs { gap: GapArg::Inline, pad: PadArg::Panel }, show_row),
    ]
}

fn show_stack(ui: &mut Ui, args: &mut GapsArgs) {
    let stack = Stack::new(args.gap.gap());
    let stack = match args.pad.pad() {
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

fn show_row(ui: &mut Ui, args: &mut GapsArgs) {
    let row = Row::new(args.gap.gap());
    let row = match args.pad.pad() {
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
