//! Panel Options: [`NavigatorSettings`], [`NavigatorOptions`], [`ProxyColor`] and
//! [`NavigatorOptionsDialog`].

use craft_theme::{Rgba, Srgba};
use egui::{Color32, Context, Ui};

use super::labels::NavigatorLabels;
use crate::composites::FormGrid;
use crate::controls::{Select, Swatch};
use crate::dialog::{ColorRequest, Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width, dialog_id};
use crate::localized::SharedLabels as _;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};

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
    pub fn label(self, labels: &NavigatorLabels) -> &str {
        match self {
            ProxyColor::Default => &labels.default_color,
            ProxyColor::Red => &labels.red,
            ProxyColor::Orange => &labels.orange,
            ProxyColor::Yellow => &labels.yellow,
            ProxyColor::Green => &labels.green,
            ProxyColor::Blue => &labels.blue,
            ProxyColor::Violet => &labels.violet,
            ProxyColor::Gray => &labels.gray,
            ProxyColor::Custom(_) => &labels.custom,
        }
    }

    /// The colour drawn: `Default` reads `navigator.proxy` from `tokens`; the named ones are the
    /// lead's data (red 255/84/84, orange 255/160/60, yellow 255/214/64, green 92/214/110, blue
    /// 84/160/255, violet 176/124/255, gray 200/200/200), moved into `values.rs` by TK2; `Custom`
    /// is its own value.
    pub fn color(self, tokens: &UiTokens) -> Color32 {
        let navigator = &tokens.component.navigator;
        let named = |index: usize| navigator.proxy_colors[index].to_egui();
        match self {
            ProxyColor::Default => navigator.proxy.to_egui(),
            ProxyColor::Red => named(0),
            ProxyColor::Orange => named(1),
            ProxyColor::Yellow => named(2),
            ProxyColor::Green => named(3),
            ProxyColor::Blue => named(4),
            ProxyColor::Violet => named(5),
            ProxyColor::Gray => named(6),
            ProxyColor::Custom([r, g, b]) => Color32::from_rgb(r, g, b),
        }
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
    pub fn new() -> Self {
        Self {
            id: dialog_id(OPTIONS_DIALOG),
            options: NavigatorOptions::default(),
            custom: DEFAULT_CUSTOM,
            color_request: None,
            before_custom: None,
            state: DialogState::new(),
        }
    }

    /// Opens with `options` (the app's current settings).
    pub fn initial(mut self, options: NavigatorOptions) -> Self {
        self.options = options;
        if let ProxyColor::Custom(rgb) = options.proxy_color {
            self.custom = rgb;
        }
        self
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
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key != NAVIGATOR_PROXY_COLOR_KEY {
            return;
        }
        let [r, g, b, _] = color.to_rgba().0;
        self.custom = [r, g, b];
        self.options.proxy_color = ProxyColor::Custom(self.custom);
        self.before_custom = None;
        self.color_request = None;
    }

    /// The picker was cancelled: the colour goes back to what it was before Custom….
    pub fn cancel_color(&mut self) {
        if let Some(before) = self.before_custom.take() {
            self.options.proxy_color = before;
        }
        self.color_request = None;
    }

    /// Chooses Custom…: the request for the picker, opening at the last custom colour (or the
    /// colour already custom).
    fn ask_color(&mut self, title: &str) {
        if self.before_custom.is_none() {
            self.before_custom = Some(self.options.proxy_color);
        }
        let rgb = match self.options.proxy_color {
            ProxyColor::Custom(rgb) => rgb,
            _ => self.custom,
        };
        self.options.proxy_color = ProxyColor::Custom(rgb);
        let [r, g, b] = rgb;
        self.color_request = Some(ColorRequest::new(NAVIGATOR_PROXY_COLOR_KEY, Rgba::rgb(r, g, b).into()).title(title));
    }

    /// The Color row: a list of [`ProxyColor::ALL`] with a swatch each, then Custom….
    fn color_row(&mut self, ui: &mut Ui, labels: &NavigatorLabels) -> egui::Response {
        let tokens = Tokens::of(ui);
        let side = Swatch::style(&tokens, craft_theme::ControlSize::Md).side;
        let mut choice = self.options.proxy_color;
        let mut select = Select::new("navigator-proxy-color", labels.proxy_color.as_str(), &mut choice).row_content(
            egui::vec2(side, side),
            move |painter, rect, value| {
                painter.rect_filled(rect, 0.0, value.color(&tokens));
            },
        );
        for named in ProxyColor::ALL {
            select = select.option(named, named.label(labels));
        }
        select = select.option(ProxyColor::Custom(self.custom), labels.custom.as_str());
        let response = select.show(ui).response;
        let mut asks = false;
        if choice != self.options.proxy_color {
            match choice {
                ProxyColor::Custom(_) => asks = true,
                named => self.options.proxy_color = named,
            }
        }
        if let ProxyColor::Custom(rgb) = self.options.proxy_color {
            let [r, g, b] = rgb;
            // A click on the swatch asks again, also while the choice is already Custom….
            asks |= ui.add(Swatch::new("navigator-proxy-swatch", labels.custom.as_str(), Rgba::rgb(r, g, b))).clicked();
        }
        if asks {
            self.ask_color(&labels.picker_title);
        }
        response
    }
}

/// The dialog's key.
const OPTIONS_DIALOG: &str = "navigator.panelOptions";

/// The colour Custom… starts at before one was chosen: the default proxy red.
const DEFAULT_CUSTOM: [u8; 3] = [255, 84, 84];

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
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<NavigatorOptions>> {
        let labels = NavigatorLabels::shared_ctx(ctx);
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::new(OPTIONS_DIALOG, labels.options_title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            &mut state,
            &mut (),
            |ui, ()| {
                FormGrid::new("navigator-options").show(ui, |form| {
                    form.row(labels.proxy_color.as_str(), |ui, _| self.color_row(ui, &labels));
                });
            },
        );
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.options)),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}
