//! Plain value types the component styles in `craft-ui` build their looks from: sizes by control
//! size, colours by state, and the labelled colour sets of buttons, badges, checks and fields.
//! They hold no theme data of their own.

use crate::color::Rgba;
use crate::contrast::{contrast_ratio, ensure_contrast};
use crate::derive::over;
use crate::tokens::{ColorTokens, StatusKind};
use crate::values;

/// The two control sizes a family offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlSize {
    /// Small: toolbars, dense rows.
    Sm,
    /// Standard.
    Md,
}

/// One value per [`ControlSize`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BySize<T> {
    /// Small.
    pub sm: T,
    /// Standard.
    pub md: T,
}

impl<T: Copy> BySize<T> {
    /// The value for `size`.
    pub fn get(&self, size: ControlSize) -> T {
        match size {
            ControlSize::Sm => self.sm,
            ControlSize::Md => self.md,
        }
    }
}

/// An interaction state. Keyboard focus is not one: the `FocusRing` primitive draws it from
/// `line.focus`. Selected and checked belong to the families that have them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum State {
    /// Idle.
    Rest,
    /// Under the pointer.
    Hover,
    /// While pressed.
    Pressed,
    /// Not interactive.
    Disabled,
}

impl State {
    /// Every state, in path order.
    pub const ALL: [State; 4] = [State::Rest, State::Hover, State::Pressed, State::Disabled];

    /// The path segment, e.g. `"hover"`.
    pub const fn name(self) -> &'static str {
        match self {
            State::Rest => "rest",
            State::Hover => "hover",
            State::Pressed => "pressed",
            State::Disabled => "disabled",
        }
    }

    /// The state whose [`name`](State::name) is `name`.
    pub fn from_name(name: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// One value per [`State`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct States<T> {
    /// Idle.
    pub rest: T,
    /// Under the pointer.
    pub hover: T,
    /// While pressed.
    pub pressed: T,
    /// Not interactive.
    pub disabled: T,
}

impl<T: Copy> States<T> {
    /// The four values, in [`State::ALL`] order.
    pub const fn new(rest: T, hover: T, pressed: T, disabled: T) -> States<T> {
        States { rest, hover, pressed, disabled }
    }

    /// The value for `state`.
    pub fn get(&self, state: State) -> T {
        match state {
            State::Rest => self.rest,
            State::Hover => self.hover,
            State::Pressed => self.pressed,
            State::Disabled => self.disabled,
        }
    }

    /// The value for `state`, to change it.
    pub fn get_mut(&mut self, state: State) -> &mut T {
        match state {
            State::Rest => &mut self.rest,
            State::Hover => &mut self.hover,
            State::Pressed => &mut self.pressed,
            State::Disabled => &mut self.disabled,
        }
    }
}

/// One coloured part of an item without an outline: a segment, a toggled icon button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemPart {
    /// The background.
    Fill,
    /// Label and icon.
    Text,
}

impl ItemPart {
    /// Every part, in path order.
    pub const ALL: [ItemPart; 2] = [ItemPart::Fill, ItemPart::Text];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ItemPart::Fill => "fill",
            ItemPart::Text => "text",
        }
    }
}

/// The colours of an item without an outline, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemColors {
    /// Background.
    pub fill: States<Rgba>,
    /// Label and icon.
    pub text: States<Rgba>,
}

impl ItemColors {
    /// The colours of `part`.
    pub fn part(&self, part: ItemPart) -> &States<Rgba> {
        match part {
            ItemPart::Fill => &self.fill,
            ItemPart::Text => &self.text,
        }
    }

}

/// One coloured part of a knob that rides on a track: a slider or switch thumb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThumbPart {
    /// The face.
    Fill,
    /// The outline; a bevel keeps its own colours.
    Border,
}

impl ThumbPart {
    /// Every part, in path order.
    pub const ALL: [ThumbPart; 2] = [ThumbPart::Fill, ThumbPart::Border];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ThumbPart::Fill => "fill",
            ThumbPart::Border => "border",
        }
    }
}

/// The colours of a knob that rides on a track, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThumbColors {
    /// Face.
    pub fill: States<Rgba>,
    /// Outline.
    pub border: States<Rgba>,
}

