//! [`ColorPicker`]: the colour picker's body, the area and strip with the channel rows, the hex
//! field and the hooks a host adds.

use std::cell::Cell;
use std::ops::RangeInclusive;

use craft_theme::{Channel, ColorModel, ControlSize, MAX_CHANNELS, ModelValues, Rgba, Srgba, TypeRole, builtin_models};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, Context, Id, IdSalt, Layout, Rect, Ui, UiBuilder, Vec2, pos2, vec2};

use super::area::{self, AreaOp};
use super::hex::{self, HexCodec};
use super::{AreaMode, ColorArea, ColorStrip, WEB_LEVELS, snapped, strip};
use crate::composites::PropertyRow;
use crate::controls::number_field::{self, FieldRule};
use crate::controls::slider::{self, Scale};
use crate::controls::{Button, IconButton, NumberField, Slider};
use crate::dialog::{Clicked, FooterButton};
use crate::direction;
use crate::edit::ops::{self, ScalarOp};
use crate::edit::{self, EditEvent, EditPolicy, EditResponse, Gesture, parts};
use crate::icons::Icon;
use crate::localized::{LocaleNumbers, Unit, Word, Words};
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::{Content, Gap, Row, Text};
use crate::theme::{Tokens, TokensOf};
pub use full::{ColorWarning, ColorWarningChip};

mod full;

/// What a [`ColorPicker`] reports: the colour, and the values of the model the user edited it
/// in while they still give it, so a host can keep a CMYK colour CMYK.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PickedColor {
    /// Encoded sRGB with straight alpha, after the host's [`constrain`](ColorPicker::constrain).
    pub color: Srgba,
    /// The values of the model edited, as the user set them: a typed Lab or CMYK colour outside
    /// sRGB stays as typed. `None` for a colour set as such (hex, a sample) and for one the
    /// constraint moved away from its values.
    pub model: Option<ModelValues>,
}

/// How a [`ColorPicker`] lays out its parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorPickerLayout {
    /// PhotoCraft's Color panel and compact popup: the area over the channel slider rows of one
    /// model, a model switch, the hex row and the optional alpha row.
    #[default]
    Compact,
    /// PhotoCraft's Color Picker dialog: the square area and the strip, the compare swatch and
    /// the host's warnings beside them over the channel grid, every model's channels at once with
    /// radios choosing what the area shows, and the hex field.
    Full,
}

/// What [`ColorPicker::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct ColorPickerResponse {
    /// The picker's edit; the response covers the picker, and its id is the one
    /// [`ColorPicker::sampled`] takes.
    pub edit: EditResponse<PickedColor>,
    /// The eyedropper button was pressed: the host starts sampling and gives the result with
    /// [`ColorPicker::sampled`].
    pub eyedropper_requested: bool,
    /// The [side action](ColorPicker::side_actions) clicked this pass, by click, Enter or Space
    /// on the focused button.
    pub side_clicked: Option<Clicked>,
}

/// A colour picker's body: a [`ColorArea`] and a [`ColorStrip`], the channel fields of its
/// [`ColorModel`]s, the hex field and the host's hooks, in PhotoCraft's Color panel and compact
/// popup layout ([`ColorPickerLayout::Compact`]) or its Color Picker dialog's
/// ([`ColorPickerLayout::Full`]):
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::ColorPicker;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.2, 0.5, 0.8);
/// let numbers = LocaleNumbers::from_tag("en");
/// let picked = ColorPicker::new("foreground", "Foreground colour", color, &numbers).alpha(true).show(ui);
/// if let Some(p) = picked.edit.event.as_ref().and_then(|e| e.value()) {
///     color = p.color;
/// }
/// # });
/// ```
///
/// - Compact layout: the area fills the width less the strip, `color_picker.area_height` high
///   ([`area_height`](ColorPicker::area_height)), the strip `color_picker.strip` wide at the
///   reading end. Under them each channel of the model is a [`PropertyRow`] whose word label
///   scrubs the [`NumberField`] at the row's end, then a [`Slider`] whose track shows the
///   channel's colours ([`dynamic_tracks`](ColorPicker::dynamic_tracks)). A model switch lists
///   the [`models`](ColorPicker::models) (HSB, RGB and OKLCH unless the host chooses). Then "#"
///   and the hex field, with the eyedropper button when the host gives the
///   [hook](ColorPicker::eyedropper); and with [`alpha`](ColorPicker::alpha) an "Opacity" row
///   over the checkerboard. The area shows the [`AreaMode`] the host holds
///   ([`mode`](ColorPicker::mode)), hue in the strip by default.
/// - Full layout: the area `color_picker.area` square, the strip `color_picker.strip_full` wide
///   at its reading end, the "Opacity" row with [`alpha`](ColorPicker::alpha) and the eyedropper
///   button under them. Beside them the [`compare`](ColorPicker::compare) swatch ("new" over
///   "current") and the host's [`warning`](ColorPicker::warning)s, over the channel grid: H, S, B,
///   R, G, B, then "#" and the hex field; L, a, b and the host's models; OKLCH in a column of its
///   own when the host turns it on ([`models`](ColorPicker::models); HSB, RGB and Lab
///   otherwise). A radio before each built-in channel's label chooses the [`AreaMode`]; it
///   edits no colour. The hex field takes the focus, its text selected, when the layout opens:
///   when it shows after a pass without it ([`focus_hex`](ColorPicker::focus_hex)), unless the
///   host turns that off ([`focus_on_open`](ColorPicker::focus_on_open)).
/// - Channel fields: one width in both layouts, the widest value of every built-in channel and
///   of the picker's models with its unit ([`NumberField::fit_width`]).
/// - Edits: every inner control's gesture is the picker's, with the whole colour as its value:
///   an area or strip drag, a slider drag, a scrub or a typing run is one Begin … Commit (or
///   Cancel); keys, the wheel, Enter and a sample are [`Set`](EditEvent::Set)s. A control's
///   step during another control's gesture waits for that gesture to end, then steps the colour
///   it left, as a Set of its own. Esc during a
///   gesture cancels it; Esc without one is the host's. Inside [`edit::scope`] only updates go
///   out. The host's [`EditPolicy`] decides how the picker's gesture reports.
/// - Kept components: the picker keeps the values the user last edited, in that model, while
///   they still give the colour's 8-bit sRGB (or are what gave the colour the picker reported):
///   greys keep their hue, black its hue and saturation, a typed Lab colour outside sRGB its
///   values. [`PickedColor::model`] carries them.
/// - Hex field: the colour in uppercase; valid input (six digits, eight with alpha, with or
///   without "#") applies as typed; partial or invalid input waits while the field has the
///   focus and goes when it leaves; signs and non-ASCII text are invalid. Another control's
///   change drops what was typed.
/// - Hooks: [`eyedropper`](ColorPicker::eyedropper) and [`sampled`](ColorPicker::sampled),
///   [`web_colors`](ColorPicker::web_colors), [`constrain`](ColorPicker::constrain),
///   [`text`](ColorPicker::text); in the Full layout
///   [`compare`](ColorPicker::compare) and [`warning`](ColorPicker::warning), whose clicks are
///   [`Set`](EditEvent::Set)s.
/// - Tab: in a dialog's [`FieldTab`](crate::behavior::FieldTab) scope Tab visits the number
///   fields and the hex field; the area, the strip and the sliders are reached with Ctrl+Tab.
/// - AccessKit: a `Group` named `label`; the channel fields and sliders are named by the
///   channel's full name ("Hue"), or "<model> <channel>" ("OKLCH hue") while another model on
///   show has a channel of that name (the rows' and the area's models in Compact, every model in
///   Full; the strip and the row label follow it); the hex field "Hex colour", the eyedropper by
///   the host's label.
///   Full: the radios are a `RadioGroup` "Field shows" with a `RadioButton` per channel, named
///   by the channel's full name; the compare swatch's halves are an `Image` "New colour" and a
///   `Button` "Restore current colour", each with its colour's hex as value; each warning a
///   `Button` named by its tip; with [`web_colors`](ColorPicker::web_colors) a `CheckBox`
///   "Only Web Colors" under the strip (Ctrl+Tab reaches it in a [`FieldTab`](crate::behavior::FieldTab)
///   scope, as it does every control but the fields).
#[must_use = "show the picker with `show`"]
pub struct ColorPicker<'a> {
    id_salt: IdSalt,
    label: String,
    color: Srgba,
    numbers: &'a LocaleNumbers,
    layout: ColorPickerLayout,
    models: Option<Vec<&'a dyn ColorModel>>,
    mode: Option<&'a mut AreaMode>,
    area_height: Option<f32>,
    dynamic_tracks: bool,
    alpha: bool,
    eyedropper: Option<String>,
    eyedropper_busy: bool,
    compare: Option<Srgba>,
    warnings: Vec<ColorWarning>,
    side_actions: Vec<FooterButton>,
    constrain: Option<Box<Constrain<'a>>>,
    web_colors: Option<&'a mut bool>,
    text: Option<Box<Translate<'a>>>,
    policy: EditPolicy,
    enabled: bool,
    focus_on_open: bool,
}

