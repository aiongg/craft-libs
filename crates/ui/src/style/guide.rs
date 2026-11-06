//! Style of the `guide` family.

const DOT_RADIUS: f32 = 0.6;
const MEASURE_CAP: f32 = 4.0;

/// Guide values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideStyle {
    /// Radius of a dotted guide's dots.
    pub dot_radius: f32,
    /// Length of the end caps of a smart guide's distance.
    pub measure_cap: f32,
    /// Width of Guide Layout's Number field.
    pub number_width: f32,
    /// Width of its Width and Height fields.
    pub size_width: f32,
    /// Width of the field in each of its Margin sides.
    pub margin_width: f32,
    /// Width of New Guide's Position field.
    pub position_width: f32,
    /// Width of New Guide's Color list.
    pub color_width: f32,
    /// Width of Guide Layout's Preset and Target lists.
    pub layout_list_width: f32,
    /// Width of Guide Layout's Color list.
    pub layout_color_width: f32,
}

impl GuideStyle {
    /// The guide style for `tokens`.
    pub fn new(tokens: &craft_theme::Tokens) -> Self {
        let control = tokens.size.control.md;
        Self {
            dot_radius: DOT_RADIUS,
            measure_cap: MEASURE_CAP,
            number_width: control * 2.25,
            size_width: control * 2.0,
            margin_width: control * 2.25,
            position_width: control * 4.0,
            color_width: control * 4.5,
            layout_list_width: control * 7.5,
            layout_color_width: control * 5.75,
        }
    }
}
