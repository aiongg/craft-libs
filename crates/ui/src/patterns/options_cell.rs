//! [`OptionsBar::cell`]: a caption and a compact control, with a width class.

use egui::text::LayoutJob;
use std::ops::RangeInclusive;

use egui::{AsIdSalt, Id, IdSalt, Label, Response, Sense, Ui, vec2};

use super::options_bar::OptionsBar;
use crate::canvas::rulers;
use crate::controls::expr::{self, ExprUnits};
use crate::controls::{CheckState, Checkbox, NumberField, Select};
use crate::direction;
use crate::edit::EditResponse;
use crate::localized::Labels as _;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::localized::{LocaleNumbers, Unit, Word, Words};
use crate::primitives::{Content, FocusRing, Text};
use crate::story::{PatternStoryLabels, StoryArgs, StoryLabels};
use crate::theme::{Tokens, TokensOf};
use craft_theme::{ControlSize, TypeRole};

/// How wide a cell's control is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellWidth {
    /// `options_bar.field_sm`: a short number.
    Sm,
    /// `options_bar.field_md`: a number with a unit.
    Md,
    /// `options_bar.field_lg`: a long number.
    Lg,
    /// The control's own width, for one that sizes itself (a select, a checkbox).
    Content,
}

/// The caption followed by its colon; a language writes its own punctuation (French " :").
const CAPTION_COLON: Word = Word::keyed("options.captionColon", "{caption}:");

/// The words of the options bar's cells.
pub(crate) const WORDS: &[Word] = &[CAPTION_COLON];

/// `caption` with the colon of the installed language, unless it already ends in one.
fn with_colon(words: &Words, caption: &str) -> String {
    if caption.ends_with([':', '\u{ff1a}']) {
        return caption.to_owned();
    }
    words.format(CAPTION_COLON, &[("caption", caption)])
}

impl OptionsBar {
    /// A caption and the control it names. `caption` is the host's translated text ("Size"); the
    /// cell adds the colon of the installed language, and a caption that already ends in one is
    /// shown as it is. The control is laid out in a box of the class's width, `options_bar.gap`
    /// after the caption, and `add` receives the caption's response so a number field can
    /// [`scrub_by`](NumberField::scrub_by) it. The caption is not selectable and is not shortened
    /// for a narrow bar: the bar scrolls instead.
    pub fn cell<R>(ui: &mut Ui, caption: &str, width: CellWidth, add: impl FnOnce(&mut Ui, &Response) -> R) -> R {
        let tokens = Tokens::of(ui);
        let style = OptionsBar::style(&tokens, false);
        ui.spacing_mut().item_spacing.x = style.gap;
        let mut format = Text::format(&tokens, TypeRole::Body, Content::Muted);
        format.valign = ui.text_valign();
        let label = ui.add(Label::new(LayoutJob::single_section(with_colon(&Words::of(ui), caption), format)).selectable(false));
        match width {
            CellWidth::Content => add(ui, &label),
            fixed => {
                let side = match fixed {
                    CellWidth::Sm => style.fields[0],
                    CellWidth::Md => style.fields[1],
                    _ => style.fields[2],
                };
                let size = vec2(side, ui.available_height());
                ui.allocate_ui_with_layout(size, direction::row(ui).with_main_justify(true), |ui| {
                    ui.set_width(side);
                    add(ui, &label)
                })
                .inner
            }
        }
    }

    /// Hint text in the body: `text.faint`.
    pub fn hint(ui: &mut Ui, text: &str) -> Response {
        ui.add(Text::new(text).content(Content::Faint))
    }

    /// A number cell: [`cell`](Self::cell) with `field` at [`ControlSize::Sm`](craft_theme::ControlSize::Sm), scrubbed by
    /// dragging the caption ([`NumberField::scrub_by`]), as Photoshop's "Size:" or "Tolerance:".
    /// The drag area is the caption, the gap after it (up to 24 points) and nothing of the box,
    /// whose own text scrubs.
    // pc: crates/ui-egui/src/panels/options_bar.rs:735 `opt_label` (a label before a value field scrubs it).
    pub fn number_cell(ui: &mut Ui, caption: &str, width: CellWidth, field: NumberField<'_>) -> EditResponse<f64> {
        Self::cell(ui, caption, width, |ui, label| {
            let area = scrub_area(ui, label, label.id);
            field.size(ControlSize::Sm).scrub_by(&area).show(ui)
        })
    }

