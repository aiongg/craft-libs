//! [`LabelColorLabels`]: the names of the label colours, for the colour menus of Layers and
//! Actions.

use craft_theme::LabelColor;

crate::words! {
    /// The names of the label colours and of no colour, English by default.
    #[non_exhaustive]
    pub struct LabelColorLabels {
        none: "labelColor.none" = "None";
        red: "labelColor.red" = "Red";
        orange: "labelColor.orange" = "Orange";
        yellow: "labelColor.yellow" = "Yellow";
        green: "labelColor.green" = "Green";
        seafoam: "labelColor.seafoam" = "Seafoam";
        blue: "labelColor.blue" = "Blue";
        indigo: "labelColor.indigo" = "Indigo";
        magenta: "labelColor.magenta" = "Magenta";
        fuchsia: "labelColor.fuchsia" = "Fuchsia";
        violet: "labelColor.violet" = "Violet";
        gray: "labelColor.gray" = "Gray";
    }
}

impl LabelColorLabels {
    /// The name of `color`; `None` is "None".
    pub fn name(&self, color: Option<LabelColor>) -> &str {
        match color {
            None => &self.none,
            Some(LabelColor::Red) => &self.red,
            Some(LabelColor::Orange) => &self.orange,
            Some(LabelColor::Yellow) => &self.yellow,
            Some(LabelColor::Green) => &self.green,
            Some(LabelColor::Seafoam) => &self.seafoam,
            Some(LabelColor::Blue) => &self.blue,
            Some(LabelColor::Indigo) => &self.indigo,
            Some(LabelColor::Magenta) => &self.magenta,
            Some(LabelColor::Fuchsia) => &self.fuchsia,
            Some(LabelColor::Violet) => &self.violet,
            Some(LabelColor::Gray) => &self.gray,
        }
    }
}
