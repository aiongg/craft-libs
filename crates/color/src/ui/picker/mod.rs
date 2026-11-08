//! `ColorPicker`: the colour picker's body, the area and strip with the channel rows, the hex field and the hooks a
//! host adds.

mod channel;
mod compact;
mod full;
mod layout;
#[cfg(feature = "screen-testing")]
mod stories;
mod value;

use std::sync::{Arc, Mutex};

use craft_behavior::{EditEvent, Flow, GestureId, Metrics, Mixed, Sense};
use craft_theme::{ColorModel, ModelValues, Srgba, builtin_models};
use craft_ui::choice::RadioKind;
use craft_ui::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, GroupHandle, GroupState, MeasureCx, Outcome, PaintUi, Pointer as _, Props as _,
    Remember as _, Role, WordSetCx as _,
};
use craft_ui::runtime::env::AppValues;
use craft_ui::runtime::focus::{self, FocusTarget};
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Pos2, Response, Vec2};

use super::{AreaMode, channels};
pub(crate) use channel::{ChannelRow, ChannelRule, ChannelUnit, channel_name};
pub use full::{ColorWarning, ColorWarningChip, ColorWarningChipOutput};
use layout::Sizes;
pub use value::ColorConstraint;
use value::Rules;

/// What a [`ColorPicker`] reports: the colour, and the values of the model the user edited it in while they still
/// give it, so a host can keep a CMYK colour CMYK.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PickedColor {
    /// Encoded sRGB with straight alpha, after the host's [`constrain`](ColorPicker::constrain).
    pub color: Srgba,
    /// The values of the model edited, as the user set them: a typed Lab or CMYK colour outside sRGB stays as typed.
    /// `None` for a colour set as such (hex, a sample) and for one the constraint moved away from its values.
    pub model: Option<ModelValues>,
}

impl PickedColor {
    /// A colour set as such (hex, a sample): `color` is encoded sRGB with straight alpha and no model values stay
    /// with it.
    pub fn rgb(color: Srgba) -> PickedColor {
        PickedColor { color, model: None }
    }

    /// A colour picked in the model of `values`, which stay with it as typed. `color` is what `values` give in
    /// encoded sRGB; the caller converts it, as the model decides.
    pub fn in_model(color: Srgba, values: ModelValues) -> PickedColor {
        PickedColor { color, model: Some(values) }
    }

    /// A colour picked as CMYK `inks` in percent (cyan, magenta, yellow, black, each 0 to 100, the units of the
    /// picker's fields). `color` is the sRGB the inks convert to under the host's CMYK converter.
    pub fn cmyk(color: Srgba, inks: [f32; 4]) -> PickedColor {
        PickedColor::in_model(color, ModelValues::new("cmyk", &inks))
    }
}

/// How a [`ColorPicker`] lays out its parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorPickerLayout {
    /// The Color panel and compact popup: the area over the channel slider rows of one model, a model switch, the hex
    /// row and the optional alpha row.
    #[default]
    Compact,
    /// The Color Picker dialog: the square area and the strip, the compare swatch and the host's warnings beside them
    /// over the channel grid, every model's channels at once with radios choosing what the area shows, and the hex
    /// field.
    Full,
}

/// The screen-pick button's press, for the host that samples the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScreenPickGesture {
    /// The button was clicked: the host starts sampling and answers with [`ColorPicker::sampled`].
    Clicked,
    /// The button is held and the pointer moved to `at`: Photoshop's press and drag.
    #[non_exhaustive]
    Held {
        /// The pointer, in the picker's points.
        at: Pos2,
    },
    /// The held button was released at `at`.
    #[non_exhaustive]
    Released {
        /// The pointer, in the picker's points.
        at: Pos2,
    },
}

/// What [`ColorPicker`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct ColorPickerOutput {
    /// The picker's edit.
    pub edit: EditResponse<PickedColor>,
    /// The screen-pick button's press this pass.
    pub screen_pick: Option<ScreenPickGesture>,
}

