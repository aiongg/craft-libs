//! The channel-mixing adjustments: Black & White, Channel Mixer and Selective Color.

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::Value;
use craft_theme::Srgba;
use craft_theme::variant::Tone;
use craft_ui::buttons::Button;
use craft_ui::choice::{Checkbox, Radio, RadioGroup, Swatch};
use craft_ui::contract::{Bind, EditResponse, Entry, Outcome, Pattern, Props as _, Remember as _, Show as _, ShowContainer as _, clicked};
use craft_ui::display::Text;
use craft_ui::form::{Field, FieldGroup, Help};
use craft_ui::layout_components::Row;
use craft_ui::pickers::Select;
use craft_ui::runtime::env::{Env, Label, UnitKind, Units};
use craft_ui::runtime::requests::ColorRequest;
use craft_ui::style::TypeRole;
use egui::Ui;

use super::sliders::{Run, lifted, read, words};
use super::style::{mix, tones};
use super::{AdjustContext, AdjustField, AdjustTrack, AdjustWords};

/// The key of the tint colour request.
const TINT_KEY: &str = "blackWhite.tint";

// ---------------------------------------------------------------------------------------------
// Black & White

/// A Black & White value.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlackWhite {
    /// Reds, Yellows, Greens, Cyans, Blues, Magentas, −200..=300 %.
    pub weights: [f64; 6],
    /// Tint is on.
    pub tint: bool,
    /// The tint colour.
    pub tint_color: Srgba,
}

impl BlackWhite {
    /// A value of the six `weights`, Tint and its colour.
    pub fn new(weights: [f64; 6], tint: bool, tint_color: Srgba) -> Self {
        Self { weights, tint, tint_color }
    }

    /// Photoshop's default weights: Reds 40, Yellows 60, Greens 40, Cyans 60, Blues 20, Magentas 80.
    pub const DEFAULT_WEIGHTS: [f64; 6] = [40.0, 60.0, 40.0, 60.0, 20.0, 80.0];
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self { weights: Self::DEFAULT_WEIGHTS, tint: false, tint_color: Srgba::rgb(0.882, 0.827, 0.702) }
    }
}

/// What a Black & White editor keeps between passes: the value its running gesture began with.
#[derive(Clone, Default)]
struct BlackWhiteMemory {
    began: Option<BlackWhite>,
}

/// What [`BlackWhiteEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BlackWhiteOutput {
    /// The edit of the value.
    pub edit: EditResponse<BlackWhite>,
    /// The user clicked the Tint well: open the colour picker with this request (key `"blackWhite.tint"`) and write
    /// the answer into the value's `tint_color`.
    pub pick_color: Option<ColorRequest>,
    /// The user pressed Auto: the host computes the weights from the image and sets them.
    pub auto: bool,
}

impl Outcome for BlackWhiteOutput {
    type Value = BlackWhite;

    fn response(&self) -> &egui::Response {
        &self.edit.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.edit.story_facts()
    }
}

impl std::ops::Deref for BlackWhiteOutput {
    type Target = EditResponse<BlackWhite>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

/// Black & White: six colour rows, Tint with a colour well, and the Auto and Default buttons.
///
/// - Rows Reds … Magentas, −200..300 %, each over a track from black to its colour while dynamic tracks are on; a
///   reset restores that row's default.
/// - Tint is a check box with a well that asks for a colour through [`BlackWhiteOutput::pick_color`]; the well is
///   off while Tint is.
/// - Auto reports `auto`; Default sets the six default weights as one `Set`.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct BlackWhiteEditor<'a> {
    label: Label,
    value: Bind<'a, BlackWhite>,
    context: AdjustContext,
}

