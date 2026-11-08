//! `Checkbox`, `Switch`, `RadioGroup`, `Segmented` and `Rating`.

pub mod check;
pub mod radio;
mod rating;
mod segmented;
mod switch;
mod words;

mod swatch;

pub use check::{Checkbox, CheckboxStyle};
pub use radio::{GroupStyle, Radio, RadioGroup, RadioKind, RadioStyle};
pub use rating::{Rating, RatingStyle};
pub use segmented::{Segmented, SegmentedStyle};
pub use switch::{Switch, SwitchStyle};
pub use words::ChoiceWords;

pub use swatch::{Swatch, SwatchFace, SwatchMetrics, SwatchOutput, SwatchStyle};
