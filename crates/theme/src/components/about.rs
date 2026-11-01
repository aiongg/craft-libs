//! About windows: the window width, the Contributors list height and the grab bag. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// About tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AboutTokens {
    /// Width of the window and of its tabs' bodies: `about.width`.
    pub width: f32,
    /// Height of the Contributors list, header included: `about.list_height`.
    pub list_height: f32,
    /// Horizontal and vertical gap between the names of the grab bag: `about.bag_gap`.
    pub bag_gap: [f32; 2],
    /// Width of the community button: `about.community_width`.
    pub community_width: f32,
    /// Least width of the flexible Name and Model columns: `about.flex_min`.
    pub flex_min: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AboutField {}

impl Family for AboutTokens {
    type Field = AboutField;
    const FIELDS: &'static [AboutField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AboutTokens;
        "width" 0.0..=4096.0 => width;
        "list_height" 0.0..=4096.0 => list_height;
        "bag_gap.0" 0.0..=256.0 => bag_gap[0];
        "bag_gap.1" 0.0..=256.0 => bag_gap[1];
        "community_width" 0.0..=1024.0 => community_width;
        "flex_min" 0.0..=1024.0 => flex_min;
    };

    fn defaults(_: &Semantic<'_>) -> AboutTokens {
        AboutTokens {
            width: values::ABOUT_WIDTH,
            list_height: values::ABOUT_LIST_HEIGHT,
            bag_gap: values::ABOUT_BAG_GAP,
            community_width: values::ABOUT_COMMUNITY_WIDTH,
            flex_min: values::ABOUT_FLEX_MIN,
        }
    }

    fn path(f: AboutField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AboutField> {
        None
    }

    fn kind(f: AboutField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AboutField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AboutField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