impl Outcome for ColorPickerOutput {
    type Value = PickedColor;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.edit.story_facts()
    }
}

craft_ui::words! {
    /// The picker's words, English by default. The words of the colour models and their channels are the models'.
    #[non_exhaustive]
    pub struct ColorPickerWords {
        /// The opacity row's name.
        opacity: "colorPicker.opacity" = "Opacity";
        /// The hex field's accessible name.
        hex: "colorPicker.hex" = "Hex color";
        /// The model switch's accessible name; `{model}` is the model's name.
        color_model: template "colorPicker.colorModel" = "Color model: {model}";
        /// The Full layout's radios.
        field_shows: "colorPicker.fieldShows" = "Field shows";
        /// The caption over the compare swatch's new half.
        new_caption: "colorPicker.newCaption" = "new";
        /// The caption under the compare swatch's current half.
        current_caption: "colorPicker.currentCaption" = "current";
        /// The new half's accessible name.
        new_color: "colorPicker.newColor" = "New color";
        /// The current half's accessible name and tooltip.
        restore: "colorPicker.restore" = "Restore current color";
        /// A channel's label in the Full grid; `{label}` is the channel's short label.
        grid_label: template "colorPicker.gridLabel" = "{label}:";
        /// The check box under the Full strip.
        only_web_colors: "colorPicker.onlyWebColors" = "Only Web Colors";
        /// The screen-pick button's name and tooltip.
        sample_screen: "colorPicker.sampleScreen" = "Sample color from the screen";
    }
}

/// The style of a colour picker: the sizes its layouts are made of.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ColorPickerStyle {
    #[style(metrics)]
    metrics: Metrics,
    sizes: Sizes,
}

/// What the picker keeps between passes.
#[derive(Clone, Default)]
struct Memo {
    /// The model of the Compact rows.
    model: Option<&'static str>,
    /// What the area and strip show while the host holds no mode.
    mode: AreaMode,
    /// The Full layout has taken the focus for the hex field.
    opened: bool,
    /// The radios' registrations.
    live: Arc<Mutex<GroupState<AreaMode>>>,
}

/// What the children reported in this pass, for `finish`.
#[derive(Default)]
struct Events {
    /// Colours the compare swatch's current half and the warning chips set.
    clicks: Vec<Srgba>,
    screen_pick: Option<ScreenPickGesture>,
    model_pick: Option<&'static str>,
    /// Only Web Colors turned on.
    web_on: bool,
}