    /// Photoshop's Opacity and Flow cell: the caption scrubs the percentage, 0.5 percentage
    /// points per screen point measured from the press (no jump on the press), clamped to
    /// 0..=100, one edit gesture per drag; the field is 0..=100 with `%`, no decimals, and a ▾ that
    /// opens a pop-up slider ([`NumberField::popup_slider`], named after the caption). `value` is
    /// the percentage. Width [`CellWidth::Sm`].
    // pc: crates/ui-egui/src/panels/options_bar.rs:698 `scrub_pct_label`, :776 `percent_field`.
    pub fn percent_cell(ui: &mut Ui, id_salt: impl AsIdSalt, caption: &str, value: f64, numbers: &LocaleNumbers) -> EditResponse<f64> {
        let salt = IdSalt::new(id_salt);
        let field_id = ui.make_persistent_id(salt);
        Self::cell(ui, caption, CellWidth::Sm, |ui, label| {
            let area = scrub_area(ui, label, field_id);
            NumberField::new(salt, caption, value, numbers)
                .range(0.0..=100.0)
                .decimals(0..=0)
                .unit(Unit::new("{n}%"))
                .size(ControlSize::Sm)
                .scrub_by(&area)
                .scrub_rate(PERCENT_PER_POINT)
                .popup_slider(caption)
                .show(ui)
        })
    }

    /// A length cell (the marquee's Fixed Size Width and Height): `pixels` shown in `unit` with
    /// [`LengthUnit::decimals`] and its [`suffix`](LengthUnit::suffix), converted by `context`;
    /// typed text may carry another unit (`2in`) or arithmetic
    /// ([`expr::parse_with_units`](crate::controls::expr::parse_with_units)). The edit reports
    /// pixels, clamped to `range` (pixels). Leaving the field re-reads the rounded text it showed,
    /// which is not an edit: the pixels stay exact unless the shown value changed. A secondary
    /// click on the field opens the units menu of `units` (no menu when it is empty), as on the rulers
    /// ([`LengthUnitLabels`](crate::localized::units::LengthUnitLabels) names); the pick comes
    /// back in [`UnitCellResponse::unit_picked`]. Width [`CellWidth::Md`].
    // pc: crates/ui-egui/src/panels/options_bar.rs:758 `fixed_size_field`, :747 `fixed_size_units`.
    #[expect(clippy::too_many_arguments, reason = "a cell names every input of its field")]
    pub fn unit_cell(
        ui: &mut Ui,
        id_salt: impl AsIdSalt,
        caption: &str,
        pixels: f64,
        range: RangeInclusive<f64>,
        unit: LengthUnit,
        units: &[LengthUnit],
        context: UnitContext,
    ) -> UnitCellResponse {
        let salt = IdSalt::new(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let places = unit.decimals();
        let scale = 10f64.powi(places as i32);
        let round = |v: f64| (v * scale).round() / scale;
        let (low, high) = (range.start().min(*range.end()), range.start().max(*range.end()));
        let bounds = {
            let (a, b) = (context.from_pixels(low, unit).unwrap_or(f64::NEG_INFINITY), context.from_pixels(high, unit).unwrap_or(f64::INFINITY));
            a.min(b)..=a.max(b)
        };
        // The field shows (and re-reads) the rounded value, so leaving it is no change.
        let shown = context.from_pixels(pixels, unit).map_or(f64::NAN, round);
        let entries = rulers::unit_entries(ui, units, unit);
        let mut picked = None;
        let units_of_field = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context };
        let edit = Self::cell(ui, caption, CellWidth::Md, |ui, label| {
            let area = scrub_area(ui, label, label.id);
            let mut field = unit_number_field(salt, caption, shown, &numbers, &units_of_field).range(bounds).scrub_by(&area);
            // Without units there is nothing to pick, so a secondary click opens no menu.
            if !entries.is_empty() {
                field = field.unit_menu(&entries, &mut picked);
            }
            field.show(ui)
        });
        let to_pixels = |value: f64| {
            let exact = if (round(value) - shown).abs() < 0.5 / scale { pixels } else { context.to_pixels(value, unit).unwrap_or(pixels) };
            exact.clamp(low, high)
        };
        UnitCellResponse { edit: edit.map(to_pixels), unit_picked: picked.and_then(|i| units.get(i)).copied() }
    }

