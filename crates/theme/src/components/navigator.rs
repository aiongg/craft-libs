//! The Navigator: the preview box, the proxy rectangle and the zoom row's spacing. The box fill,
//! its corner radius and the halo's colour are the canvas surface, `shape.radius.control` and
//! `overlay.halo`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Navigator tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NavigatorTokens {
    /// The proxy rectangle's colour when the app sets none: `navigator.proxy`
    /// ([`values::NAVIGATOR_PROXY`]). A design target with no contrast minimum, since it is
    /// drawn over any picture.
    pub proxy: Rgba,
    /// The proxy's stroke width: [`values::NAVIGATOR_PROXY_WIDTH`].
    pub proxy_width: f32,
    /// Width of the halo outline under the proxy, also the margin its edges are clipped to:
    /// [`values::NAVIGATOR_HALO`].
    pub halo: f32,
    /// The proxy's corner radius on an upright view: [`values::NAVIGATOR_PROXY_RADIUS`].
    pub proxy_radius: f32,
    /// Space between the preview box and the fitted document: [`values::NAVIGATOR_INSET`].
    pub inset: f32,
    /// The least height of the preview box: [`values::NAVIGATOR_MIN_PREVIEW`].
    pub min_preview: f32,
    /// Side of a checkerboard square behind transparent pixels: [`values::NAVIGATOR_CHECKER`].
    pub checker: f32,
    /// The gap above the zoom row and between its controls: [`values::NAVIGATOR_ROW_GAP`].
    pub row_gap: f32,
    /// The named proxy colours the options dialog lists after Default, in order: red, orange,
    /// yellow, green, blue, violet, gray ([`values::NAVIGATOR_PROXY_COLORS`]). The same in every
    /// theme.
    pub proxy_colors: [Rgba; 7],
}

/// An overridable Navigator field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NavigatorField {
    /// `navigator.proxy`
    Proxy,
}

impl Family for NavigatorTokens {
    type Field = NavigatorField;
    const FIELDS: &'static [NavigatorField] = &[NavigatorField::Proxy];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { NavigatorTokens;
        "proxy_width" 0.5..=8.0 => proxy_width;
        "halo" 0.0..=4.0 => halo;
        "proxy_radius" 0.0..=16.0 => proxy_radius;
        "inset" 0.0..=32.0 => inset;
        "min_preview" 16.0..=240.0 => min_preview;
        "checker" 1.0..=32.0 => checker;
        "row_gap" 0.0..=32.0 => row_gap;
    };

    fn defaults(_: &Semantic<'_>) -> NavigatorTokens {
        NavigatorTokens {
            proxy: values::NAVIGATOR_PROXY,
            proxy_width: values::NAVIGATOR_PROXY_WIDTH,
            halo: values::NAVIGATOR_HALO,
            proxy_radius: values::NAVIGATOR_PROXY_RADIUS,
            inset: values::NAVIGATOR_INSET,
            min_preview: values::NAVIGATOR_MIN_PREVIEW,
            checker: values::NAVIGATOR_CHECKER,
            row_gap: values::NAVIGATOR_ROW_GAP,
            proxy_colors: values::NAVIGATOR_PROXY_COLORS,
        }
    }

    fn path(f: NavigatorField) -> String {
        match f {
            NavigatorField::Proxy => "proxy",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<NavigatorField> {
        Self::FIELDS.iter().copied().find(|f| Self::path(*f) == path)
    }

    fn kind(_: NavigatorField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: NavigatorField) -> OverrideValue {
        match f {
            NavigatorField::Proxy => OverrideValue::Color(self.proxy),
        }
    }

    fn set(&mut self, f: NavigatorField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let (NavigatorField::Proxy, OverrideValue::Color(c)) = (f, v) {
            self.proxy = c;
        }
    }

    /// The proxy is drawn over the user's picture, so it has no background to pair with.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
