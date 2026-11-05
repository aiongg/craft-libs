//! `ChannelRow` and `ChannelKind`: the rows the app passes in, and what the panel reports
//! (`ChannelsEvent`, `ChannelsOutput`).

use craft_theme::Srgba;
use craft_ui::behavior::Move;
use craft_ui::canvas::SelectionOperation;
use craft_ui::edit::EditEvent;
use craft_ui::list::ToggleEdit;
use egui::Response;

use super::footer::ChannelsFooterButton;
use super::menu::ChannelsCommand;

/// What a channel row is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelKind {
    /// The composite (RGB, CMYK, Lab): targeting it targets every colour channel.
    Composite,
    /// One colour channel (Red, Cyan, …); listed only when there is more than one and the
    /// document isn't Multichannel (the app's rule).
    Color,
    /// An alpha channel; `indicates` is its Color Indicates setting.
    Alpha {
        /// Masked or selected areas.
        indicates: ColorIndicates,
    },
    /// A spot channel: italic, with a swatch of its ink at the thumbnail's corner.
    Spot {
        /// The ink, encoded sRGB.
        ink: Srgba,
    },
    /// Quick Mask, while it is on: italic, last.
    QuickMask {
        /// Masked or selected areas.
        indicates: ColorIndicates,
    },
    /// The selected layer's mask, a temporary channel listed while the layer is selected: italic,
    /// named "{layer} Mask" by the app.
    LayerMask {
        /// The mask is enabled (its menu offers Disable, else Enable).
        enabled: bool,
    },
}

impl ChannelKind {
    /// Alpha and spot channels rename in place (F2, a double click on the name).
    pub fn renames(self) -> bool {
        matches!(self, ChannelKind::Alpha { .. } | ChannelKind::Spot { .. })
    }

    /// Alpha and spot channels reorder (drag, Cmd/Ctrl+] and [).
    pub fn moves(self) -> bool {
        self.renames()
    }

    /// Alpha, spot, Quick Mask and layer-mask rows have Channel Options (a double click on the
    /// thumbnail, Enter).
    pub fn has_options(self) -> bool {
        !matches!(self, ChannelKind::Composite | ChannelKind::Color)
    }

    /// Spot, Quick Mask and layer-mask rows draw their name italic (temporary or special
    /// channels, PhotoCraft).
    pub fn italic(self) -> bool {
        matches!(self, ChannelKind::Spot { .. } | ChannelKind::QuickMask { .. } | ChannelKind::LayerMask { .. })
    }
}

/// Channel Options › Color Indicates; it lives in the egui-free [`crate::selection`] with the
/// Quick Mask rules.
pub use crate::selection::ColorIndicates;

/// One of the six overlay colours of the context menu's Overlay Color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlayPreset {
    /// `#ff0000`.
    Red,
    /// `#00c000`.
    Green,
    /// `#0050ff`.
    Blue,
    /// `#00d0e0`.
    Cyan,
    /// `#e000c0`.
    Magenta,
    /// `#f0d000`.
    Yellow,
}

impl OverlayPreset {
    /// Every preset, in the menu's order.
    pub const ALL: [OverlayPreset; 6] =
        [OverlayPreset::Red, OverlayPreset::Green, OverlayPreset::Blue, OverlayPreset::Cyan, OverlayPreset::Magenta, OverlayPreset::Yellow];

    /// The preset's colour, 8-bit sRGB (PhotoCraft's values, `channels_panel.rs:584-589`).
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            OverlayPreset::Red => [0xff, 0x00, 0x00],
            OverlayPreset::Green => [0x00, 0xc0, 0x00],
            OverlayPreset::Blue => [0x00, 0x50, 0xff],
            OverlayPreset::Cyan => [0x00, 0xd0, 0xe0],
            OverlayPreset::Magenta => [0xe0, 0x00, 0xc0],
            OverlayPreset::Yellow => [0xf0, 0xd0, 0x00],
        }
    }
}

/// The overlay opacities of the context menu's Overlay Opacity, in percent.
pub const OVERLAY_OPACITIES: [u8; 4] = [25, 50, 75, 100];

/// One row of the Channels panel, built by the app once per pass in display order: Composite,
/// the colour channels, the layer mask, the alpha and spot channels, Quick Mask (PhotoCraft's).
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelRow<K> {
    /// The app's key of the channel: the key of the row, its thumbnail and its events.
    pub key: K,
    /// What the channel is.
    pub kind: ChannelKind,
    /// The name as shown: the app translates built-in names (Red, Quick Mask) and keeps alpha
    /// and spot names verbatim.
    pub name: String,
    /// The eye: off leaves the cell empty.
    pub visible: bool,
    /// The app's highlight: the targeted channel, or every colour row while the composite is
    /// targeted (PhotoCraft).
    pub targeted: bool,
}