impl ThumbColors {
    /// The colours of `part`.
    pub fn part(&self, part: ThumbPart) -> &States<Rgba> {
        match part {
            ThumbPart::Fill => &self.fill,
            ThumbPart::Border => &self.border,
        }
    }

}

/// A button's emphasis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonVariant {
    /// The main action: accent fill.
    Primary,
    /// Other actions: neutral fill and border.
    Secondary,
    /// Toolbar and inline actions: no fill until hovered.
    Ghost,
    /// Destructive actions: `status.bad` fill.
    Danger,
}

impl ButtonVariant {
    /// Every variant, in path order.
    pub const ALL: [ButtonVariant; 4] = [ButtonVariant::Primary, ButtonVariant::Secondary, ButtonVariant::Ghost, ButtonVariant::Danger];

    /// The path segment, e.g. `"primary"`.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Danger => "danger",
        }
    }
}

/// One coloured part of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonPart {
    /// The background.
    Fill,
    /// The outline.
    Border,
    /// Label, icon and spinner.
    Text,
}

impl ButtonPart {
    /// Every part, in path order.
    pub const ALL: [ButtonPart; 3] = [ButtonPart::Fill, ButtonPart::Border, ButtonPart::Text];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ButtonPart::Fill => "fill",
            ButtonPart::Border => "border",
            ButtonPart::Text => "text",
        }
    }
}

/// The colours of one variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonColors {
    /// Background.
    pub fill: States<Rgba>,
    /// Outline, `border_width` wide.
    pub border: States<Rgba>,
    /// Label, icon and spinner.
    pub text: States<Rgba>,
}

impl ButtonColors {
    /// The colours of `part`.
    pub fn part(&self, part: ButtonPart) -> &States<Rgba> {
        match part {
            ButtonPart::Fill => &self.fill,
            ButtonPart::Border => &self.border,
            ButtonPart::Text => &self.text,
        }
    }

}

/// What a badge says about its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BadgeTone {
    /// No status: a count, a tag.
    #[default]
    Neutral,
    /// The accent: new, selected.
    Accent,
    /// Success.
    Good,
    /// Needs attention.
    Warn,
    /// Failure.
    Bad,
    /// Information.
    Info,
}

impl BadgeTone {
    /// Every tone, in path order.
    pub const ALL: [BadgeTone; 6] = [BadgeTone::Neutral, BadgeTone::Accent, BadgeTone::Good, BadgeTone::Warn, BadgeTone::Bad, BadgeTone::Info];

    /// The path segment, e.g. `"good"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgeTone::Neutral => "neutral",
            BadgeTone::Accent => "accent",
            BadgeTone::Good => "good",
            BadgeTone::Warn => "warn",
            BadgeTone::Bad => "bad",
            BadgeTone::Info => "info",
        }
    }

    /// The tone whose [`name`](BadgeTone::name) is `name`.
    pub fn from_name(name: &str) -> Option<BadgeTone> {
        BadgeTone::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// How a badge fills its tone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BadgeStyle {
    /// The tone's tinted background, text and border.
    #[default]
    Soft,
    /// The tone's solid fill, no border.
    Solid,
}

impl BadgeStyle {
    /// Both styles, in path order.
    pub const ALL: [BadgeStyle; 2] = [BadgeStyle::Soft, BadgeStyle::Solid];

    /// The path segment, e.g. `"soft"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgeStyle::Soft => "soft",
            BadgeStyle::Solid => "solid",
        }
    }

    /// The style whose [`name`](BadgeStyle::name) is `name`.
    pub fn from_name(name: &str) -> Option<BadgeStyle> {
        BadgeStyle::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// One coloured part of a badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BadgePart {
    /// The background.
    Fill,
    /// The label.
    Text,
    /// The outline; a solid badge's is its fill.
    Border,
}

impl BadgePart {
    /// Every part, in path order.
    pub const ALL: [BadgePart; 3] = [BadgePart::Fill, BadgePart::Text, BadgePart::Border];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgePart::Fill => "fill",
            BadgePart::Text => "text",
            BadgePart::Border => "border",
        }
    }
}

/// The colours of one tone in one style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BadgeColors {
    /// Background.
    pub fill: Rgba,
    /// Label.
    pub text: Rgba,
    /// Outline.
    pub border: Rgba,
}

