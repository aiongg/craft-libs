//! [`AdjustContext`], what every adjustment editor shares with its host, and the tone channels of
//! Levels and Curves ([`ToneSpace`], [`ToneChannel`], [`ToneChannels`]) (task AD-1).

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Id, InnerResponse, Ui, UiBuilder};

use craft_ui::composites::ChannelColor;

use super::AdjustLabels;

/// Where an adjustment editor is shown and how its sliders look: one value for every editor.
///
/// ```
/// use craft_imaging::ui::adjust::AdjustContext;
/// // Image › Adjustments › Hue/Saturation, with Preferences › Interface › Dynamic Color Sliders on.
/// let dialog = AdjustContext::dialog(true);
/// assert!(dialog.wheel());
/// // The same editor on an adjustment layer in the Properties panel.
/// let panel = AdjustContext::properties(true).gray(true);
/// assert!(!panel.wheel() && panel.is_gray());
/// ```
// pc: crates/ui-egui/src/dialogs/adjust_editors.rs:105-124 `EditorCx` (`dialog`,
// `dynamic_color_sliders`, `gray`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AdjustContext {
    dialog: bool,
    dynamic_color_sliders: bool,
    gray: bool,
}

impl AdjustContext {
    /// An editor in an Image › Adjustments dialog: the mouse wheel over a slider steps it without
    /// focus, by its wheel step (1; Exposure 0.01), as in Photoshop's dialogs. `dynamic` is
    /// Preferences › Interface › Dynamic Color Sliders: colour tracks on the colour sliders, the
    /// plain track when off.
    pub fn dialog(dynamic: bool) -> Self {
        Self { dialog: true, dynamic_color_sliders: dynamic, gray: false }
    }

    /// An editor in the Properties panel (an adjustment layer): the wheel scrolls the panel and
    /// never steps a slider. `dynamic` as in [`dialog`](Self::dialog).
    pub fn properties(dynamic: bool) -> Self {
        Self { dialog: false, dynamic_color_sliders: dynamic, gray: false }
    }

    /// A Grayscale (or Duotone, Bitmap) document: Levels and Curves list one Gray channel, the
    /// Neutral Gray picker is off, and Curves' ⌘-click on the image reads the grey mean.
    pub fn gray(mut self, gray: bool) -> Self {
        self.gray = gray;
        self
    }

    /// Whether the editor is in a dialog.
    pub fn is_dialog(&self) -> bool {
        self.dialog
    }

    /// Whether the wheel steps an unfocused slider: in a dialog only.
    pub fn wheel(&self) -> bool {
        self.dialog
    }

    /// Whether colour sliders draw their colour tracks.
    pub fn dynamic_color_sliders(&self) -> bool {
        self.dynamic_color_sliders
    }

    /// Whether the document is grey.
    pub fn is_gray(&self) -> bool {
        self.gray
    }

    /// `stops` when the colour tracks are on, else `None` (the plain track).
    // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:46 `track_stops`.
    pub fn track<'t>(&self, stops: &'t [Srgba]) -> Option<&'t [Srgba]> {
        self.dynamic_color_sliders.then_some(stops)
    }
}

/// The channel space a Levels or Curves value addresses.
// pc: crates/ui-egui/src/dialogs/adjust_editors.rs:127 `space_of`; engine `adjust_params.rs`
// `channel_keys`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToneSpace {
    /// RGB (and Grayscale with [`AdjustContext::gray`]): the composite, then Red, Green, Blue.
    #[default]
    Rgb,
    /// CMYK: the composite, then Cyan, Magenta, Yellow, Black.
    Cmyk,
    /// Lab: Lightness, a, b; no composite.
    Lab,
}

/// One channel of a tone editor's Channel select.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToneChannel {
    /// The composite of RGB ("RGB") or CMYK ("CMYK").
    Composite(ToneSpace),
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    Black,
    Lightness,
    A,
    B,
    /// The one channel of a grey document.
    Gray,
    /// A channel the host names, shown exactly as given (a user-named alpha channel, a layer mask
    /// or Quick Mask isolated in the Channels panel, PhotoCraft #5335), in `color`.
    Named {
        name: String,
        color: ChannelColor,
    },
}

impl ToneChannel {
    /// The channel's curve and histogram colour: Red, Green, Blue, Cyan, Magenta, Yellow their own;
    /// the composite, Black, Lightness and Gray neutral; Lab a magenta and b yellow (as
    /// [`ChannelColor`] documents).
    // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:69 `hist_color`, :76 `bar_color`.
    pub fn color(&self) -> ChannelColor {
        match self {
            ToneChannel::Red => ChannelColor::Red,
            ToneChannel::Green => ChannelColor::Green,
            ToneChannel::Blue => ChannelColor::Blue,
            ToneChannel::Cyan => ChannelColor::Cyan,
            ToneChannel::Magenta | ToneChannel::A => ChannelColor::Magenta,
            ToneChannel::Yellow | ToneChannel::B => ChannelColor::Yellow,
            ToneChannel::Named { color, .. } => *color,
            ToneChannel::Composite(_) | ToneChannel::Black | ToneChannel::Lightness | ToneChannel::Gray => ChannelColor::Neutral,
        }
    }