impl<'a> BlackWhiteEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, BlackWhite>) -> Self {
        Self { label: label.into(), value, context: AdjustContext::dialog(true) }
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit and requests.
    pub fn show(self, ui: &mut Ui) -> BlackWhiteOutput {
        let BlackWhiteEditor { label, mut value, context } = self;
        let words = words(ui);
        let mut out = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let names = [&words.reds, &words.yellows, &words.greens, &words.cyans, &words.blues, &words.magentas];
            let keys = ["reds", "yellows", "greens", "cyans", "blues", "magentas"];
            let mut run = Run::start(&mut value, context, now, cx.memory::<BlackWhiteMemory>().began);
            for (i, (name, key)) in names.into_iter().zip(keys).enumerate() {
                let field = AdjustField::new(key, name.clone(), -200.0..=300.0, BlackWhite::DEFAULT_WEIGHTS[i])
                    .unit_kind(UnitKind::Percent)
                    .track(AdjustTrack::Colors(vec![tones::BLACK, mix::RANGES[i]]));
                run.slider_row(ui, cx, &field, now.weights[i], move |now: &BlackWhite, x| {
                    let mut next = *now;
                    next.weights[i] = x;
                    next
                });
            }
            let (mut tint, mut pick_color, mut auto, mut reset) = (None, None, false, false);
            Row::new().show(ui, |ui, _| {
                tint = Some(Checkbox::new(Bind::given(now.tint)).label(words.tint.clone()).show(ui));
                pick_color = Swatch::new(words.tint_color.clone(), now.tint_color).picker(TINT_KEY).enabled(now.tint).show(ui).pick_color;
                auto = clicked(&Button::new(words.auto.clone()).id_salt("auto").show(ui));
                reset = clicked(&Button::new(words.default_button.clone()).id_salt("default").show(ui));
            });
            if let Some(tint) = tint {
                run.push(cx, tint, |now, on| BlackWhite { tint: *on, ..*now });
            }
            if reset {
                run.set(cx, &words.default_button, BlackWhite { weights: BlackWhite::DEFAULT_WEIGHTS, ..now });
            }
            cx.memory::<BlackWhiteMemory>().began = run.began();
            out = Some((run.finish(), pick_color, auto));
        });
        let (edit, pick_color, auto) = out.expect("an editor shows its body");
        BlackWhiteOutput { edit: lifted(Some(edit), shown.output), pick_color, auto }
    }
}

// ---------------------------------------------------------------------------------------------
// The select over a table of slider rows (Channel Mixer, Selective Color)

/// A select that picks one of several tables of slider rows, and the rows of the picked table.
struct Tables<'t> {
    name: Label,
    entries: &'t [Entry<usize>],
    picked: usize,
    fields: &'t [AdjustField],
    values: &'t [f64],
}

impl Tables<'_> {
    /// Shows the select and the rows. A row `i` edits through `set(value, i, x)`. Returns the table the user picked.
    fn show<V: Value>(
        &self,
        ui: &mut Ui,
        cx: &mut craft_ui::contract::BodyCx<'_>,
        run: &mut Run<'_, '_, V>,
        set: impl Fn(&V, usize, f64) -> V + Clone,
    ) -> Option<usize> {
        let mut chosen = None;
        Field::new(self.name.clone()).id_salt("select").show(ui, |ui, _| {
            chosen = Select::new(self.name.clone(), Bind::given(self.picked), self.entries).show(ui).value().copied();
        });
        for (i, field) in self.fields.iter().enumerate() {
            let set = set.clone();
            run.slider_row(ui, cx, field, self.values[i], move |now, x| set(now, i, x));
        }
        chosen
    }
}

// ---------------------------------------------------------------------------------------------
// Channel Mixer

/// A Channel Mixer value: per output channel the Red, Green, Blue and Constant percentages.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChannelMixer {
    /// The Red output: Red, Green, Blue, Constant.
    pub red: [f64; 4],
    /// The Green output.
    pub green: [f64; 4],
    /// The Blue output.
    pub blue: [f64; 4],
    /// The Gray output while Monochrome is on.
    pub gray: Option<[f64; 4]>,
}

impl ChannelMixer {
    /// A value of the three output rows and the Gray row of Monochrome.
    pub fn new(red: [f64; 4], green: [f64; 4], blue: [f64; 4], gray: Option<[f64; 4]>) -> Self {
        Self { red, green, blue, gray }
    }

    /// The Gray row Photoshop starts Monochrome with.
    pub const MONOCHROME_START: [f64; 4] = [40.0, 40.0, 20.0, 0.0];
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self { red: [100.0, 0.0, 0.0, 0.0], green: [0.0, 100.0, 0.0, 0.0], blue: [0.0, 0.0, 100.0, 0.0], gray: None }
    }
}

impl ChannelMixer {
    /// The row of output `out`; the one Gray row in Monochrome.
    fn shown_row(&self, out: usize) -> [f64; 4] {
        match (self.gray, out) {
            (Some(gray), _) => gray,
            (None, 0) => self.red,
            (None, 1) => self.green,
            (None, _) => self.blue,
        }
    }

    /// The value with the row of output `out` replaced, in whole percent.
    fn with_row(mut self, out: usize, row: [f64; 4]) -> Self {
        let row = row.map(f64::round);
        match (self.gray, out) {
            (Some(_), _) => self.gray = Some(row),
            (None, 0) => self.red = row,
            (None, 1) => self.green = row,
            (None, _) => self.blue = row,
        }
        self
    }

    /// The sum of the three sources.
    fn total(row: [f64; 4]) -> f64 {
        row[0] + row[1] + row[2]
    }
}

/// The Total readout as text: whole percent, with a sign for zero and up.
fn total_text(units: &Units, words: &AdjustWords, total: f64) -> String {
    let whole = total.round();
    let text = units.text(UnitKind::Percent, whole, 0..=0);
    if whole < 0.0 { text } else { words.plus.fill(&[("n", &text)]) }
}

