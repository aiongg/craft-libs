//! Choice enums that many stories share.

use craft_theme::ControlSize;

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