/// The host's constraint on every colour reported.
type Constrain<'a> = dyn Fn(Srgba) -> Srgba + 'a;
/// The host's translation of the picker's words.
type Translate<'a> = dyn Fn(&'static str) -> String + 'a;

/// The picker's own words and templates, English: the keys [`ColorPicker::text`] translates.
pub(super) mod texts {
    pub(crate) const OPACITY: &str = "Opacity";
    pub(crate) const HEX_COLOUR: &str = "Hex colour";
    /// The model switch's accessible name; `{model}` is the model's translated name.
    pub(crate) const COLOUR_MODEL: &str = "Colour model: {model}";
    pub(crate) const NOT_HEX: &str = super::hex::NOT_HEX;
    /// A percentage in a field; `{n}` is the number.
    pub(crate) const PERCENT: &str = "{n}%";
    pub(crate) const FIELD_SHOWS: &str = "Field shows";
    pub(crate) const NEW: &str = "new";
    pub(crate) const CURRENT: &str = "current";
    pub(crate) const NEW_COLOUR: &str = "New colour";
    pub(crate) const RESTORE: &str = "Restore current colour";
    /// A channel's label in the Full grid; `{label}` is the channel's translated short label.
    pub(crate) const GRID_LABEL: &str = "{label}:";
    pub(crate) const ONLY_WEB_COLORS: &str = "Only Web Colors";
    /// The Full layout's eyedropper button's tooltip.
    pub(crate) const SAMPLE_SCREEN: &str = "Sample color from the screen";
}

/// `texts` in the language installed on `ui`'s context, for the parts that are shown without a
/// host translation.
pub(super) fn installed<const N: usize>(ui: &Ui, texts: [&'static str; N]) -> [String; N] {
    let words = Words::of(ui);
    texts.map(|text| words.get(Word::plain(text)).into_owned())
}

/// The picker's plain words: [`ColorPicker::TEXTS`], the other parts' texts and the areas' names.
/// The words of the colour models and their channels come from the models.
pub(crate) const WORDS: &[Word] = &[
    Word::plain(texts::OPACITY),
    Word::plain(texts::HEX_COLOUR),
    Word::plain(texts::COLOUR_MODEL),
    Word::plain(texts::NOT_HEX),
    Word::plain(texts::PERCENT),
    Word::plain(texts::FIELD_SHOWS),
    Word::plain(texts::NEW),
    Word::plain(texts::CURRENT),
    Word::plain(texts::NEW_COLOUR),
    Word::plain(texts::RESTORE),
    Word::plain(texts::GRID_LABEL),
    Word::plain(texts::ONLY_WEB_COLORS),
    Word::plain(texts::SAMPLE_SCREEN),
    Word::plain("Saturation and brightness"),
    Word::plain("Hue and brightness"),
    Word::plain("Hue and saturation"),
    Word::plain("Blue and green"),
    Word::plain("Blue and red"),
    Word::plain("Red and green"),
    Word::plain("Green-red and blue-yellow"),
    Word::plain("Blue-yellow and lightness"),
    Word::plain("Green-red and lightness"),
    Word::plain("Hue and chroma"),
    Word::plain("Hue and lightness"),
    Word::plain("Chroma and lightness"),
    Word::plain("Hue"),
    Word::plain("Saturation"),
    Word::plain("Brightness"),
    Word::plain("Color"),
    Word::plain("degrees"),
    Word::plain("White"),
    Word::plain("Black"),
];

impl ColorPicker<'_> {
    /// The picker's own words and templates, English: keys [`text`](ColorPicker::text)
    /// translates. A template's `{placeholder}` stays in its translation, and the picker puts the
    /// part, translated on its own, in its place. [`keys`](ColorPicker::keys) lists these with
    /// the words of the models and the area.
    pub const TEXTS: [&'static str; 13] = [
        texts::OPACITY,
        texts::HEX_COLOUR,
        texts::COLOUR_MODEL,
        texts::NOT_HEX,
        texts::PERCENT,
        texts::FIELD_SHOWS,
        texts::NEW,
        texts::CURRENT,
        texts::NEW_COLOUR,
        texts::RESTORE,
        texts::GRID_LABEL,
        texts::ONLY_WEB_COLORS,
        texts::SAMPLE_SCREEN,
    ];

    /// Every key a picker offering `models` may ask [`text`](ColorPicker::text) for, each once:
    /// [`TEXTS`](ColorPicker::TEXTS), the area's names ([`AreaMode::name`]), and for the built-in
    /// models and `models` the model's name and each channel's name, qualified name
    /// ([`Channel::qualified`](craft_theme::Channel::qualified), "OKLCH hue") and short label.
    /// A host gives its translators this list.
    pub fn keys<'m>(models: impl IntoIterator<Item = &'m dyn ColorModel>) -> Vec<&'static str> {
        let builtin: [&dyn ColorModel; 4] = builtin_models();
        let models: Vec<&dyn ColorModel> = builtin.into_iter().chain(models).collect();
        let words = models.iter().flat_map(|m| std::iter::once(m.name()).chain(m.channels().iter().flat_map(|ch| [ch.name, ch.qualified, ch.label])));
        let mut keys: Vec<&'static str> = Vec::new();
        for key in Self::TEXTS.into_iter().chain(AreaMode::ALL.map(AreaMode::name)).chain(words) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }
}