    /// The channel's name in the Channel select, from `labels` ("RGB", "Red", "a"); a
    /// [`Named`](Self::Named) channel's own name.
    pub fn label<'l>(&'l self, labels: &'l AdjustLabels) -> &'l str {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:18-41 `tone_channels` (the
        // labels), :50 `channel_picker_opts` (a user-typed name shown as entered).
        match self {
            ToneChannel::Composite(ToneSpace::Rgb) => &labels.rgb,
            ToneChannel::Composite(ToneSpace::Cmyk) => &labels.cmyk,
            ToneChannel::Composite(ToneSpace::Lab) | ToneChannel::Lightness => &labels.lightness,
            ToneChannel::Red => &labels.red,
            ToneChannel::Green => &labels.green,
            ToneChannel::Blue => &labels.blue,
            ToneChannel::Cyan => &labels.cyan,
            ToneChannel::Magenta => &labels.magenta,
            ToneChannel::Yellow => &labels.yellow,
            ToneChannel::Black => &labels.black,
            ToneChannel::A => &labels.lab_a,
            ToneChannel::B => &labels.lab_b,
            ToneChannel::Gray => &labels.gray,
            ToneChannel::Named { name, .. } => name,
        }
    }
}

/// The channels a Levels or Curves panel lists, in Channel-select order; a histogram, a Levels
/// value or a curve of the host is addressed by the same index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToneChannels {
    channels: Vec<ToneChannel>,
    gray: bool,
    isolated: bool,
}

impl ToneChannels {
    /// The channels of `space` (PhotoCraft's `tone_channels`, Photoshop's list):
    ///
    /// | Space | Channels |
    /// |---|---|
    /// | RGB | RGB, Red, Green, Blue |
    /// | RGB with `gray` | Gray |
    /// | CMYK | CMYK, Cyan, Magenta, Yellow, Black |
    /// | Lab | Lightness, a, b |
    pub fn of(space: ToneSpace, gray: bool) -> Self {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:18-41 `tone_channels`.
        let channels = match space {
            ToneSpace::Rgb if gray => vec![ToneChannel::Gray],
            ToneSpace::Rgb => vec![ToneChannel::Composite(space), ToneChannel::Red, ToneChannel::Green, ToneChannel::Blue],
            ToneSpace::Cmyk => {
                vec![ToneChannel::Composite(space), ToneChannel::Cyan, ToneChannel::Magenta, ToneChannel::Yellow, ToneChannel::Black]
            }
            ToneSpace::Lab => vec![ToneChannel::Lightness, ToneChannel::A, ToneChannel::B],
        };
        Self { gray: gray && space == ToneSpace::Rgb, channels, isolated: false }
    }

    /// One channel the host isolated (Image › Curves with one Channels-panel target selected,
    /// PhotoCraft #5335): the select shows only it, Curves overlays no other channel, and the
    /// ⌘-click point and the pickers are the host's to refuse for a mask.
    pub fn isolated(channel: ToneChannel) -> Self {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:251-270 (`isolated_curve`).
        Self { channels: vec![channel], gray: false, isolated: true }
    }

    /// The channels in order.
    pub fn as_slice(&self) -> &[ToneChannel] {
        &self.channels
    }

    /// How many channels there are.
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Whether the list is empty (never for [`of`](Self::of)).
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// Whether the list is a grey document's single Gray channel.
    pub fn is_gray(&self) -> bool {
        self.gray
    }

    /// Whether the list is one isolated channel ([`isolated`](Self::isolated)).
    pub fn is_isolated(&self) -> bool {
        self.isolated
    }

    /// Whether channel `index` is a composite (RGB or CMYK), over which Curves overlays the
    /// other channels' curves.
    pub fn is_composite(&self, index: usize) -> bool {
        matches!(self.channels.get(index), Some(ToneChannel::Composite(_)))
    }
}

/// Draws `add` in a group node named `name` for screen readers, disabled when `enabled` is false.
/// The group's node gets its bounds from the scope, which egui does not give a `Ui`'s node.
pub(super) fn named_group<R>(ui: &mut Ui, id: Id, name: &str, enabled: bool, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(name);
    if !enabled {
        builder = builder.disabled();
    }
    let group = ui.scope_builder(builder, add);
    let rect = group.response.rect;
    let bounds = egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() };
    ui.ctx().accesskit_node_builder(id, |node| node.set_bounds(bounds));
    group
}
