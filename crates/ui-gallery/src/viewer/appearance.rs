//! The choices of the options bar: the named options of each root value, the theme list and the cell filters.

use craft_behavior::MotionMode;
use craft_i18n::Direction;
use craft_theme::{Contrast, ControlSize, Mode, Size, TextSize, ThemeDef, ThemeId, builtins};
use craft_ui::contract::Entry;
use craft_ui::runtime::env::Scale;

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
choices!(SCALES: Scale = [
    (Scale::new(1.0), "100", "100%"),
    (Scale::new(1.25), "125", "125%"),
    (Scale::new(1.5), "150", "150%"),
    (Scale::new(2.0), "200", "200%"),
]);
choices!(DIRECTIONS: Option<Direction> = [(None, "both", "Both"), (Some(Direction::Ltr), "ltr", "LTR"), (Some(Direction::Rtl), "rtl", "RTL")]);
choices!(SIZES: Option<ControlSize> = [(None, "both", "Both"), (Some(ControlSize::Sm), "sm", "Sm"), (Some(ControlSize::Md), "md", "Md")]);

/// Which cells of a story the viewer shows: the directions and control sizes it lays out. `None` shows both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CellFilter {
    /// The direction the cells show.
    pub direction: Option<Direction>,
    /// The control size the cells show.
    pub size: Option<ControlSize>,
}

/// The built-in theme saved as `name`.
pub fn theme_named(name: &str) -> Option<ThemeId> {
    builtins().iter().find(|theme| theme.id.as_str() == name).map(|theme| theme.id.clone())
}

/// The built-in theme `id` names; the suite default when `id` names none.
pub fn theme_of(id: &ThemeId) -> &'static ThemeDef {
    let themes = builtins();
    themes.iter().find(|theme| theme.id == *id).or_else(|| themes.iter().find(|theme| theme.id == ThemeId::suite_default())).unwrap_or(&themes[0])
}

/// The entries of the Theme select: every built-in theme under its name.
pub fn theme_entries() -> Vec<Entry<ThemeId>> {
    builtins().iter().map(|theme| Entry::new(theme.id.clone(), theme.name.clone())).collect()
}