/// A colour picker's body: a [`ColorArea`](super::ColorArea) and a [`ColorStrip`](super::ColorStrip), the channel
/// fields of its [`ColorModel`]s, the hex field and the host's hooks, in the Color panel and compact popup layout
/// ([`ColorPickerLayout::Compact`]) or the Color Picker dialog's ([`ColorPickerLayout::Full`]).
///
/// - Compact: the area over the channel rows of one model (a label that scrubs, a slider whose track shows the
///   channel's colours, a number field), a model switch, the hex row with the screen-pick button, and with
///   [`alpha`](ColorPicker::alpha) an "Opacity" row over the checkerboard.
/// - Full: the area square, the strip at its reading end, the opacity row and the screen-pick button under them.
///   Beside them the [`compare`](ColorPicker::compare) swatch (new over current) and the host's
///   [`warnings`](ColorPicker::warnings), over the channel grid: H S B, R G B, then "#" and the hex field; L a b and
///   the host's models; OKLCH in a column of its own when the host turns it on. A radio before each built-in
///   channel's label chooses the [`AreaMode`]; it edits no colour. The hex field takes the focus, its text selected,
///   when the layout first shows ([`focus_on_open`](ColorPicker::focus_on_open)).
/// - Edits: every child's gesture is the picker's, with the whole colour as its value: an area or strip drag, a slider
///   drag, a scrub or a typing run is one Begin to Commit (or Cancel); keys, the wheel, Enter, a sample, a compare or
///   warning click are `Set`s.
/// - Kept components: the picker keeps the values the user last edited, in that model, while they still give the
///   colour's 8-bit sRGB: greys keep their hue, black its hue and saturation, a typed Lab colour outside sRGB its
///   values. [`PickedColor::model`] carries them, and the host's binding holds them between passes.
/// - Only Web Colors ([`web_colors`](ColorPicker::web_colors)): every colour reported has each of red, green and blue
///   on a multiple of 0x33, whichever child made it. The Full layout shows the check box.
///
/// AccessKit: a `Group` named by the label; channel fields and sliders are named by the channel's name ("Hue"), or
/// `<model> <channel>` ("OKLCH hue") while another shown model has a channel of that name.
#[must_use = "show the picker with `show`"]
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
pub struct ColorPicker<'a> {
    #[common]
    common: Common,
    value: Bind<'a, PickedColor>,
    /// The layout; Compact by default.
    #[prop]
    layout: ColorPickerLayout,
    models: Option<Vec<&'a dyn ColorModel>>,
    /// What the area and the strip show, held by the host; kept per picker without it.
    #[prop]
    mode: Option<Bind<'a, AreaMode>>,
    /// Only Web Colors, held by the host (kept across opens). The Full layout shows its check box; the Compact layout
    /// has none, the host sets it.
    #[prop]
    web_colors: Option<Bind<'a, bool>>,
    /// Whether the picker edits the alpha: an "Opacity" row, and eight hex digits.
    #[prop]
    alpha: bool,
    /// Whether the channel sliders' tracks show the channel's colours with the others fixed (Dynamic Color Sliders).
    #[prop]
    dynamic_tracks: bool,
    /// Shows the screen-pick button.
    #[prop]
    eyedropper: bool,
    /// Disables the screen-pick button while the host samples.
    #[prop]
    eyedropper_busy: bool,
    /// The Full layout's compare swatch: the colour being edited over this one, the colour the host started from; a
    /// click on it sets it again.
    #[prop]
    compare: Option<Srgba>,
    warnings: Vec<ColorWarning>,
    /// Applied to every colour before it is reported, after Only Web Colors' snap.
    #[prop(func = "Fn(Srgba) -> Srgba")]
    constrain: Option<ColorConstraint<'a>>,
    /// The host's answer to a screen or canvas pick, for this pass; a `Set` that keeps the picker's alpha.
    #[prop]
    sampled: Option<Srgba>,
    /// Whether the Full layout focuses its hex field when it first shows; on by default.
    #[prop]
    focus_on_open: bool,
    rules: Rules<'a>,
    events: Events,
}

impl<'a> ColorPicker<'a> {
    /// A picker named `label` editing `value`.
    pub fn new(label: impl Into<craft_ui::runtime::env::Label>, value: Bind<'a, PickedColor>) -> Self {
        ColorPicker {
            common: Common::new(),
            value,
            layout: ColorPickerLayout::default(),
            models: None,
            mode: None,
            web_colors: None,
            alpha: false,
            dynamic_tracks: true,
            eyedropper: false,
            eyedropper_busy: false,
            compare: None,
            warnings: Vec::new(),
            constrain: None,
            sampled: None,
            focus_on_open: true,
            rules: Rules { web: false, constrain: None },
            events: Events::default(),
        }
        .label(label)
    }

