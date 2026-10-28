//! The design system's controls, built on the [primitives](crate::primitives).
//!
//! - [`Button`]: a push button in one of four [`ButtonVariant`]s, optionally with a leading icon
//!   and a loading state.
//! - [`IconButton`]: a square ghost button showing only an icon, optionally a toggle.
//! - [`ToggleButton`]: a secondary-looking button that stays pressed while on.
//! - [`Segmented`]: one of a few options (tool modes) on a track, a WAI-ARIA radio group.
//! - [`Checkbox`]: a box that is checked, unchecked or mixed ([`CheckState`]).
//! - [`RadioGroup`] and [`RadioSet`]: one of several options as radio buttons, a WAI-ARIA radio
//!   group; a set places each option where the layout needs it.
//! - [`Switch`]: an on/off setting that applies at once.
//! - [`Swatch`]: a colour well that reports a click, for the host to open its picker.
//! - [`Badge`]: a small label in a tone; [`Kbd`]: a key chord in the platform's notation.
//! - [`Spinner`] and [`Progress`]: indeterminate and determinate progress.
//! - [`Tooltip`] and [`TooltipExt`]: the look of every tooltip, and the helpers that add one to a
//!   [`Response`](egui::Response).
//! - [`Link`]: text in `accent.text` that opens a URL or reports a click.
//! - [`Slider`] and [`RangeSlider`]: one value, or a lower and an upper value, on a track; they
//!   report through the edit lifecycle ([`crate::edit`]) and return an
//!   [`EditResponse`](crate::edit::EditResponse).
//! - [`TextField`] and [`SearchField`]: one line of text; a typing run is one edit gesture.
//!
//! Controls take semantic properties only (variant, [`ControlSize`], icon, state), never sizes or
//! colours. Colours come from their component tokens (`button.*`, `icon_button.*`,
//! `segmented.*`, `check.*`, `slider.*`, `swatch.*`, `badge.*`, `kbd.*`, `progress.*`, `field.*`), so a theme's component
//! overrides reach them; shape and edge come from the
//! [`Surface`](crate::primitives::Surface) primitive through
//! [`Interactive`](crate::primitives::Interactive). Each has a pure `style` function of
//! [`UiTokens`](crate::theme::UiTokens) and its properties, dumped under
//! `crates/ui/tests/dumps/components/`.
//!
//! A control's own `enabled(false)` draws its disabled tokens. In a disabled `Ui` a control is
//! disabled too, and egui fades it as it fades everything there.
//!
//! ```
//! use craft_ui::controls::{Button, ButtonVariant, ControlSize};
//! # egui::__run_test_ui(|ui| {
//! if ui.add(Button::new("Export").variant(ButtonVariant::Primary).size(ControlSize::Md)).clicked() {
//!     // export
//! }
//! # });
//! ```

pub(crate) mod badge;
pub(crate) mod button;
mod check;
pub(crate) mod checkbox;
pub(crate) mod icon_button;
pub(crate) mod kbd;
pub(crate) mod link;
pub(crate) mod progress;
pub(crate) mod radio;
pub(crate) mod range_slider;
pub(crate) mod segmented;
pub(crate) mod slider;
#[cfg(test)]
mod slider_tests;
#[cfg(test)]
mod small_tests;
pub(crate) mod spinner;
pub(crate) mod swatch;
pub(crate) mod switch;
#[cfg(test)]
mod tests;
pub(crate) mod text_field;
#[cfg(test)]
mod text_field_tests;
pub(crate) mod toggle_button;
pub(crate) mod tooltip;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod ui_tests;

use egui::{IdSalt, Ui};

pub use badge::{Badge, BadgeLook, BadgeStyle, BadgeTone};
pub use button::{Button, ButtonStyle};
pub use check::IndicatorColors;
pub use checkbox::{CheckState, CheckValue, Checkbox, CheckboxStyle};
pub use craft_theme::{ButtonVariant, ControlSize};
pub use icon_button::{IconButton, IconButtonStyle};
pub use kbd::{Kbd, KbdStyle};
pub use link::{Link, LinkStyle, LinkUnderline};
pub use progress::{Progress, ProgressStyle};
pub use radio::{PreparedRadioSet, RadioGroup, RadioSet, RadioStyle};
pub use range_slider::RangeSlider;
pub use segmented::{Segmented, SegmentedStyle};
pub use slider::{Slider, SliderStyle};
pub use spinner::Spinner;
pub use swatch::{Swatch, SwatchStyle};
pub use switch::{Switch, SwitchStyle};
pub use text_field::{SearchField, TextField, TextFieldStyle};
pub use toggle_button::{ToggleButton, ToggleButtonStyle};
pub use tooltip::{Tooltip, TooltipExt, TooltipStyle};

use crate::icons::IconSize;

/// The icon size class of a control size; the points are the family's `icon` token.
pub(crate) const fn icon_size(size: ControlSize) -> IconSize {
    match size {
        ControlSize::Sm => IconSize::Sm,
        ControlSize::Md => IconSize::Md,
    }
}

/// `id_salt`, or the `Ui`'s next automatic id, as egui's own widgets take.
pub(crate) fn salt(ui: &mut Ui, id_salt: Option<IdSalt>) -> IdSalt {
    id_salt.unwrap_or_else(|| {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        IdSalt::new(id)
    })
}

/// The names of `ControlSize`s, for story args and dumps.
pub(crate) const fn size_name(size: ControlSize) -> &'static str {
    match size {
        ControlSize::Sm => "sm",
        ControlSize::Md => "md",
    }
}

/// Every control size, small first.
pub(crate) const SIZES: [ControlSize; 2] = [ControlSize::Sm, ControlSize::Md];

/// Story arg options for sizes, in `SIZES` order.
pub(crate) const SIZE_NAMES: &[&str] = &["sm", "md"];

/// The size a story's `size` arg names.
pub(crate) fn size_arg(name: &str) -> ControlSize {
    SIZES.into_iter().find(|s| size_name(*s) == name).unwrap_or(ControlSize::Md)
}
