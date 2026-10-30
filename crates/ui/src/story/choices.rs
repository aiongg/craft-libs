//! Choice enums that many stories share.

use craft_theme::{ControlSize, Tone};

use super::StoryChoice;

/// The `size` arg of a control: `sm` or `md`.
#[derive(StoryChoice, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeArg {
    Sm,
    #[default]
    Md,
}

impl SizeArg {
    /// The size the arg names.
    pub const fn control(self) -> ControlSize {
        match self {
            SizeArg::Sm => ControlSize::Sm,
            SizeArg::Md => ControlSize::Md,
        }
    }
}

/// The `tone` arg of a component: a [`Tone`] of the notice family.
#[derive(StoryChoice, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToneArg {
    #[default]
    Neutral,
    Info,
    Good,
    Warn,
    Bad,
}

impl ToneArg {
    /// The tone the arg names.
    pub const fn tone(self) -> Tone {
        match self {
            ToneArg::Neutral => Tone::Neutral,
            ToneArg::Info => Tone::Info,
            ToneArg::Good => Tone::Good,
            ToneArg::Warn => Tone::Warn,
            ToneArg::Bad => Tone::Bad,
        }
    }
}
