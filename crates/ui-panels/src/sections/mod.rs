//! Sections and title rows: `TitleRow`, `FooterBar`, `Section`, `GroupBox` and `Panel`.

pub(crate) mod footer;
#[cfg(spike_full)]
pub(crate) mod group_box;
#[cfg(feature = "suite")]
#[cfg(spike_full)]
mod sample;
#[cfg(spike_full)]
pub(crate) mod section;
#[cfg(spike_full)]
pub(crate) mod stack;
#[cfg(spike_full)]
pub(crate) mod title_row;
mod words;

// Panel.
#[cfg(spike_full)]
pub(crate) mod panel;

pub use footer::{
    DialogButtonLook, DialogRole, DropAction, DropActionOutput, DropActionStyle, FooterAction, FooterBar, FooterBarOutput, FooterButton, FooterDrop,
    FooterOutput,
};
#[cfg(spike_full)]
pub use group_box::{GroupBox, GroupBoxStyle};
#[cfg(spike_full)]
pub use section::{Section, SectionOutput, SectionStyle};
#[cfg(spike_full)]
pub use stack::{SectionStack, SectionStackOutput, SectionStackStyle, StackRows, StackSlot};
#[doc(hidden)]
#[cfg(spike_full)]
pub use title_row::MenuRows;
#[cfg(spike_full)]
pub use title_row::{HeaderAction, HeaderCommand, HeaderStrength, QuietTitleRow, TitleOutput, TitleRow, TitleRowStyle};
pub use words::SectionWords;

#[cfg(spike_full)]
pub use panel::{Panel, PanelOutput, PanelStyle};

// Reorderable sections.
#[cfg(spike_full)]
mod layout;
#[cfg(spike_full)]
pub(crate) mod reorderable;

#[cfg(spike_full)]
pub use layout::{SectionEvent, SectionKey, SectionLayout, SectionState};
#[cfg(spike_full)]
pub use reorderable::{ReorderableSections, ReorderableSectionsOutput, SectionButton, SectionCommands, SectionSpec};
pub use words::ReorderableWords;