/// Whether the Total is over what the channels can show.
fn total_warns(total: f64) -> bool {
    total > 100.0
}

/// What a Channel Mixer editor keeps between passes: the output channel shown and the value its running gesture began
/// with.
#[derive(Clone, Default)]
struct MixerMemory {
    output: usize,
    began: Option<ChannelMixer>,
}

/// Channel Mixer: the Output Channel select, the three source rows of that output, the Total readout, Constant and
/// Monochrome.
///
/// - The output channel is view state of the editor. With Monochrome on the one output is Gray, and turning it off
///   removes the Gray row.
/// - Rows Red, Green, Blue (−200..200 %, tracks from black to the channel while dynamic tracks are on) start from the
///   identity of their output on a reset; Constant resets to 0.
/// - The Total readout is the sum of the three sources and warns above 100 %.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct ChannelMixerEditor<'a> {
    label: Label,
    value: Bind<'a, ChannelMixer>,
    context: AdjustContext,
}

impl<'a> ChannelMixerEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, ChannelMixer>) -> Self {
        Self { label: label.into(), value, context: AdjustContext::dialog(true) }
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit of the `ChannelMixer`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<ChannelMixer> {
        let ChannelMixerEditor { label, mut value, context } = self;
        let words = words(ui);
        let units = Env::of(ui).units();
        let mut edit = None;
        let shown = FieldGroup::new(label.clone()).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let mono = now.gray.is_some();
            // Monochrome has the one output, Gray.
            let outputs = if mono { 1 } else { 3 };
            let memory = cx.memory::<MixerMemory>().clone();
            let out = if memory.output < outputs { memory.output } else { 0 };
            let row = now.shown_row(out);
            // A row starts from the identity of its output (Gray from Photoshop's mix).
            let identity: [f64; 4] = if mono { ChannelMixer::MONOCHROME_START } else { std::array::from_fn(|i| if i == out { 100.0 } else { 0.0 }) };
            let sources = [("red", &words.red), ("green", &words.green), ("blue", &words.blue)];
            let fields: Vec<AdjustField> = sources
                .iter()
                .enumerate()
                .map(|(i, (key, name))| {
                    AdjustField::new(key, (*name).clone(), -200.0..=200.0, identity[i])
                        .unit_kind(UnitKind::Percent)
                        .track(AdjustTrack::Colors(vec![tones::BLACK, mix::SOURCES[i]]))
                })
                .collect();
            let entries: Vec<Entry<usize>> = if mono {
                vec![Entry::new(0, words.gray.clone())]
            } else {
                sources.iter().enumerate().map(|(i, (_, name))| Entry::new(i, (*name).clone())).collect()
            };
            let mut run = Run::start(&mut value, context, now, memory.began);
            let set = move |now: &ChannelMixer, i: usize, x: f64| {
                let mut row = now.shown_row(out);
                row[i] = x;
                now.with_row(out, row)
            };
            let tables = Tables { name: words.output_channel.clone(), entries: &entries, picked: out, fields: &fields, values: &row[..3] };
            let picked = tables.show(ui, cx, &mut run, set);
            let total = ChannelMixer::total(row);
            let warn = total_warns(total);
            let text = total_text(&units, &words, total);
            let mut readout = Field::new(words.total.clone()).id_salt("total");
            if warn {
                readout = readout.help(Help::Status(Tone::Warn, Label::from(words.total_over.fill(&[("value", &text)]))));
            }
            readout.show(ui, |ui, _| {
                let mut shown = Text::new(text.clone()).role(TypeRole::Mono);
                if warn {
                    shown = shown.tone(Tone::Warn);
                }
                shown.show(ui);
            });
            let constant = AdjustField::new("constant", words.constant.clone(), -200.0..=200.0, 0.0).unit_kind(UnitKind::Percent);
            run.slider_row(ui, cx, &constant, row[3], move |now: &ChannelMixer, x| set(now, 3, x));
            let monochrome = Checkbox::new(Bind::given(mono)).label(words.monochrome.clone()).show(ui);
            run.push(cx, monochrome, |now, on| ChannelMixer { gray: on.then_some(ChannelMixer::MONOCHROME_START), ..*now });
            let memory = cx.memory::<MixerMemory>();
            memory.began = run.began();
            memory.output = picked.unwrap_or(out);
            edit = Some(run.finish());
        });
        lifted(edit, shown.output)
    }
}

// ---------------------------------------------------------------------------------------------
// Selective Color

/// Selective Color's method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SelectiveMethod {
    /// Adjusts by a share of the colour already there; the default.
    #[default]
    Relative,
    /// Adjusts by the amount given.
    Absolute,
}