impl BadgeColors {
    /// The colour of `part`.
    pub fn part(&self, part: BadgePart) -> Rgba {
        match part {
            BadgePart::Fill => self.fill,
            BadgePart::Text => self.text,
            BadgePart::Border => self.border,
        }
    }

}

/// One coloured part of a check box, radio button or switch indicator while off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckOffPart {
    /// The inside of the box, circle or track.
    Fill,
    /// The outline of the box, circle or track, over a flat surface's border.
    Border,
}

impl CheckOffPart {
    /// Every part, in path order.
    pub const ALL: [CheckOffPart; 2] = [CheckOffPart::Fill, CheckOffPart::Border];

    /// The path segment, e.g. `"border"`.
    pub const fn name(self) -> &'static str {
        match self {
            CheckOffPart::Fill => "fill",
            CheckOffPart::Border => "border",
        }
    }
}

/// One coloured part of a check box, radio button or switch indicator while on (checked, mixed
/// or selected).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckOnPart {
    /// The inside of the box, circle or track.
    Fill,
    /// The outline of the box, circle or track, over a flat surface's border.
    Border,
    /// The check or dash, the radio dot. A switch draws none.
    Mark,
}

impl CheckOnPart {
    /// Every part, in path order.
    pub const ALL: [CheckOnPart; 3] = [CheckOnPart::Fill, CheckOnPart::Border, CheckOnPart::Mark];

    /// The path segment, e.g. `"mark"`.
    pub const fn name(self) -> &'static str {
        match self {
            CheckOnPart::Fill => "fill",
            CheckOnPart::Border => "border",
            CheckOnPart::Mark => "mark",
        }
    }
}

/// The colours of the indicator while off, per state. Off has no mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckOffColors {
    /// Inside of the box, circle or track.
    pub fill: States<Rgba>,
    /// Outline of the box, circle or track.
    pub border: States<Rgba>,
}

impl CheckOffColors {
    /// The colours of `part`.
    pub fn part(&self, part: CheckOffPart) -> &States<Rgba> {
        match part {
            CheckOffPart::Fill => &self.fill,
            CheckOffPart::Border => &self.border,
        }
    }

}

/// The colours of the indicator and its mark while on, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckOnColors {
    /// Inside of the box, circle or track.
    pub fill: States<Rgba>,
    /// Outline of the box, circle or track.
    pub border: States<Rgba>,
    /// Check, dash or dot.
    pub mark: States<Rgba>,
}

impl CheckOnColors {
    /// The colours of `part`.
    pub fn part(&self, part: CheckOnPart) -> &States<Rgba> {
        match part {
            CheckOnPart::Fill => &self.fill,
            CheckOnPart::Border => &self.border,
            CheckOnPart::Mark => &self.mark,
        }
    }

}

/// One coloured part of a switch track while on.
pub type CheckSwitchPart = CheckOffPart;

/// The colours of a switch track while on, per state. A switch draws no mark.
pub type CheckSwitchColors = CheckOffColors;

/// A part of a field that has one colour while enabled and one while disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldPart {
    /// The inside of the box.
    Fill,
    /// The typed text.
    Text,
    /// The text shown while the field is empty.
    Placeholder,
    /// A number's unit, drawn at the box's end.
    Unit,
    /// The label or icon at a number field's start that a drag scrubs the value with.
    Scrub,
}

impl FieldPart {
    /// Every part, in path order.
    pub const ALL: [FieldPart; 5] = [FieldPart::Fill, FieldPart::Text, FieldPart::Placeholder, FieldPart::Unit, FieldPart::Scrub];

    /// The path segment, e.g. `"placeholder"`.
    pub const fn name(self) -> &'static str {
        match self {
            FieldPart::Fill => "fill",
            FieldPart::Text => "text",
            FieldPart::Placeholder => "placeholder",
            FieldPart::Unit => "unit",
            FieldPart::Scrub => "scrub",
        }
    }
}

/// The state of a field's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BorderState {
    /// Idle.
    Rest,
    /// Under the pointer.
    Hover,
    /// With keyboard focus, however it got there.
    Focused,
    /// Not interactive.
    Disabled,
}

