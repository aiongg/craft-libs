//! Panel Options as data: `NavigatorSettings`, `NavigatorOptions` and `ProxyColor`. The dialog that edits them is
//! `patterns::form_dialogs::NavigatorOptionsDialog`.

use crate::canvas::CanvasWords;
use crate::runtime::env::Label;
use craft_theme::Srgba;
use egui::Color32;

/// The `ColorRequest::key` of Panel Options' Custom… colour.
pub const NAVIGATOR_PROXY_COLOR_KEY: &str = "navigatorOptionsDialog.proxyColor";

/// The proxy rectangle's colour (Panel Options, Color).
///
/// `Default` is the light red of the proxy in Photoshop's Navigator; the named colours are fixed data, the same in every
/// theme, chosen to read on the dark preview box; `Custom` is a colour from the app's picker, kept as 8-bit sRGB.
/// Serialised by variant name, `Custom` as `{ Custom = [r, g, b] }`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ProxyColor {
    /// The default proxy colour.
    #[default]
    Default,
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Violet.
    Violet,
    /// Gray.
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

    /// The colour Custom… opens its picker at before a custom colour was chosen: the default proxy colour.
    pub const DEFAULT_CUSTOM: [u8; 3] = [255, 84, 84];

    /// The colour's name in the dialog; `Custom` is the word for Custom….
    pub fn label(self, words: &CanvasWords) -> Label {
        let word = match self {
            ProxyColor::Default => words.proxy_default.clone(),
            ProxyColor::Red => words.proxy_red.clone(),
            ProxyColor::Orange => words.proxy_orange.clone(),
            ProxyColor::Yellow => words.proxy_yellow.clone(),
            ProxyColor::Green => words.proxy_green.clone(),
            ProxyColor::Blue => words.proxy_blue.clone(),
            ProxyColor::Violet => words.proxy_violet.clone(),
            ProxyColor::Gray => words.proxy_gray.clone(),
            ProxyColor::Custom(_) => words.proxy_custom.clone(),
        };
        word.into()
    }

    /// The colour as 8-bit sRGB: `Default` is the default proxy colour (255, 84, 84); red is 255, 84, 84; orange 255,
    /// 160, 60; yellow 255, 214, 64; green 92, 214, 110; blue 84, 160, 255; violet 176, 124, 255; gray 200, 200, 200;
    /// `Custom` is its own value.
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            ProxyColor::Default | ProxyColor::Red => [255, 84, 84],
            ProxyColor::Orange => [255, 160, 60],
            ProxyColor::Yellow => [255, 214, 64],
            ProxyColor::Green => [92, 214, 110],
            ProxyColor::Blue => [84, 160, 255],
            ProxyColor::Violet => [176, 124, 255],
            ProxyColor::Gray => [200, 200, 200],
            ProxyColor::Custom(rgb) => rgb,
        }
    }

    /// The colour drawn.
    pub fn color(self) -> Srgba {
        let [r, g, b] = self.rgb();
        Srgba::from(Color32::from_rgb(r, g, b))
    }

    /// The `Custom` colour for a picker's answer, rounded to 8 bits.
    pub fn custom(color: Srgba) -> Self {
        let color = Color32::from(color);
        ProxyColor::Custom([color.r(), color.g(), color.b()])
    }
}

/// The Navigator's options, the value Panel Options edits and answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct NavigatorOptions {
    /// The proxy rectangle's colour.
    pub proxy_color: ProxyColor,
}

impl NavigatorOptions {
    /// Options with the proxy drawn in `proxy_color`.
    pub fn new(proxy_color: ProxyColor) -> Self {
        NavigatorOptions { proxy_color }
    }
}

/// The Navigator's persisted section: the app keeps it with its preferences. It holds the dialog's
/// [`NavigatorOptions`] flattened, so the document reads `proxyColor = "Red"`. Unknown fields are ignored and missing
/// ones take their defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct NavigatorSettings {
    /// The Panel Options values.
    #[serde(flatten)]
    pub options: NavigatorOptions,
}

impl NavigatorSettings {
    /// Settings that hold `options`.
    pub fn new(options: NavigatorOptions) -> Self {
        NavigatorSettings { options }
    }
}
