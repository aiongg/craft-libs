//! The appearance the viewer applies to every shown story, as the values `RootEnv` takes.

use craft_behavior::MotionMode;
use craft_i18n::Direction;
use craft_theme::{Contrast, ControlSize, Mode, Size, TextSize};
use craft_ui::contract::Entry;
use craft_ui::runtime::env::{RootEnv, Scale};

/// One option of a choice: the value, the name it is saved under and the label the options bar shows.
#[non_exhaustive]
pub struct Choice<T: 'static> {
    /// The value.
    pub value: T,
    /// The saved name.
    pub name: &'static str,
    /// The text the options bar shows.
    pub label: &'static str,
}

/// The options of one appearance field, in the order the options bar lists them.
#[non_exhaustive]
pub struct Choices<T: 'static>(pub &'static [Choice<T>]);

impl<T: Copy + PartialEq + 'static> Choices<T> {
    /// The value saved as `name`.
    pub fn parse(&self, name: &str) -> Option<T> {
        self.0.iter().find(|choice| choice.name == name).map(|choice| choice.value)
    }

    /// The name `value` is saved under.
    pub fn name(&self, value: T) -> &'static str {
        self.0.iter().find(|choice| choice.value == value).map_or("", |choice| choice.name)
    }

    /// The entries of a select or a segmented row over the options.
    pub fn entries(&self) -> Vec<Entry<T>> {
        self.0.iter().map(|choice| Entry::new(choice.value, choice.label)).collect()
    }
}

macro_rules! choices {
    ($name:ident: $ty:ty = [$(($value:expr, $saved:literal, $label:literal)),+ $(,)?]) => {
        /// The options of the field.
        pub const $name: Choices<$ty> = Choices(&[$(Choice { value: $value, name: $saved, label: $label }),+]);
    };
}

choices!(MODES: Mode = [(Mode::Light, "light", "Light"), (Mode::Dark, "dark", "Dark")]);
choices!(CONTRASTS: Contrast = [(Contrast::Standard, "standard", "Standard"), (Contrast::High, "high", "High")]);
choices!(TEXT_SIZES: TextSize = [
    (TextSize::Tiny, "tiny", "Tiny"),
    (TextSize::Small, "small", "Small"),
    (TextSize::Medium, "medium", "Medium"),
    (TextSize::Large, "large", "Large"),
]);
choices!(DENSITIES: Size = [(Size::Compact, "compact", "Compact"), (Size::Comfortable, "comfortable", "Comfortable")]);
choices!(MOTIONS: MotionMode = [(MotionMode::Full, "full", "Full"), (MotionMode::Reduced, "reduced", "Reduced"), (MotionMode::Off, "off", "Off")]);
choices!(SCALES: ScalePercent = [
    (ScalePercent(100), "100", "100%"),
    (ScalePercent(125), "125", "125%"),
    (ScalePercent(150), "150", "150%"),
    (ScalePercent(200), "200", "200%"),
]);
choices!(DIRECTIONS: Option<Direction> = [(None, "both", "Both"), (Some(Direction::Ltr), "ltr", "LTR"), (Some(Direction::Rtl), "rtl", "RTL")]);
choices!(SIZES: Option<ControlSize> = [(None, "both", "Both"), (Some(ControlSize::Sm), "sm", "Sm"), (Some(ControlSize::Md), "md", "Md")]);

/// The interface scale as a whole percentage, so it compares and saves exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScalePercent(pub u16);

impl ScalePercent {
    /// The scale the percentage stands for.
    pub fn scale(self) -> Scale {
        Scale::new(f32::from(self.0) / 100.0)
    }
}

/// What the viewer applies to the stories it shows. A story is shown under the root values of this appearance,
/// except the direction and control size, which each cell sets itself.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Appearance {
    /// Light or dark.
    pub mode: Mode,
    /// Standard or high contrast.
    pub contrast: Contrast,
    /// The text-size step.
    pub text_size: TextSize,
    /// The density preset.
    pub density: Size,
    /// The motion mode.
    pub motion: MotionMode,
    /// The interface scale.
    pub scale: ScalePercent,
    /// The direction the cells show; `None` shows both.
    pub direction: Option<Direction>,
    /// The control size the cells show; `None` shows both.
    pub size: Option<ControlSize>,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            mode: Mode::Light,
            contrast: Contrast::Standard,
            text_size: TextSize::default(),
            density: Size::default(),
            motion: MotionMode::default(),
            scale: ScalePercent(100),
            direction: None,
            size: None,
        }
    }
}

impl Appearance {
    /// The root values of the window.
    pub fn root_env(&self) -> RootEnv {
        let mut root = RootEnv::new().mode(self.mode).contrast(self.contrast).density(self.density).motion(self.motion).scale(self.scale.scale());
        root.text_size = self.text_size;
        root
    }
}