impl BorderState {
    /// Every state, in path order.
    pub const ALL: [BorderState; 4] = [BorderState::Rest, BorderState::Hover, BorderState::Focused, BorderState::Disabled];

    /// The path segment, e.g. `"focused"`.
    pub const fn name(self) -> &'static str {
        match self {
            BorderState::Rest => "rest",
            BorderState::Hover => "hover",
            BorderState::Focused => "focused",
            BorderState::Disabled => "disabled",
        }
    }
}

/// A colour while enabled and while disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dimmed {
    /// Enabled, in every state.
    pub rest: Rgba,
    /// Disabled.
    pub disabled: Rgba,
}

impl Dimmed {
    /// The colour while enabled or not.
    pub fn get(&self, enabled: bool) -> Rgba {
        if enabled { self.rest } else { self.disabled }
    }

}

/// The colours of a field's edge, per [`BorderState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BorderColors {
    /// Idle.
    pub rest: Rgba,
    /// Under the pointer.
    pub hover: Rgba,
    /// With keyboard focus.
    pub focused: Rgba,
    /// Not interactive.
    pub disabled: Rgba,
}

impl BorderColors {
    /// The colour in `state`.
    pub fn get(&self, state: BorderState) -> Rgba {
        match state {
            BorderState::Rest => self.rest,
            BorderState::Hover => self.hover,
            BorderState::Focused => self.focused,
            BorderState::Disabled => self.disabled,
        }
    }

}

/// The gaps and label column of one density of form grid, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FormSpacing {
    /// Between the label column and the control column: `form.column_gap`.
    pub column_gap: f32,
    /// Between two rows: `form.row_gap`.
    pub row_gap: f32,
    /// The least width of the label column; a wider label widens it: `form.label_min`.
    pub label_min: f32,
}

/// PhotoCraft's layer label colours (`LabelColor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LabelColor {
    #[allow(missing_docs)]
    Red,
    #[allow(missing_docs)]
    Orange,
    #[allow(missing_docs)]
    Yellow,
    #[allow(missing_docs)]
    Green,
    #[allow(missing_docs)]
    Seafoam,
    #[allow(missing_docs)]
    Blue,
    #[allow(missing_docs)]
    Indigo,
    #[allow(missing_docs)]
    Magenta,
    #[allow(missing_docs)]
    Fuchsia,
    #[allow(missing_docs)]
    Violet,
    #[allow(missing_docs)]
    Gray,
}

impl LabelColor {
    /// Every label colour, in path order.
    pub const ALL: [LabelColor; 11] = [
        LabelColor::Red,
        LabelColor::Orange,
        LabelColor::Yellow,
        LabelColor::Green,
        LabelColor::Seafoam,
        LabelColor::Blue,
        LabelColor::Indigo,
        LabelColor::Magenta,
        LabelColor::Fuchsia,
        LabelColor::Violet,
        LabelColor::Gray,
    ];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            LabelColor::Red => "red",
            LabelColor::Orange => "orange",
            LabelColor::Yellow => "yellow",
            LabelColor::Green => "green",
            LabelColor::Seafoam => "seafoam",
            LabelColor::Blue => "blue",
            LabelColor::Indigo => "indigo",
            LabelColor::Magenta => "magenta",
            LabelColor::Fuchsia => "fuchsia",
            LabelColor::Violet => "violet",
            LabelColor::Gray => "gray",
        }
    }

}

/// PhotoCraft's label colours (`menu_tint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MenuHue {
    #[allow(missing_docs)]
    Red,
    #[allow(missing_docs)]
    Orange,
    #[allow(missing_docs)]
    Yellow,
    #[allow(missing_docs)]
    Green,
    #[allow(missing_docs)]
    Blue,
    #[allow(missing_docs)]
    Violet,
    #[allow(missing_docs)]
    Gray,
}