impl<'a> ColorPicker<'a> {
    /// A picker named `label` (its group's accessible name, translated: "Foreground colour")
    /// editing the host's `color`; `numbers` writes and reads its fields. `id_salt` keeps its
    /// kept values, model and gestures when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, color: Srgba, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            color,
            numbers,
            layout: ColorPickerLayout::default(),
            models: None,
            mode: None,
            area_height: None,
            dynamic_tracks: true,
            alpha: false,
            eyedropper: None,
            eyedropper_busy: false,
            compare: None,
            warnings: Vec::new(),
            side_actions: Vec::new(),
            constrain: None,
            web_colors: None,
            text: None,
            policy: EditPolicy::default(),
            enabled: true,
            focus_on_open: true,
        }
    }

    /// The layout; [`Compact`](ColorPickerLayout::Compact) by default.
    pub fn layout(mut self, layout: ColorPickerLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The models offered, in order, the host's own among them; the built-in models on by
    /// default in the layout ([`ColorModel::default_on`]) without it.
    pub fn models(mut self, models: impl IntoIterator<Item = &'a dyn ColorModel>) -> Self {
        self.models = Some(models.into_iter().collect());
        self
    }

    /// What the area and the strip show, held by the host; kept per picker without it.
    pub fn mode(mut self, mode: &'a mut AreaMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// The Compact area's height; `color_picker.area_height` without it. A docked panel gives
    /// the room its group leaves.
    pub fn area_height(mut self, height: f32) -> Self {
        self.area_height = Some(height);
        self
    }

    /// Whether the channel sliders' tracks show the channel's colours with the others fixed
    /// (PhotoCraft's Dynamic Color Sliders); on by default.
    pub fn dynamic_tracks(mut self, dynamic: bool) -> Self {
        self.dynamic_tracks = dynamic;
        self
    }

    /// Whether the picker edits the alpha: an "Opacity" row, and eight hex digits; off by
    /// default.
    pub fn alpha(mut self, alpha: bool) -> Self {
        self.alpha = alpha;
        self
    }

    /// Shows the eyedropper button, named `label` (translated: "Pick screen color"); a press
    /// reports [`eyedropper_requested`](ColorPickerResponse::eyedropper_requested).
    pub fn eyedropper(mut self, label: impl Into<String>) -> Self {
        self.eyedropper = Some(label.into());
        self
    }

    /// Disables the eyedropper button while the host samples.
    pub fn eyedropper_busy(mut self, busy: bool) -> Self {
        self.eyedropper_busy = busy;
        self
    }

    /// The Full layout's compare swatch: the colour being edited over `current`, the colour the
    /// host started from (the dialog's original); a click on `current` sets it again.
    pub fn compare(mut self, current: Srgba) -> Self {
        self.compare = Some(current);
        self
    }

    /// A warning beside the Full layout's compare swatch (PhotoCraft's out-of-gamut and
    /// web-colour warnings); a click on it sets its nearest passing colour. The host gives this
    /// frame's warnings for the colour it shows.
    pub fn warning(mut self, warning: ColorWarning) -> Self {
        self.warnings.push(warning);
        self
    }

    /// PhotoCraft's Only Web Colors, held by the host (kept across opens): while `on` is true
    /// every colour the picker reports has each of red, green and blue on a multiple of 0x33,
    /// whichever control made it. The area and strip take the nearest such colour under the
    /// pointer, step to the next different one on a key, and draw flat cells of the snapped
    /// colours ([`ColorArea::quantize`]); the number fields, sliders, hex field, samples and the
    /// compare and warning clicks snap what they set. The Full layout shows the "Only Web Colors"
    /// check box under the strip, whose toggle writes `on`; switching it on snaps the colour at
    /// once (a [`Set`](EditEvent::Set)). The Compact layout has no box: the host sets `on`. The
    /// colour the host passes is shown as it is until an edit or the toggle snaps it. The
    /// [`constrain`](ColorPicker::constrain) hook runs after the snap.
    pub fn web_colors(mut self, on: &'a mut bool) -> Self {
        self.web_colors = Some(on);
        self
    }

    /// Applied to every colour before it is reported, after
    /// [`web_colors`](ColorPicker::web_colors)'s snap. Values it moves the colour away from are
    /// not reported as [`PickedColor::model`].
    pub fn constrain(mut self, constrain: impl Fn(Srgba) -> Srgba + 'a) -> Self {
        self.constrain = Some(Box::new(constrain));
        self
    }

    /// Translates the picker's words, given as English keys: those [`keys`](ColorPicker::keys)
    /// lists. Without it the words installed on the context ([`Words`]) translate them.
    pub fn text(mut self, translate: impl Fn(&str) -> String + 'a) -> Self {
        self.text = Some(Box::new(translate));
        self
    }

    /// How the picker's gesture reports ([`EditPolicy`]); Esc cancels a gesture when the policy
    /// is cancellable.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the Full layout focuses its hex field when it shows after a pass without it (a
    /// dialog opening, as PhotoCraft's); on by default. A host that shows the picker in a panel
    /// turns it off, so showing the panel again doesn't take the focus from the canvas.
    pub fn focus_on_open(mut self, focus: bool) -> Self {
        self.focus_on_open = focus;
        self
    }

    /// Buttons in a column at the trailing side of the Full layout, in the order given: a dialog's
    /// OK and Cancel, then actions such as Add to Swatches. They are the dialog footer's buttons
    /// ([`FooterButton`]; the [`Default`](crate::dialog::ButtonRole::Default) one is primary),
    /// as wide as the longest label. Enter presses the Default one and Esc the Cancel one, as a
    /// dialog footer does (not while a popup is open or the focus is outside the picker). The picker reports the click as
    /// [`side_clicked`](ColorPickerResponse::side_clicked) and the host acts on it, so a host
    /// that shows the picker in a [`Dialog`](crate::dialog::Dialog) with `Footer::none()` gives
    /// the dialog's answers itself. Full only.
    pub fn side_actions(mut self, actions: impl IntoIterator<Item = FooterButton>) -> Self {
        self.side_actions = actions.into_iter().collect();
        self
    }

    /// Gives the picker with id `id` (its [`response`](ColorPickerResponse::edit)'s) the
    /// eyedropper's sample; the picker reports it as a [`Set`](EditEvent::Set) in its next
    /// frame, with its own alpha. A cancelled sample gives nothing.
    pub fn sampled(ctx: &Context, id: Id, color: Srgba) {
        ctx.data_mut(|d| d.insert_temp(sample_key(id), color));
        ctx.request_repaint();
    }

    /// Focuses the hex field of the picker with id `id` (its [`response`](ColorPickerResponse::edit)'s)
    /// with its text selected, so typing replaces it. The Full layout does it when it opens
    /// ([`focus_on_open`](ColorPicker::focus_on_open)).
    pub fn focus_hex(ctx: &Context, id: Id) {
        hex::focus(ctx, id);
    }

    /// Shows the picker and returns this frame's edit.
    pub fn show(self, ui: &mut Ui) -> ColorPickerResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new()
            .scope_id(id)
            .layout(Layout::top_down_justified(Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(self.label.clone());
        let inner = ui.scope_builder(builder, |ui| self.show_inside(ui, id));
        let (event, eyedropper_requested, side_clicked) = inner.inner;
        ColorPickerResponse { edit: EditResponse::new(inner.response, event), eyedropper_requested, side_clicked }
    }

    fn show_inside(mut self, ui: &mut Ui, id: Id) -> (Option<EditEvent<PickedColor>>, bool, Option<Clicked>) {
        let tokens = Tokens::of(ui);
        ui.spacing_mut().item_spacing.y = Gap::Stack.points(&tokens);
        let enabled = self.enabled && ui.is_enabled();
        let full = self.layout == ColorPickerLayout::Full;
        let mut state = State::load(ui, id);
        let pass = ui.ctx().cumulative_pass_nr();
        if full && enabled && self.focus_on_open && state.shown_pass.is_none_or(|p| p + 1 < pass) {
            ColorPicker::focus_hex(ui.ctx(), id);
        }
        state.shown_pass = Some(pass);
        let models = self
            .models
            .clone()
            .unwrap_or_else(|| builtin_models().into_iter().filter(|m| if full { m.default_on().full } else { m.default_on().compact }).collect());
        let mut mode = self.mode.as_deref().copied().unwrap_or(state.mode);
        let (eyedropper_requested, side_clicked);
        let web = Cell::new(self.web_colors.as_deref().copied().unwrap_or(false));
        let event = {
            let words = Words::of(ui);
            let translate = |s: &'static str| self.text.as_ref().map_or_else(|| words.get(Word::plain(s)).into_owned(), |t| t(s));
            let constrain = |c: Srgba| {
                let c = if web.get() { snapped(c, WEB_LEVELS) } else { c };
                self.constrain.as_ref().map_or(c, |f| f(c))
            };
            let width = channel_width(ui, self.numbers, &models, &translate);

            let host = Picked::from_host(self.color, state.kept.as_ref());
            // Esc is the inner controls': each cancels its own gesture, which cancels this one.
            let outer = Gesture::load(ui, id, &host, self.policy.cancellable(false)).enabled(enabled);
            let start = outer.start().cloned();
            let inner = Inner { policy: EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost), enabled, width };
            let mut parts =
                Parts { edits: Edits::new(id, &state.parts, &outer, edit::in_scope(ui)), models: &models, constrain: &constrain, web: &web };
            parts.replay(ui.ctx());

            (eyedropper_requested, side_clicked) = match self.layout {
                ColorPickerLayout::Compact => (self.compact(ui, id, mode, &models, &mut state, inner, &translate, &mut parts), None),
                ColorPickerLayout::Full => self.full(ui, id, &mut mode, &models, inner, &translate, &mut parts),
            };
            let mut edits = parts.edits;
            if let Some(color) = ui.data_mut(|d| {
                let color = d.get_temp::<Srgba>(sample_key(id));
                d.remove::<Srgba>(sample_key(id));
                color
            }) {
                edits.input(Input::Sample(color));
            }
            // An input makes its value against the running value: the host's value after the typing
            // run it ended, and the part events of this pass that went out.
            for input in edits.inputs(ui.ctx(), &mut state.parts) {
                let now = edits.value().clone();
                edits.set(match input {
                    Input::Click(color) => now.with_color(color, &constrain),
                    Input::Sample(color) => now.with_color(color.with_alpha(now.color.a), &constrain),
                });
            }

            let outer = edits.feed(outer, &mut state.parts);
            let value = outer.shown().clone();
            let event = outer.finish_as(|p: Picked| p.report());
            match &event {
                Some(EditEvent::Cancel { .. }) => state.kept = start.or(state.kept),
                Some(e) if e.value().is_some() => state.kept = Some(value),
                _ => {}
            }
            event
        };
        if let Some(held) = self.web_colors.as_deref_mut() {
            *held = web.get();
        }
        match self.mode.as_deref_mut() {
            Some(held) => *held = mode,
            None => state.mode = mode,
        }
        state.store(ui, id);
        (event, eyedropper_requested, side_clicked)
    }

    /// The Compact layout: the area and strip, the slider rows of one model, the model switch,
    /// the hex row and the alpha row; whether the eyedropper button was pressed.
    #[allow(clippy::too_many_arguments)]
    fn compact(
        &self,
        ui: &mut Ui,
        id: Id,
        mode: AreaMode,
        models: &[&dyn ColorModel],
        state: &mut State,
        inner: Inner,
        translate: &dyn Fn(&'static str) -> String,
        parts: &mut Parts<'_>,
    ) -> bool {
        let style = ColorArea::style(&Tokens::of(ui));
        let height = self.area_height.unwrap_or(style.area_height);
        let gap = Gap::Inline.points(&Tokens::of(ui));
        let area_width = (ui.available_width() - gap - ColorStrip::outer_width(&style, style.strip)).max(0.0);
        let model = models.iter().copied().find(|m| Some(m.id()) == state.model).or_else(|| models.first().copied());
        let visible: Vec<&dyn ColorModel> = model.into_iter().chain([mode.model()]).collect();
        self.area_and_strip(ui, mode, &visible, inner, translate, parts, vec2(area_width, height), style.strip);
        if let Some(model) = model {
            let constrain = parts.constrain;
            for (i, ch) in model.channels().iter().enumerate() {
                let name = channel_name(model, i, &visible, translate);
                let row = RowRule::of(ch);
                let channel_op = |part| move |op| PickerOp::Channel { model: model.id(), i, part, op };
                let field =
                    row_field(ui, ("channel", i), &name, parts.value().values_in(model).1[i], &row, unit_of(ch.unit, translate), self.numbers, inner);
                let map = |x, now: &Picked| now.with_channel(model, i, x, constrain);
                parts.typing(&field, ops::take(ui.ctx(), field.response.id).into_iter().map(channel_op(RowPart::Field)), map);
                let (values, exact) = parts.value().values_in(model);
                let track = self.dynamic_tracks.then(|| model.track(i, &values));
                let slider = row_slider(ui, ("channel", i), &name, exact[i], &row, track, inner);
                parts.push(&slider, ops::take(ui.ctx(), slider.response.id).into_iter().map(channel_op(RowPart::Slider)), map);
            }
            if models.len() > 1 {
                state.model = Some(model_switch(ui, models, model.id(), translate, inner.enabled));
            }
        }
        let requested = self.hex_row(ui, id, translate, inner.enabled, parts);
        if self.alpha {
            self.alpha_row(ui, inner, translate, parts);
        }
        requested
    }

    /// The "Opacity" slider row, 0–100 % over the checkerboard.
    fn alpha_row(&self, ui: &mut Ui, inner: Inner, translate: &dyn Fn(&'static str) -> String, parts: &mut Parts<'_>) {
        let name = translate(texts::OPACITY);
        let constrain = parts.constrain;
        let map = |pct, now: &Picked| now.with_alpha(pct, constrain);
        let field = row_field(ui, "alpha", &name, parts.value().alpha, &ALPHA, unit_of("%", translate), self.numbers, inner);
        parts.typing(&field, ops::take(ui.ctx(), field.response.id).into_iter().map(|op| PickerOp::Alpha(RowPart::Field, op)), map);
        let now = parts.value();
        let opaque = now.color.with_alpha(1.0);
        let slider = row_slider(ui, "alpha", &name, now.alpha, &ALPHA, Some(vec![opaque.with_alpha(0.0), opaque]), inner);
        parts.push(&slider, ops::take(ui.ctx(), slider.response.id).into_iter().map(|op| PickerOp::Alpha(RowPart::Slider, op)), map);
    }

    /// The area `area` large and, at the reading end, the strip `strip` wide. The strip and the
    /// area's components are named as the `visible` models' channels are ([`channel_name`]).
    #[allow(clippy::too_many_arguments)]
    fn area_and_strip(
        &self,
        ui: &mut Ui,
        mode: AreaMode,
        visible: &[&dyn ColorModel],
        inner: Inner,
        translate: &dyn Fn(&'static str) -> String,
        parts: &mut Parts<'_>,
        area: Vec2,
        strip: f32,
    ) {
        let model = mode.model();
        let [across, down] = mode.area();
        let area_name = translate(mode.name());
        let constrain = parts.constrain;
        let map = |v: ModelValues, now: &Picked| now.with_values(&v, v.values.map(f64::from), constrain);
        Row::new(Gap::Inline).show(ui, |ui| {
            let area = ColorArea::new("area", area_name, mode, parts.value().values_in(model).0)
                .channel_names(channel_name(model, across, visible, translate), channel_name(model, down, visible, translate))
                .size(area)
                .policy(inner.policy)
                .enabled(inner.enabled)
                .part(true);
            let (area, ops) = match parts.quantize() {
                Some(levels) => area.quantize(levels),
                None => area,
            }
            .show_with_ops(ui);
            parts.push(&area, ops.into_iter().map(|op| PickerOp::Area(op, mode)), map);
            let strip = ColorStrip::new("strip", channel_name(model, mode.strip(), visible, translate), mode, parts.value().values_in(model).0)
                .size(vec2(strip, area.response.rect.height()))
                .policy(inner.policy)
                .enabled(inner.enabled)
                .part(true);
            let (strip, ops) = match parts.quantize() {
                Some(levels) => strip.quantize(levels),
                None => strip,
            }
            .show_with_ops(ui);
            parts.push(&strip, ops.into_iter().map(|op| PickerOp::Strip(op, mode)), map);
        });
    }

    /// "#", the hex field and the eyedropper button; whether the button was pressed.
    fn hex_row(&self, ui: &mut Ui, id: Id, translate: &dyn Fn(&'static str) -> String, enabled: bool, parts: &mut Parts<'_>) -> bool {
        let tokens = Tokens::of(ui);
        let gap = Gap::Inline.points(&tokens);
        let button = self.eyedropper.as_ref().map(|_| IconButton::style(&tokens, ControlSize::Md).side + gap);
        let mut requested = false;
        Row::new(Gap::Inline).show(ui, |ui| {
            ui.add(Text::new("#").role(TypeRole::Mono).content(Content::Muted));
            let width = (ui.available_width() - button.unwrap_or(0.0)).max(0.0);
            self.hex_field(ui, id, width, translate, enabled, parts);
            if let Some(label) = &self.eyedropper {
                let pressed = ui.add(IconButton::new(Icon::Eyedropper, label.as_str()).enabled(enabled && !self.eyedropper_busy)).clicked();
                requested = pressed;
            }
        });
        requested
    }

    /// The hex field, `width` wide.
    fn hex_field(&self, ui: &mut Ui, id: Id, width: f32, translate: &dyn Fn(&'static str) -> String, enabled: bool, parts: &mut Parts<'_>) {
        let codec = HexCodec { alpha: self.alpha, write_alpha: false, digits: 2, reason: translate(texts::NOT_HEX) };
        let label = translate(texts::HEX_COLOUR);
        // A constraint moves the typed colour, and the host value that differs from what the run
        // reported would cancel it: while the field has the focus it is given the colour it last
        // reported when the constraint maps that to the picker's colour.
        let reported = hex::hex_id(id).with("reported");
        let color = parts.value().color.to_rgba();
        let color = ui
            .data(|d| d.get_temp::<Rgba>(reported))
            .filter(|typed| ui.memory(|m| m.has_focus(hex::hex_id(id))) && (parts.constrain)((*typed).into()).to_rgba() == color)
            .unwrap_or(color);
        let field = ui
            .allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down_justified(Align::Min), |ui| {
                hex::show(
                    ui,
                    hex::hex_id(id),
                    &label,
                    color,
                    codec,
                    hex::Options::picker(EditPolicy::default().cancellable(self.policy.cancellable), enabled),
                )
            })
            .inner;
        let constrain = parts.constrain;
        if let Some(rgba) = field.event.as_ref().and_then(EditEvent::value) {
            ui.data_mut(|d| d.insert_temp(reported, *rgba));
        }
        let ops = field.event.as_ref().and_then(EditEvent::value).map(|rgba| PickerOp::Hex(*rgba));
        parts.typing(&field, ops, |rgba, now| now.with_hex(rgba, constrain));
    }
}

/// What the inner controls share: their policy, whether they take input, and the channel
/// fields' width.
#[derive(Clone, Copy, Debug)]
struct Inner {
    policy: EditPolicy,
    enabled: bool,
    /// The width of every channel field ([`channel_width`]).
    width: f32,
}

/// The width of every channel field, in either layout: the widest value of every built-in
/// channel and of the picker's models' channels, with its unit.
fn channel_width(ui: &Ui, numbers: &LocaleNumbers, models: &[&dyn ColorModel], translate: &dyn Fn(&'static str) -> String) -> f32 {
    let builtin: [&dyn ColorModel; 4] = builtin_models();
    builtin
        .into_iter()
        .chain(models.iter().copied())
        .flat_map(|m| m.channels().iter())
        .map(|ch| {
            let range = f64::from(ch.min)..=f64::from(ch.max);
            NumberField::fit_width(ui, ControlSize::Md, numbers, range, usize::from(ch.decimals), &unit_of(ch.unit, translate))
        })
        .fold(0.0, f32::max)
}

/// Channel `channel` of `model`'s accessible name, translated: the channel's name, or its
/// qualified name ("OKLCH hue") when another of the `visible` models has a channel of that name.
fn channel_name(model: &dyn ColorModel, channel: usize, visible: &[&dyn ColorModel], translate: &dyn Fn(&'static str) -> String) -> String {
    let ch = model.channels()[channel];
    let clash = visible.iter().any(|m| m.id() != model.id() && m.channels().iter().any(|other| other.name == ch.name));
    translate(if clash { ch.qualified } else { ch.name })
}

/// How a slider row's or a channel field's value steps and settles: a channel's or the alpha's.
struct RowRule {
    range: RangeInclusive<f64>,
    step: f64,
    decimals: u8,
}

/// The alpha row's rule: 0–100 %, whole percents.
const ALPHA: RowRule = RowRule { range: 0.0..=100.0, step: 1.0, decimals: 0 };

impl RowRule {
    fn of(channel: &Channel) -> RowRule {
        RowRule { range: f64::from(channel.min)..=f64::from(channel.max), step: f64::from(channel.step), decimals: channel.decimals }
    }

    /// The value `op` of `part` gives the row at `from`, as that part applies it.
    fn apply(&self, part: RowPart, op: &ScalarOp, from: f64) -> f64 {
        match part {
            RowPart::Slider => {
                let scale = Scale::new(&self.range, Some(self.step));
                slider::apply(op, scale.place(from), (scale.min, scale.max), &scale)
            }
            RowPart::Field => number_field::apply(op, from, &FieldRule::new(&self.range, self.step, usize::from(self.decimals))),
        }
    }
}

/// The word label `name` of a Compact slider row, which scrubs and names the number field at the
/// row's end; the field's edit.
#[allow(clippy::too_many_arguments)]
fn row_field(
    ui: &mut Ui,
    salt: impl AsIdSalt + Copy,
    name: &str,
    value: f64,
    rule: &RowRule,
    unit: Unit,
    numbers: &LocaleNumbers,
    inner: Inner,
) -> EditResponse<f64> {
    let Inner { policy, enabled, width } = inner;
    PropertyRow::word(name)
        .scrubs()
        .show(ui, |ui, row| {
            at_end(ui, width, |ui| {
                let field = NumberField::new(("field", salt), name, value, numbers)
                    .range(rule.range.clone())
                    .step(rule.step)
                    .decimals(0..=usize::from(rule.decimals))
                    .unit(unit)
                    .policy(policy)
                    .part(true)
                    .enabled(enabled);
                match row.scrub() {
                    Some(label) => field.scrub_by(label),
                    None => field,
                }
                .show(ui)
            })
        })
        .inner
}

/// The slider under a Compact row's label, over `track`'s colours; its edit.
fn row_slider(
    ui: &mut Ui,
    salt: impl AsIdSalt + Copy,
    name: &str,
    value: f64,
    rule: &RowRule,
    track: Option<Vec<Srgba>>,
    inner: Inner,
) -> EditResponse<f64> {
    let mut slider =
        Slider::new(("slider", salt), name, value, rule.range.clone()).step(rule.step).policy(inner.policy).enabled(inner.enabled).part(true);
    if let Some(stops) = track {
        slider = slider.track_colors(stops);
    }
    slider.show(ui)
}

/// `add` in a column `width` wide at the reading end of the `Ui`.
fn at_end<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let full = ui.available_rect_before_wrap();
    let width = width.min(full.width());
    let x = if direction::is_rtl(ui) { full.left() } else { full.right() - width };
    let rect = Rect::from_min_size(pos2(x, full.top()), vec2(width, full.height()));
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::top_down_justified(Align::Min)), add).inner
}

/// The model switch under the rows (PhotoCraft's "HSB/HSL" / "RGB Color" press-menu button): a
/// button with the current model's name opens a menu of the models, one radio item each. The
/// button's accessible name is "Colour model: <model>", which holds its visible label. Returns
/// the model chosen.
fn model_switch(
    ui: &mut Ui,
    models: &[&dyn ColorModel],
    current: &'static str,
    translate: &dyn Fn(&'static str) -> String,
    enabled: bool,
) -> &'static str {
    let name = models.iter().find(|m| m.id() == current).map_or_else(String::new, |m| translate(m.name()));
    let entries: Vec<MenuEntry<&'static str>> =
        models.iter().map(|m| MenuItem::new(translate(m.name()), m.id()).radio(m.id() == current).into()).collect();
    ui.with_layout(direction::column(ui), |ui| {
        let label = translate(texts::COLOUR_MODEL).replace("{model}", &name);
        let button = ui.add(Button::new(name).id_salt("model").enabled(enabled));
        ui.ctx().accesskit_node_builder(button.id, |node| node.set_label(label));
        PressMenu::new(&entries).show(ui, &button).chosen
    })
    .inner
    .unwrap_or(current)
}

/// A channel's unit as a number field shows it: the template for "°" or "%", none otherwise.
fn unit_of(unit: &'static str, translate: &dyn Fn(&'static str) -> String) -> Unit {
    match unit {
        "" => Unit::default(),
        "%" => Unit::new(&translate(texts::PERCENT)),
        other => Unit::new(&format!("{{n}}{other}")),
    }
}

fn sample_key(id: Id) -> Id {
    id.with("craft_ui::color_picker::sampled")
}

/// What the picker keeps between frames, per picker id.
#[derive(Clone, Debug, Default)]
struct State {
    /// The value the picker last reported: the colour with the values that made it.
    kept: Option<Picked>,
    /// The inner controls' edits: the owner of the picker's gesture, and the inputs (clicks and
    /// samples) that wait for their turn ([`Edits::inputs`]).
    parts: parts::State<PickerOp, Input>,
    /// The model of the Compact rows.
    model: Option<&'static str>,
    mode: AreaMode,
    /// The pass the picker was last shown in: the Full layout opens when it shows again after a
    /// pass without it.
    shown_pass: Option<u64>,
}

impl State {
    fn key(id: Id) -> Id {
        id.with("craft_ui::color_picker")
    }

    fn load(ui: &Ui, id: Id) -> Self {
        ui.data(|d| d.get_temp::<Self>(Self::key(id))).unwrap_or_default()
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(Self::key(id), self));
    }
}

/// The picker's value: the colour and the model values the user made it with. Two values are
/// equal when their 8-bit colours and their values are, so a host that keeps 8-bit colour still
/// echoes what the picker reported.
#[derive(Clone, Debug)]
struct Picked {
    color: Srgba,
    /// The values last edited, in their model.
    values: Option<ModelValues>,
    /// The same values as the number fields and sliders reported them, which `f32` would round.
    exact: [f64; MAX_CHANNELS],
    /// The alpha in percent, as the alpha row reported it.
    alpha: f64,
    /// `color` was made from `values` (perhaps moved by the host's constraint): they show
    /// whatever the colour.
    made: bool,
    /// `values` give `color`'s 8-bit sRGB: they go out with it.
    faithful: bool,
}

impl PartialEq for Picked {
    fn eq(&self, other: &Self) -> bool {
        self.color.to_rgba() == other.color.to_rgba()
            && self.values == other.values
            && self.exact == other.exact
            && self.alpha == other.alpha
            && self.made == other.made
            && self.faithful == other.faithful
    }
}

impl Picked {
    /// The host's `color` with the values `kept` from the picker's last report: as reported
    /// while the host shows that colour; otherwise the values stay for the kept-components rule.
    fn from_host(color: Srgba, kept: Option<&Picked>) -> Picked {
        match kept {
            Some(k) if k.color.to_rgba() == color.to_rgba() => Picked { color, ..k.clone() },
            Some(k) => Picked { color, alpha: percent(color.a), made: false, faithful: false, ..k.clone() },
            None => Picked { color, values: None, exact: [0.0; MAX_CHANNELS], alpha: percent(color.a), made: false, faithful: false },
        }
    }

    /// The colour in `model`: the kept values while they are of `model` and made the colour or
    /// still give its 8-bit sRGB, otherwise converted; with the fields' exact values.
    fn values_in(&self, model: &dyn ColorModel) -> (ModelValues, [f64; MAX_CHANNELS]) {
        match self.values {
            Some(v) if v.model == model.id() && (self.made || model.to_srgb(&v.values).same_rgb8(self.color)) => {
                let exact = std::array::from_fn(|i| if self.exact[i] as f32 == v.values[i] { self.exact[i] } else { f64::from(v.values[i]) });
                (v, exact)
            }
            _ => {
                let v = model.from_srgb(self.color);
                (v, v.values.map(f64::from))
            }
        }
    }

    /// The colour `values` give, with this alpha.
    fn with_values(&self, values: &ModelValues, exact: [f64; MAX_CHANNELS], constrain: &dyn Fn(Srgba) -> Srgba) -> Picked {
        let made = model_of(values).map_or(self.color, |m| m.to_srgb(&values.values)).with_alpha(self.color.a);
        let color = constrain(made);
        Picked { color, values: Some(*values), exact, alpha: self.alpha, made: true, faithful: color.same_rgb8(made) }
    }

    /// Channel `channel` of `model` set to `value`.
    fn with_channel(&self, model: &dyn ColorModel, channel: usize, value: f64, constrain: &dyn Fn(Srgba) -> Srgba) -> Picked {
        let (mut values, mut exact) = self.values_in(model);
        values.values[channel] = value as f32;
        exact[channel] = value;
        let made = model.to_srgb(&values.values).with_alpha(self.color.a);
        let color = constrain(made);
        Picked { color, values: Some(values), exact, alpha: self.alpha, made: true, faithful: color.same_rgb8(made) }
    }

    /// The alpha set to `percent`.
    fn with_alpha(&self, percent: f64, constrain: &dyn Fn(Srgba) -> Srgba) -> Picked {
        let color = constrain(self.color.with_alpha((percent / 100.0) as f32));
        Picked { color, alpha: percent, ..self.clone() }
    }

    /// A colour set as such (a sample); no model values.
    fn with_color(&self, color: Srgba, constrain: &dyn Fn(Srgba) -> Srgba) -> Picked {
        let color = constrain(color);
        let alpha = if color.to_rgba().0[3] == self.color.to_rgba().0[3] { self.alpha } else { percent(color.a) };
        Picked { color, values: None, exact: [0.0; MAX_CHANNELS], alpha, made: false, faithful: false }
    }

    /// The hex field's colour; this value itself while the field shows it.
    fn with_hex(&self, rgba: Rgba, constrain: &dyn Fn(Srgba) -> Srgba) -> Picked {
        if rgba == self.color.to_rgba() { self.clone() } else { self.with_color(rgba.into(), constrain) }
    }

    /// This value with `op` applied, a channel's in its model among `models`; `None` when that
    /// model isn't one of them.
    fn apply(&self, op: &PickerOp, models: &[&dyn ColorModel], constrain: &dyn Fn(Srgba) -> Srgba, quantize: Option<usize>) -> Option<Picked> {
        let with_values = |values: ModelValues| self.with_values(&values, values.values.map(f64::from), constrain);
        Some(match op {
            PickerOp::Area(op, mode) => with_values(area::apply(op, &self.values_in(mode.model()).0, *mode, quantize)),
            PickerOp::Strip(op, mode) => with_values(strip::apply(op, &self.values_in(mode.model()).0, *mode, quantize)),
            PickerOp::Channel { model, i, part, op } => {
                let model = models.iter().copied().find(|m| m.id() == *model)?;
                let rule = RowRule::of(model.channels().get(*i)?);
                self.with_channel(model, *i, rule.apply(*part, op, self.values_in(model).1[*i]), constrain)
            }
            PickerOp::Alpha(part, op) => self.with_alpha(ALPHA.apply(*part, op, self.alpha), constrain),
            PickerOp::Hex(rgba) => self.with_hex(*rgba, constrain),
        })
    }

    fn report(self) -> PickedColor {
        PickedColor { color: self.color, model: self.values.filter(|_| self.made && self.faithful) }
    }
}

fn percent(alpha: f32) -> f64 {
    f64::from(alpha) * 100.0
}

/// The built-in model of `values`, for the area and strip's values; a host model's values come
/// from its rows, which convert with the model itself.
fn model_of(values: &ModelValues) -> Option<&'static dyn ColorModel> {
    builtin_models().into_iter().find(|m| m.id() == values.model)
}

/// The inner controls' edits of a pass, as the picker's values; the clicks are the colours the
/// compare swatch's current half and the warning chips set.
type Edits = parts::Edits<Picked, PickerOp, Input>;

/// The picker's parts in a pass: their edits, and what their operations apply with.
struct Parts<'a> {
    edits: Edits,
    models: &'a [&'a dyn ColorModel],
    constrain: &'a dyn Fn(Srgba) -> Srgba,
    /// Whether Only Web Colors is on; the check box can switch it during the pass.
    web: &'a Cell<bool>,
}