    /// Every key a picker offering `models` may ask the host's catalogue for, each once: the area's names
    /// ([`AreaMode::name`]), and for the built-in models and `models` the model's name and each channel's name,
    /// qualified name ("OKLCH hue") and short label.
    pub fn keys<'m>(models: impl IntoIterator<Item = &'m dyn ColorModel>) -> Vec<&'static str> {
        let builtin: [&dyn ColorModel; 4] = builtin_models();
        let models: Vec<&dyn ColorModel> = builtin.into_iter().chain(models).collect();
        let words = models.iter().flat_map(|m| std::iter::once(m.name()).chain(m.channels().iter().flat_map(|ch| [ch.name, ch.qualified, ch.label])));
        let mut keys: Vec<&'static str> = Vec::new();
        for key in AreaMode::ALL.map(AreaMode::name).into_iter().chain(words) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    /// The models offered, in order, the host's own among them; the built-in models on by default in the layout
    /// without it.
    #[must_use]
    pub fn models(mut self, models: impl IntoIterator<Item = &'a dyn ColorModel>) -> Self {
        self.models = Some(models.into_iter().collect());
        self
    }

    /// The warnings beside the Full layout's compare swatch (out-of-gamut and web-colour warnings); a click on one
    /// sets its nearest passing colour. The host gives this pass's warnings for the colour it shows.
    #[must_use]
    pub fn warnings(mut self, warnings: impl IntoIterator<Item = ColorWarning>) -> Self {
        self.warnings = warnings.into_iter().collect();
        self
    }

    /// The models shown: the host's, or the built-in models on in the layout.
    fn models_list(&self) -> Vec<&'a dyn ColorModel> {
        let full = self.layout == ColorPickerLayout::Full;
        self.models
            .clone()
            .unwrap_or_else(|| builtin_models().into_iter().filter(|m| if full { m.default_on().full } else { m.default_on().compact }).collect())
    }

    /// The modes the Full layout's radios offer.
    fn offered(models: &[&dyn ColorModel]) -> Vec<AreaMode> {
        AreaMode::PHOTOSHOP.into_iter().chain(AreaMode::OKLCH).filter(|mode| models.iter().any(|m| m.id() == mode.model().id())).collect()
    }

    /// The levels the area and strip quantise to while Only Web Colors is on.
    fn quantize(&self) -> Option<usize> {
        self.rules.web.then_some(channels::WEB_LEVELS)
    }
}