impl MenuHue {
    /// Every hue, in path order.
    pub const ALL: [MenuHue; 7] = [MenuHue::Red, MenuHue::Orange, MenuHue::Yellow, MenuHue::Green, MenuHue::Blue, MenuHue::Violet, MenuHue::Gray];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            MenuHue::Red => "red",
            MenuHue::Orange => "orange",
            MenuHue::Yellow => "yellow",
            MenuHue::Green => "green",
            MenuHue::Blue => "blue",
            MenuHue::Violet => "violet",
            MenuHue::Gray => "gray",
        }
    }

    /// The label colour a row's tint is derived from.
    pub const fn color(self) -> Rgba {
        match self {
            MenuHue::Red => Rgba::rgb(190, 60, 60),
            MenuHue::Orange => Rgba::rgb(200, 120, 40),
            MenuHue::Yellow => Rgba::rgb(190, 170, 40),
            MenuHue::Green => Rgba::rgb(60, 150, 70),
            MenuHue::Blue => Rgba::rgb(50, 110, 200),
            MenuHue::Violet => Rgba::rgb(130, 80, 190),
            MenuHue::Gray => Rgba::rgb(120, 120, 120),
        }
    }

    /// The hue whose [`color`](Self::color) is `color`'s red, green and blue.
    pub fn of(color: Rgba) -> Option<MenuHue> {
        Self::ALL.into_iter().find(|h| h.color().0[..3] == color.0[..3])
    }

}

/// The colours of a row with a label colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuTint {
    /// Behind the row: the label colour over `surface.raised`, opaque.
    pub fill: Rgba,
    /// The label, icon, check or radio mark and submenu arrow.
    pub text: Rgba,
    /// The shortcut.
    pub shortcut: Rgba,
}

impl MenuTint {
    /// The tint of `color` (its alpha scales the strength) for a menu on the popup card `base`
    /// with the text colours of `c`. It starts at `values::MENU_TINT_ALPHA` of the colour; `text.normal` and `text.muted`
    /// then move in OKLCH lightness until they reach the text minimum (4.5:1, 7:1 with `high`)
    /// on the fill. When they can't, the alpha drops by 0.01 until they do; at 0 the fill is the
    /// card itself. The check and radio marks and the arrow use the label colour, which has
    /// the text minimum, more than the 3:1 (4.5:1) that marks need.
    pub fn derive(color: Rgba, base: Rgba, c: &ColorTokens, high: bool) -> MenuTint {
        let min = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
        let start = f64::from(values::MENU_TINT_ALPHA) * f64::from(color.a()) / 255.0;
        let steps = (values::MENU_TINT_ALPHA * 100.0).round() as u32;
        let mut tint = MenuTint { fill: base, text: c.text.normal, shortcut: c.text.muted };
        for k in (0..=steps).rev() {
            let alpha = start * f64::from(k) / f64::from(steps);
            let fill = over(Rgba([color.0[0], color.0[1], color.0[2], (alpha * 255.0).round() as u8]), base);
            tint = MenuTint { fill, text: ensure_contrast(c.text.normal, fill, min), shortcut: ensure_contrast(c.text.muted, fill, min) };
            if contrast_ratio(tint.text, fill) >= min && contrast_ratio(tint.shortcut, fill) >= min {
                break;
            }
        }
        tint
    }
}

/// What a notice or toast says about its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Tone {
    /// No status: a plain message.
    #[default]
    Neutral,
    /// Information.
    Info,
    /// Success.
    Good,
    /// Needs attention.
    Warn,
    /// Failure.
    Bad,
}

impl Tone {
    /// Every tone, in path order.
    pub const ALL: [Tone; 5] = [Tone::Neutral, Tone::Info, Tone::Good, Tone::Warn, Tone::Bad];

    /// The path segment, e.g. `"good"`.
    pub const fn name(self) -> &'static str {
        match self {
            Tone::Neutral => "neutral",
            Tone::Info => "info",
            Tone::Good => "good",
            Tone::Warn => "warn",
            Tone::Bad => "bad",
        }
    }

    /// The tone whose [`name`](Tone::name) is `name`.
    pub fn from_name(name: &str) -> Option<Tone> {
        Tone::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The status the tone reads its colours from; `None` for neutral.
    pub const fn status(self) -> Option<StatusKind> {
        match self {
            Tone::Neutral => None,
            Tone::Info => Some(StatusKind::Info),
            Tone::Good => Some(StatusKind::Good),
            Tone::Warn => Some(StatusKind::Warn),
            Tone::Bad => Some(StatusKind::Bad),
        }
    }

}