impl Parts<'_> {
    /// The levels the area and strip quantise to, when Only Web Colors is on.
    fn quantize(&self) -> Option<usize> {
        self.web.get().then_some(WEB_LEVELS)
    }

    /// Applies the queued operations whose turn has come ([`Edits::replay`]), before any part is
    /// drawn.
    fn replay(&mut self, ctx: &Context) {
        let (models, constrain, quantize) = (self.models, self.constrain, self.quantize());
        self.edits.replay(ctx, |op, now| now.apply(op, models, constrain, quantize));
    }

    /// The picker's value with the replay and the edits pushed so far: each part draws from it.
    fn value(&self) -> Picked {
        self.edits.value().clone()
    }

    /// A part's edit and its operations ([`Edits::push`]); `map` puts a gesture's value on the
    /// picker's.
    fn push<T: Clone + Send + Sync + 'static>(
        &mut self,
        edit: &EditResponse<T>,
        ops: impl IntoIterator<Item = PickerOp>,
        map: impl Fn(T, &Picked) -> Picked,
    ) {
        let (models, constrain, quantize) = (self.models, self.constrain, self.quantize());
        self.edits.push(edit, ops, |op, now| now.apply(op, models, constrain, quantize), map);
    }

    /// [`push`](Parts::push) for a field whose focus is a typing run's.
    fn typing<T: Clone + Send + Sync + 'static>(
        &mut self,
        field: &EditResponse<T>,
        ops: impl IntoIterator<Item = PickerOp>,
        map: impl Fn(T, &Picked) -> Picked,
    ) {
        let (models, constrain, quantize) = (self.models, self.constrain, self.quantize());
        self.edits.typing(field, ops, |op, now| now.apply(op, models, constrain, quantize), map);
    }
}

/// What the user did to one of the picker's parts, which applies to the picker's value of its
/// turn ([`Picked::apply`]).
#[derive(Clone, Debug, PartialEq)]
enum PickerOp {
    /// The area's, in its mode.
    Area(AreaOp, AreaMode),
    /// The strip's, in its mode.
    Strip(ScalarOp, AreaMode),
    /// Channel `i` of the model with id `model`: its slider's or its number field's.
    Channel { model: &'static str, i: usize, part: RowPart, op: ScalarOp },
    /// The alpha row's slider's or field's.
    Alpha(RowPart, ScalarOp),
    /// The hex field's colour.
    Hex(Rgba),
}

/// Which control of a row made an operation: each applies its own way ([`RowRule::apply`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowPart {
    Slider,
    Field,
}

/// One of the picker's own inputs, which wait for their turn.
#[derive(Clone, Copy, Debug)]
enum Input {
    /// A click on a swatch that sets the colour (alpha as the swatch has it).
    Click(Srgba),
    /// The eyedropper's sample; the host's alpha stays.
    Sample(Srgba),
}