/// A Selective Color value: per range (Reds, Yellows, Greens, Cyans, Blues, Magentas, Whites, Neutrals, Blacks) the
/// Cyan, Magenta, Yellow and Black shifts, −100..=100 %, whole numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SelectiveColor {
    /// The nine ranges.
    pub ranges: [[f64; 4]; 9],
    /// Relative or Absolute.
    pub method: SelectiveMethod,
}

impl SelectiveColor {
    /// A value of the nine ranges and the method.
    pub fn new(ranges: [[f64; 4]; 9], method: SelectiveMethod) -> Self {
        Self { ranges, method }
    }
}

/// What a Selective Color editor keeps between passes: the range shown and the value its running gesture began with.
#[derive(Clone, Default)]
struct SelectiveMemory {
    range: usize,
    began: Option<SelectiveColor>,
}

/// The number of ranges.
const RANGES: usize = 9;

/// What a [`SelectiveColorEditor`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct SelectiveColorOutput {
    /// The edit of the value.
    pub edit: EditResponse<SelectiveColor>,
    /// The range shown after the pass, 0 Reds … 8 Blacks.
    pub shown_range: usize,
}

impl Outcome for SelectiveColorOutput {
    type Value = SelectiveColor;

    fn response(&self) -> &egui::Response {
        &self.edit.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.edit.story_facts()
    }
}

impl std::ops::Deref for SelectiveColorOutput {
    type Target = EditResponse<SelectiveColor>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

/// Selective Color: the Colors select, the Cyan, Magenta, Yellow and Black rows of that range, and the Relative or
/// Absolute method.
///
/// The range shown is view state of the editor, Reds at first; a row edits the range shown only.
#[must_use = "show the editor with `show`"]
#[derive(Pattern)]
pub struct SelectiveColorEditor<'a> {
    label: Label,
    value: Bind<'a, SelectiveColor>,
    context: AdjustContext,
}

impl<'a> SelectiveColorEditor<'a> {
    /// The editor named `label` of `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, SelectiveColor>) -> Self {
        Self { label: label.into(), value, context: AdjustContext::dialog(true) }
    }

    /// Dialog or Properties panel; a dialog by default.
    #[must_use]
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Shows the editor and returns this pass's edit of the `SelectiveColor` and the range shown.
    pub fn show(self, ui: &mut Ui) -> SelectiveColorOutput {
        let SelectiveColorEditor { label, mut value, context } = self;
        let words = words(ui);
        let mut out = None;
        let shown = FieldGroup::new(label).show(ui, |ui, cx| {
            let now = read(&mut value, cx).unwrap_or_default();
            let memory = cx.memory::<SelectiveMemory>().clone();
            let range = memory.range.min(RANGES - 1);
            let names = [
                &words.reds,
                &words.yellows,
                &words.greens,
                &words.cyans,
                &words.blues,
                &words.magentas,
                &words.whites,
                &words.neutrals,
                &words.blacks,
            ];
            let entries: Vec<Entry<usize>> = names.iter().enumerate().map(|(i, name)| Entry::new(i, (*name).clone())).collect();
            let fields: Vec<AdjustField> = [("cyan", &words.cyan), ("magenta", &words.magenta), ("yellow", &words.yellow), ("black", &words.black)]
                .into_iter()
                .map(|(key, name)| AdjustField::new(key, name.clone(), -100.0..=100.0, 0.0).unit_kind(UnitKind::Percent))
                .collect();
            let mut run = Run::start(&mut value, context, now, memory.began);
            let tables = Tables { name: words.colors.clone(), entries: &entries, picked: range, fields: &fields, values: &now.ranges[range] };
            let picked = tables.show(ui, cx, &mut run, move |now: &SelectiveColor, i, x| {
                let mut next = *now;
                next.ranges[range][i] = x;
                next
            });
            let mut method = None;
            Field::new(words.relative_absolute.clone()).id_salt("method").show(ui, |ui, _| {
                let group = RadioGroup::new(words.relative_absolute.clone(), Bind::given(now.method)).show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        Radio::new(SelectiveMethod::Relative).label(words.relative.clone()).show(ui);
                        Radio::new(SelectiveMethod::Absolute).label(words.absolute.clone()).show(ui);
                    });
                });
                method = Some(group.output);
            });
            if let Some(method) = method {
                run.push(cx, method, |now, method| SelectiveColor { method: *method, ..*now });
            }
            let memory = cx.memory::<SelectiveMemory>();
            memory.began = run.began();
            memory.range = picked.unwrap_or(range);
            out = Some((run.finish(), picked.unwrap_or(range)));
        });
        let (edit, shown_range) = out.expect("an editor shows its body");
        SelectiveColorOutput { edit: lifted(Some(edit), shown.output), shown_range }
    }
}