    /// Scroll the bar so a control that just took the keyboard focus (Tab into a cell scrolled
    /// out of a narrow bar) is in view, centred. Call once per pass inside the bar's scroll area,
    /// after the cells. Skipped on a pass with a pointer press or release (reading a popup's newly
    /// focused field during its opening click can undo the focus it just requested), and only for
    /// a focused widget on this layer whose rect lies inside the bar's content.
    // pc: crates/ui-egui/src/panels/options_bar.rs:7 `reveal_focused_control`.
    pub fn reveal_focused_control(ui: &Ui) {
        super::options_bar::reveal_focused_control(ui, FocusRing::style(&Tokens::of(ui)).outset());
    }
}

/// A [`NumberField`] for a number with a unit, as the length cells and the Character panel's size
/// fields show it: `shown` is already in the field's unit; `units.suffix` follows the number
/// (`{n} pt`, `{n}%` for a percentage, none for a plain number); typed text may carry another
/// unit (`2in`) or arithmetic ([`expr::parse_with_units`]). Decimals follow
/// [`LengthUnit::decimals`] (`0..=2` for a field with no length unit). The caller adds the range,
/// the unit menu, presets and the scrub area.
pub fn unit_number_field<'a>(
    id_salt: impl AsIdSalt,
    caption: &str,
    shown: f64,
    numbers: &'a LocaleNumbers,
    units: &ExprUnits<'_>,
) -> NumberField<'a> {
    let places = units.unit.map_or(2, LengthUnit::decimals);
    let template = match units.suffix {
        "" => "{n}".to_owned(),
        "%" => "{n}%".to_owned(),
        suffix => format!("{{n}} {suffix}"),
    };
    let (unit, suffix, context) = (units.unit, units.suffix.to_owned(), units.context);
    let reader = move |text: &str| {
        if suffix.is_empty() { expr::parse(text) } else { expr::parse_with_units(text, &ExprUnits { unit, suffix: &suffix, context }) }
    };
    NumberField::new(id_salt, caption, shown, numbers).decimals(0..=places).unit(Unit::new(&template)).or_parse(reader).size(ControlSize::Sm)
}

/// Percentage points a caption drag adds per screen point ([`NumberField::scrub_rate`]).
const PERCENT_PER_POINT: f64 = 0.5;

/// The longest gap after a caption that still scrubs.
const MAX_SCRUB_GAP: f32 = 24.0;

/// A scrub area over `caption` and the gap after it (up to [`MAX_SCRUB_GAP`], toward the field in
/// the reading direction), as a response of its own so the caption keeps its widget.
// pc: crates/ui-egui/src/widgets/scrub.rs:49 `scrub_option_area`.
fn scrub_area(ui: &Ui, caption: &Response, base: Id) -> Response {
    let gap = ui.spacing().item_spacing.x.min(MAX_SCRUB_GAP);
    let mut rect = caption.rect;
    if direction::is_rtl(ui) {
        rect.min.x -= gap;
    } else {
        rect.max.x += gap;
    }
    ui.interact(rect, base.with("craft_ui::options_cell::scrub_area"), Sense::click_and_drag())
}

/// What a [`OptionsBar::unit_cell`] did.
#[derive(Clone, Debug)]
pub struct UnitCellResponse {
    /// The field's edit, in pixels.
    pub edit: EditResponse<f64>,
    /// A unit picked from the field's units menu: the app stores it (the ruler unit preference
    /// in PhotoCraft, which the rulers share).
    pub unit_picked: Option<LengthUnit>,
}

#[derive(Clone, Debug)]
struct Demo {
    size: f64,
    opacity: f64,
    mode: String,
    sample_all: CheckState,
    home: bool,
    last: String,
}