impl<K> ChannelRow<K> {
    /// A visible, untargeted row.
    pub fn new(key: K, kind: ChannelKind, name: impl Into<String>) -> Self {
        Self { key, kind, name: name.into(), visible: true, targeted: false }
    }
}

/// The row the footer's Delete and Load act on: the targeted layer-mask, alpha, spot or Quick
/// Mask row; else the composite when it is targeted; else the one targeted colour row; `None`
/// with no rows or no target.
pub fn targeted_row<K>(rows: &[ChannelRow<K>]) -> Option<&ChannelRow<K>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:458-464 (`target_ref` from the view's target;
    // the mask row while it is the visible target, :478-485).
    let targeted = |kinds: fn(ChannelKind) -> bool| rows.iter().find(|row| row.targeted && kinds(row.kind));
    targeted(|kind| !matches!(kind, ChannelKind::Composite | ChannelKind::Color))
        .or_else(|| targeted(|kind| matches!(kind, ChannelKind::Composite)))
        .or_else(|| targeted(|kind| matches!(kind, ChannelKind::Color)))
}

/// What the Channels panel reports, in the order it happened.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ChannelsEvent<K> {
    /// A click on a row (not with Cmd/Ctrl), or ↑ ↓ to a row: the app targets the channel
    /// (PhotoCraft: Quick Mask targets the composite).
    Target(K),
    /// An eye click (`Set`) or a sweep over several eyes (`Begin`, `Update`, `Commit`; Esc and
    /// focus loss `Cancel`).
    Visibility(EditEvent<ToggleEdit<K>>),
    /// Cmd/Ctrl-click on a row, or Cmd/Ctrl+Enter on the lead: load the channel as a selection
    /// with the operation of Shift and Alt.
    LoadSelection {
        /// The channel.
        key: K,
        /// New, add (Shift), subtract (Alt), intersect (Shift+Alt).
        operation: SelectionOperation,
    },
    /// A rename committed (alpha and spot rows): never empty after trimming, never unchanged.
    Rename {
        /// The channel.
        key: K,
        /// The new name, trimmed.
        text: String,
    },
    /// A double click on the thumbnail, Enter, or Channel Options… in a menu, on a row that
    /// [has options](ChannelKind::has_options): the app opens its Channel Options dialog.
    Options(K),
    /// An alpha or spot row moved: a drag (`Begin`, `Commit` or `Cancel`) or Cmd/Ctrl+] and [
    /// (`Set`). Targets: before or after another alpha or spot row, before Quick Mask.
    Reorder(EditEvent<Move<K>>),
    /// A click on a footer button; `rows` holds [`targeted_row`]'s key (empty without rows).
    Footer {
        /// The button.
        button: ChannelsFooterButton,
        /// The rows it acts on.
        rows: Vec<K>,
    },
    /// A row dropped on a footer button: New duplicates it, Delete deletes it (the app asks first
    /// for a layer mask), Load Channel as Selection loads it. Refused drops report nothing.
    FooterDrop {
        /// The button.
        button: ChannelsFooterButton,
        /// The dropped rows: the row the pointer grabbed.
        rows: Vec<K>,
    },
    /// A pick in a row's context menu or the panel menu.
    Command {
        /// The command.
        command: ChannelsCommand,
        /// The row it was opened on (the targeted row for the panel menu); `None` for the
        /// commands that act on the document (New Channel…, Split Channels).
        key: Option<K>,
    },
}

/// What [`ChannelsPanel::show`](super::ChannelsPanel::show) returns.
#[derive(Debug)]
#[must_use = "the app applies the panel's events"]
pub struct ChannelsOutput<K> {
    /// What the user did this pass, in order.
    pub events: Vec<ChannelsEvent<K>>,
    /// The panel.
    pub response: Response,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: u8, kind: ChannelKind, targeted: bool) -> ChannelRow<u8> {
        ChannelRow { targeted, ..ChannelRow::new(key, kind, "c") }
    }

    // While the composite is targeted every colour row is highlighted too; the footer acts on the
    // composite. A targeted alpha row wins over both.
    #[test]
    fn targeted_row_prefers_the_single_channel_then_the_composite() {
        let indicates = ColorIndicates::MaskedAreas;
        let mut rows = vec![row(0, ChannelKind::Composite, true), row(1, ChannelKind::Color, true), row(2, ChannelKind::Color, true)];
        assert_eq!(targeted_row(&rows).map(|r| r.key), Some(0));
        rows.push(row(3, ChannelKind::Alpha { indicates }, true));
        assert_eq!(targeted_row(&rows).map(|r| r.key), Some(3));
        rows[0].targeted = false;
        rows[3].targeted = false;
        rows[2].targeted = false;
        assert_eq!(targeted_row(&rows).map(|r| r.key), Some(1));
        rows[1].targeted = false;
        assert!(targeted_row(&rows).is_none());
    }
}
