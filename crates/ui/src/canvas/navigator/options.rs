//! Panel Options: [`NavigatorSettings`], [`NavigatorOptions`], [`ProxyColor`] and
//! [`NavigatorOptionsDialog`].

use craft_theme::Srgba;
use egui::{Color32, Context};

use super::labels::NavigatorLabels;
use crate::dialog::{ColorRequest, DialogId, DialogResult, DialogStack, StackDialog};
use crate::theme::UiTokens;

/// The [`ColorRequest::key`] of Panel Options' Custom… colour.
pub const NAVIGATOR_PROXY_COLOR_KEY: &str = "navigator.proxyColor";

/// The proxy rectangle's colour (Panel Options › Color).
///
/// `Default` is the theme's `navigator.proxy` (PhotoCraft's and Photoshop's light red,
/// `rgb(255, 84, 84)`); the named colours are fixed data the same in every theme, chosen to read on
/// the dark preview box (lead call L2: Photoshop's exact list was not available, so the hues follow
/// PhotoCraft's Button Mode names); `Custom` is a colour from the app's picker, kept as 8-bit sRGB
/// (as `GuideColor::Custom`). Serialised by variant name, `Custom` as `{"Custom": [r, g, b]}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProxyColor {
    #[default]
    Default,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
    Gray,
    /// Chosen with Custom… (the app's colour picker).
    Custom([u8; 3]),
}

impl ProxyColor {
    /// The colours the dialog lists, in order; Custom… follows them.
    pub const ALL: [ProxyColor; 8] = [
        ProxyColor::Default,
        ProxyColor::Red,
        ProxyColor::Orange,
        ProxyColor::Yellow,
        ProxyColor::Green,
        ProxyColor::Blue,
        ProxyColor::Violet,
        ProxyColor::Gray,
    ];

    /// The colour's name in the dialog; `Custom` is [`NavigatorLabels::custom`].
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn label(self, labels: &NavigatorLabels) -> &str {
        // new: one word per variant (`navigator.*`).
        todo!()
    }

    /// The colour drawn: `Default` reads `navigator.proxy` from `tokens`; the named ones are the
    /// lead's data (red 255/84/84, orange 255/160/60, yellow 255/214/64, green 92/214/110, blue
    /// 84/160/255, violet 176/124/255, gray 200/200/200), moved into `values.rs` by TK2; `Custom`
    /// is its own value.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn color(self, tokens: &UiTokens) -> Color32 {
        // new: lead call L1, L2; until TK2 `Default` is `Navigator::style(tokens).proxy`.
        todo!()
    }
}

/// The Navigator's options, the value [`NavigatorOptionsDialog`] edits and answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NavigatorOptions {
    /// The proxy rectangle's colour.
    pub proxy_color: ProxyColor,
}

/// The Navigator's persisted section: the app keeps it with its preferences and passes it to
/// [`Navigator::settings`](super::Navigator::settings). It holds the dialog's
/// [`NavigatorOptions`] (as `LayersSettings` holds `LayersOptions`), flattened, so the JSON is
/// `{"proxyColor": "Red"}`. Unknown fields are ignored and missing ones take their defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NavigatorSettings {
    /// The Panel Options values.
    #[serde(flatten)]
    pub options: NavigatorOptions,
}

/// Navigator Panel Options: the proxy colour.
///
/// The user sees a Color list ([`ProxyColor::ALL`], each with its swatch, then Custom…; one Tab
/// stop, arrows pick), and OK and Cancel. Enter is OK, Esc is Cancel. With Custom… a swatch shows
/// the custom colour; choosing Custom… or clicking its swatch asks the app for a colour: in that
/// pass [`pick_color`](Self::pick_color) returns a [`ColorRequest`] keyed
/// [`NAVIGATOR_PROXY_COLOR_KEY`], titled [`NavigatorLabels::picker_title`], and the app answers
/// with [`set_color`](Self::set_color) or [`cancel_color`](Self::cancel_color), as
/// `NewGuideDialog` does.
///
/// OK answers the [`NavigatorOptions`] shown (one settings change for the app); Cancel leaves the
/// settings as they were. The dialog never edits the settings itself:
///
/// ```ignore
/// dialogs.open(NavigatorOptionsDialog::new().initial(settings.options));
/// // later, when it answers:
/// if let DialogResult::Ok(options) = result { settings.options = options; persist(&settings); }
/// ```
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct NavigatorOptionsDialog {
    pub(super) id: DialogId,
    pub(super) options: NavigatorOptions,
    /// The colour Custom… starts the picker at: the last custom colour.
    pub(super) custom: [u8; 3],
    /// The request to hand out once.
    pub(super) color_request: Option<ColorRequest>,
    /// The colour before Custom… was chosen, while the picker is open.
    pub(super) before_custom: Option<ProxyColor>,
    pub(super) state: crate::dialog::DialogState,
}

impl NavigatorOptionsDialog {
    /// Panel Options with the default options.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn new() -> Self {
        // new: `dialog_id("navigator.panelOptions")`, as `NewGuideDialog::new`.
        todo!()
    }

    /// Opens with `options` (the app's current settings).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn initial(self, options: NavigatorOptions) -> Self {
        // new: as `NewGuideDialog::initial`.
        todo!()
    }

    /// The options shown now: what OK answers.
    pub fn options(&self) -> NavigatorOptions {
        self.options
    }

    /// Custom… was chosen (or its swatch clicked) since the last call: the colour to ask the app's
    /// picker for, keyed [`NAVIGATOR_PROXY_COLOR_KEY`]. Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`: Color becomes [`ProxyColor::Custom`]
    /// (rounded to 8 bits). Other keys are ignored.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        // new: as craft-ui crates/ui/src/dialog/guides.rs:462 `NewGuideDialog::set_color`.
        todo!()
    }

    /// The picker was cancelled: the colour goes back to what it was before Custom….
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn cancel_color(&mut self) {
        // new: as craft-ui crates/ui/src/dialog/guides.rs:474 `NewGuideDialog::cancel_color`.
        todo!()
    }
}

impl Default for NavigatorOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl StackDialog for NavigatorOptionsDialog {
    type Result = NavigatorOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    /// `Dialog` at `Width::Compact` with a `FormGrid` row Color (a `Select` with a swatch per
    /// option) and `Footer::standard()`; the list has the initial focus.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<NavigatorOptions>> {
        // new: as craft-ui crates/ui/src/dialog/guides.rs:586 `NewGuideDialog::show` (title, width, footer, outcome).
        todo!()
    }
}