impl Default for Demo {
    fn default() -> Self {
        Self { size: 30.0, opacity: 100.0, mode: "Normal".into(), sample_all: CheckState::Unchecked, home: false, last: String::new() }
    }
}

fn brush_cells(ui: &mut Ui, numbers: &LocaleNumbers, size: &mut f64, opacity: &mut f64, mode: &mut String) {
    let words = StoryLabels::of(ui);
    OptionsBar::cell(ui, "Size:", super::CellWidth::Sm, |ui, caption| {
        let edit = NumberField::new("demo-size", &words.size, *size, numbers)
            .range(1.0..=5000.0)
            .unit(Unit::new("{n} px"))
            .size(ControlSize::Sm)
            .scrub_by(caption)
            .show(ui);
        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
            *size = *v;
        }
    });
    OptionsBar::cell(ui, "Opacity:", super::CellWidth::Md, |ui, caption| {
        let edit = NumberField::new("demo-opacity", &words.opacity, *opacity, numbers)
            .range(1.0..=100.0)
            .unit(Unit::new("{n}%"))
            .size(ControlSize::Sm)
            .scrub_by(caption)
            .show(ui);
        if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
            *opacity = *v;
        }
    });
    OptionsBar::cell(ui, "Mode:", super::CellWidth::Content, |ui, _| {
        let _ = Select::new("demo-mode", "Mode", mode)
            .option("Normal".to_owned(), "Normal")
            .option("Multiply".to_owned(), "Multiply")
            .option("Screen".to_owned(), "Screen")
            .size(ControlSize::Sm)
            .show(ui);
    });
}

fn sample_group(ui: &mut Ui, state: &mut CheckState, disabled: bool) {
    OptionsBar::group(ui, !disabled, |ui| {
        let _ = Checkbox::new("Sample all layers", state).size(ControlSize::Sm).show(ui);
        OptionsBar::hint(ui, "Samples the visible image");
    });
}

/// The args of the options bar story.
#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct BarArgs {
    pub(super) narrow: bool,
    pub(super) session: bool,
    pub(super) disabled: bool,
    pub(super) two_rows: bool,
    #[arg(range = 160.0..=1200.0)]
    pub(super) width: f32,
}

/// The bar of PhotoCraft's brush tool, with a disabled group and, while a session is on, commit and cancel.
pub(super) fn show_story(ui: &mut Ui, args: &mut BarArgs) {
    let (narrow, session, disabled, two_rows) = (args.narrow, args.session, args.disabled, args.two_rows);
    let width = args.width.min(ui.available_width());
    let state_id = ui.scope_id().with("options-bar-demo");
    let mut demo = ui.data(|d| d.get_temp::<Demo>(state_id)).unwrap_or_default();
    let numbers = LocaleNumbers::from_tag("en-US");
    let pattern_words = PatternStoryLabels::of(ui);
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let mut finished = None;
        OptionsBar::new("options-bar-demo").narrow(narrow).show(ui, |bar| {
            bar.leading(|ui| {
                OptionsBar::home(ui, &mut demo.home, &pattern_words.home);
                OptionsBar::tool_icon(ui, crate::icons::Icon::Brush, &pattern_words.brush);
            });
            if two_rows {
                bar.row(|ui| brush_cells(ui, &numbers, &mut demo.size, &mut demo.opacity, &mut demo.mode));
                bar.row(|ui| sample_group(ui, &mut demo.sample_all, disabled));
            } else {
                bar.row(|ui| {
                    brush_cells(ui, &numbers, &mut demo.size, &mut demo.opacity, &mut demo.mode);
                    OptionsBar::divider(ui);
                    sample_group(ui, &mut demo.sample_all, disabled);
                });
            }
            if session {
                bar.trailing(|ui| finished = Some(OptionsBar::commit_cancel(ui, "Commit transform", "Cancel transform")));
            }
        });
        if let Some(done) = finished {
            if done.commit {
                demo.last = "Committed".into();
            } else if done.cancel {
                demo.last = "Cancelled".into();
            }
        }
        if !demo.last.is_empty() {
            ui.add(Text::new(demo.last.clone()).content(Content::Muted));
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}