impl Component for ColorPicker<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ColorPickerStyle;
    type Output = ColorPickerOutput;

    fn style(cx: &StyleCx<'_>, _: ColorPickerStyleKey) -> ColorPickerStyle {
        let control = cx.control();
        let tokens = cx.tokens;
        let sizes = Sizes {
            gap: tokens.space.gap.inline,
            stack: tokens.space.gap.stack,
            tight: tokens.space.gap.tight,
            section: tokens.space.gap.section,
            row: control.height,
            area: Vec2::new(tokens.size.plot.width, tokens.size.plot.height),
            strip: control.height,
            cell: tokens.size.length.short,
            short: tokens.size.length.short,
            hex: tokens.size.length.field,
            compare: Vec2::new(tokens.size.thumb.lg, tokens.size.thumb.md),
            caption: tokens.size.icon.md,
            chip: Vec2::new(tokens.size.icon.md, tokens.size.icon.md + tokens.space.gap.tight + control.height),
        };
        ColorPickerStyle { metrics: Metrics::new(), sizes }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sizes = &cx.style.sizes;
        let models = self.models_list();
        match self.layout {
            ColorPickerLayout::Compact => {
                let channels = models.iter().map(|m| m.channels().len()).max().unwrap_or(0);
                let input = layout::CompactInput { channels, switch: models.len() > 1, sample: self.eyedropper, alpha: self.alpha };
                layout::compact(sizes, 0.0, input).size
            }
            ColorPickerLayout::Full => {
                let input = layout::FullInput {
                    models: &models,
                    web: self.web_colors.is_some(),
                    alpha: self.alpha,
                    sample: self.eyedropper,
                    compare: self.compare.is_some(),
                    warnings: self.warnings.len(),
                };
                layout::full(sizes, input).size
            }
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, PickedColor>> {
        Some(self.value.reborrow())
    }

    fn content_env(&self, cx: &Cx<'_, Self>, values: &mut AppValues) {
        let handle = GroupHandle::<RadioKind<AreaMode>>::new(Arc::clone(&cx.recall::<Memo>().live));
        handle.begin_registration();
        values.set(handle);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let full = self.layout == ColorPickerLayout::Full;
        let models = self.models_list();
        let memo = cx.memory::<Memo>().clone();
        let held_mode = self.mode.as_mut().and_then(|mode| mode.reborrow().read(&mut *cx)).and_then(|mode| mode.same().copied());
        let mut mode = held_mode.unwrap_or(memo.mode);
        let web = self.web_colors.as_mut().and_then(|web| web.reborrow().read(&mut *cx)).and_then(|web| web.same().copied()).unwrap_or(false);
        self.rules = Rules { web, constrain: self.constrain.clone() };

        // The Full layout's radios pick like a radio group's: the member activated in this pass, found among the
        // registrations of the last.
        let handle = GroupHandle::<RadioKind<AreaMode>>::new(Arc::clone(&memo.live));
        if full {
            let offered = Self::offered(&models);
            let picked = cx.activated_member().and_then(|id| handle.member(id));
            let wanted = picked.or_else(|| offered.first().copied().filter(|first| !offered.contains(&mode) && *first != mode));
            if let Some(next) = wanted.filter(|next| *next != mode) {
                if let Some(held) = self.mode.as_mut() {
                    let event = EditEvent::set(GestureId::default(), Mixed::Same(mode), next, Arc::from("Field shows"), None);
                    let _ = held.reborrow().write(&mut *cx, &event);
                }
                mode = next;
            }
            handle.select(Some(mode));
        }

        let first_show = !memo.opened;
        let model = memo.model.filter(|id| models.iter().any(|m| m.id() == *id)).or_else(|| models.first().map(|m| m.id()));
        {
            let memo = cx.memory::<Memo>();
            memo.mode = mode;
            memo.model = model;
            memo.opened = true;
        }
        if full && self.focus_on_open && first_show && cx.resolved.enabled {
            focus::request(&cx.response.ctx, FocusTarget::Id(cx.id.with("hex")));
        }
        if let Some(color) = self.sampled {
            let now = cx.value().copied().unwrap_or(PickedColor::rgb(Srgba::BLACK));
            cx.set(now.with_color(color.with_alpha(now.color.a), &self.rules));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let words = cx.words::<ColorPickerWords>();
        let memo = cx.recall::<Memo>();
        let pass = compact::Pass {
            shown: cx.value().copied().unwrap_or(PickedColor::rgb(Srgba::BLACK)),
            rules: self.rules.clone(),
            models: self.models_list(),
            mode: memo.mode,
            model: memo.model,
            flow: Flow::new(cx.rect, cx.env.direction().is_rtl()),
            origin: cx.rect.min.to_vec2(),
            width: cx.rect.width(),
            words,
        };
        match self.layout {
            ColorPickerLayout::Compact => self.paint_compact(ui, cx, &pass),
            ColorPickerLayout::Full => self.paint_full(ui, cx, &pass),
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> ColorPickerOutput {
        if let Some(model) = self.events.model_pick {
            cx.memory::<Memo>().model = Some(model);
        }
        let mut now = cx.value().copied().unwrap_or(PickedColor::rgb(Srgba::BLACK));
        for color in &self.events.clicks {
            now = now.with_color(*color, &self.rules);
            cx.set(now);
        }
        if self.events.web_on {
            let snapped = Rules { web: true, constrain: self.rules.constrain.clone() }.apply(now.color);
            if !snapped.same_rgb8(now.color) {
                cx.set(now.with_color(snapped, &Rules { web: true, constrain: self.rules.constrain.clone() }));
            }
        }
        ColorPickerOutput { edit: cx.edit_response(), screen_pick: self.events.screen_pick }
    }
}
