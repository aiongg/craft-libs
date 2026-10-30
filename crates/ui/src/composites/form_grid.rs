//! [`FormGrid`]: the two-column grid of a dialog or a preferences page, labels in the first
//! column and controls in the second.

use craft_theme::{ControlSize, TypeRole};
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, IdSalt, InnerResponse, Label, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use super::field::{FieldControl, name_control, one_line};
use super::measure;
use super::property_row::RowLabel;
use crate::controls::{Checkbox, IconButton, NumberField, Select};
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::LocaleNumbers;
use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A form grid's gaps, label column and label look, from [`FormGrid::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FormStyle {
    /// Between the label column and the control column: `form.column_gap`, or
    /// `form.roomy.column_gap`.
    pub column_gap: f32,
    /// Between two rows: `form.row_gap`, or `form.roomy.row_gap`.
    pub row_gap: f32,
    /// The least width of the label column: `form.label_min`, or `form.roomy.label_min`.
    pub label_min: f32,
    /// Where a label sits on the first pass of a row, before the control it names is measured: it
    /// is centred on a band this high from the top of the row (`field.height.md`). From the next
    /// pass a label is centred on its control, whatever the control's size.
    pub label_band: f32,
    /// The label's type role: `label`.
    pub label_role: TypeRole,
    /// The label: `text.muted`.
    pub label: Color32,
}

/// Where a label sits in the label column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelAlign {
    /// At the reading start, as in a preferences page.
    Start,
    /// Against the controls, as in a Photoshop-style dialog.
    #[default]
    End,
}

/// A two-column form: each row is a label and the control beside it.
///
/// The label column is as wide as the widest label (and at least `form.label_min`), the control
/// column takes the rest. A label is centred on the first control height of its row, and names
/// the row's control for AccessKit (`labelled_by`). Right to left, the columns swap.
///
/// ```
/// # use craft_ui::composites::FormGrid;
/// # use craft_ui::controls::Checkbox;
/// # egui::__run_test_ui(|ui| { let mut resample = true;
/// FormGrid::new("size").show(ui, |form| {
///     form.value("Dimensions:", "1200 px × 800 px");
///     form.unlabelled(|ui| Checkbox::bool("Resample", &mut resample).show(ui));
/// });
/// # });
/// ```
#[derive(Clone, Copy, Debug)]
#[must_use = "show the grid with `show`"]
pub struct FormGrid {
    id_salt: IdSalt,
    roomy: bool,
    align: LabelAlign,
    label_min: Option<f32>,
}

const TAG: &str = "FormGrid";

impl FormGrid {
    /// A grid. `id_salt` must differ between the grids under one parent.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), roomy: false, align: LabelAlign::End, label_min: None }
    }

    /// The preferences-page spacing (`form.roomy.*`) in place of the dialog's.
    pub fn roomy(mut self, roomy: bool) -> Self {
        self.roomy = roomy;
        self
    }

    /// Where labels sit in their column. The default is [`LabelAlign::End`].
    pub fn label_align(mut self, align: LabelAlign) -> Self {
        self.align = align;
        self
    }

    /// The least width of the label column, in place of the token's.
    pub fn min_label_width(mut self, width: f32) -> Self {
        self.label_min = Some(width);
        self
    }

    /// The gaps and label look of a grid.
    pub fn style(tokens: &UiTokens, roomy: bool) -> FormStyle {
        let f = &tokens.component.form;
        let spacing = if roomy { f.roomy } else { f.normal };
        FormStyle {
            column_gap: spacing.column_gap,
            row_gap: spacing.row_gap,
            label_min: spacing.label_min,
            label_band: tokens.component.field.height.md,
            label_role: TypeRole::Label,
            label: Content::Muted.color(tokens).to_egui(),
        }
    }

    /// Shows the grid; `add_rows` adds its rows to the [`Form`] it is given.
    pub fn show<R>(self, ui: &mut Ui, add_rows: impl FnOnce(&mut Form<'_>) -> R) -> InnerResponse<R> {
        let tokens = Tokens::of(ui);
        let mut style = Self::style(&tokens, self.roomy);
        style.label_min = self.label_min.unwrap_or(style.label_min);
        let (id_salt, align) = (self.id_salt, self.align);
        measure::group(ui, TAG, id_salt, |ui| {
            let spacing = ui.spacing().item_spacing;
            ui.spacing_mut().item_spacing.y = style.row_gap;
            add_rows(&mut Form { ui, style, align, tokens, spacing, rows: 0, link: None })
        })
    }
}

/// The rows of a [`FormGrid`] being shown.
pub struct Form<'a> {
    ui: &'a mut Ui,
    style: FormStyle,
    align: LabelAlign,
    tokens: std::sync::Arc<UiTokens>,
    /// The item spacing outside the grid, which a control cell keeps.
    spacing: egui::Vec2,
    /// Rows shown so far, to key each row's measured control centre.
    rows: usize,
    /// The rows being added inside [`Form::linked`].
    link: Option<Linked>,
}

/// What the rows of a [`Form::linked`] scope measured, for the bracket drawn after them.
struct Linked {
    /// The room kept at the control column's end for the bracket.
    reserve: f32,
    /// The y of each row's first control's centre.
    centres: Vec<f32>,
    /// The x furthest towards the reading end that a row's controls reach.
    edge: Option<f32>,
}

impl Form<'_> {
    /// A row of `label` and the control `add_control` draws in a row of the control column; the
    /// control it returns is the one the label names and centres on. Controls after the first in
    /// the cell (a unit menu, a caption) sit beside it.
    ///
    /// The closure gets the row's [`RowLabel`], whose [`scrub`](RowLabel::scrub) is the label's
    /// response for [`NumberField::scrub_by`], so that a label scrubs its field:
    ///
    /// ```
    /// # use craft_ui::composites::FormGrid;
    /// # use craft_ui::controls::NumberField;
    /// # use craft_ui::localized::LocaleNumbers;
    /// # egui::__run_test_ui(|ui| { let numbers = LocaleNumbers::from_tag("en");
    /// FormGrid::new("shape").show(ui, |form| {
    ///     form.row("Radius:", |ui, label| {
    ///         let field = NumberField::new("radius", "Radius", 4.0, &numbers);
    ///         match label.scrub() {
    ///             Some(label) => field.scrub_by(label),
    ///             None => field,
    ///         }
    ///         .show(ui)
    ///     });
    /// });
    /// # });
    /// ```
    ///
    /// A cell of several controls returns their responses as a tuple, `(select.show(ui),
    /// field.show(ui))`: they sit side by side, an item gap apart, and the label names and
    /// centres on the first. A cell that returns a
    /// [`Row`](crate::primitives::Row)'s response instead is an accessible group named by the
    /// label.
    ///
    /// The label replaces the control's own accessible name: a text field labelled "Name" in the
    /// row "Name:" is named "Name:". A control that carries a name of its own that the row label
    /// must not replace, such as a colour swatch named "Fill color" in the row "Color:", is
    /// marked with [`FieldControl::own_name`] (`swatch.show(ui).own_name()`). It keeps its name
    /// and is `described_by` the row label instead. The label still centres on it.
    pub fn row<C: FieldControl>(&mut self, label: impl Into<String>, add_control: impl FnOnce(&mut Ui, &RowLabel) -> C) -> InnerResponse<C> {
        self.cell(Some(label.into()).filter(|l| !l.is_empty()), add_control, true)
    }

    /// A row with no label, whose control starts at the control column.
    pub fn unlabelled<C: FieldControl>(&mut self, add_control: impl FnOnce(&mut Ui) -> C) -> InnerResponse<C> {
        self.cell(None, |ui, _| add_control(ui), true)
    }

    /// A row of `label` and a read-only `text` in the normal text colour. The text is not a
    /// control, so the label does not name it.
    pub fn value(&mut self, label: impl Into<String>, text: impl Into<String>) -> Response {
        let format = role_format(&self.tokens, TypeRole::Body, Content::Normal.color(&self.tokens).to_egui());
        let job = LayoutJob::single_section(text.into(), format);
        self.cell(Some(label.into()).filter(|l| !l.is_empty()), |ui, _| ui.add(Label::new(job).selectable(false)), false).inner
    }

    /// Two consecutive rows joined by a chain bracket, as Width and Height are in Photoshop's
    /// Image Size: `add_rows` adds the rows, and a bracket runs from the first row's control
    /// centre to the second's, past the end of whatever the rows hold (unit menus included), with
    /// a chain-link toggle in its middle. `on` is the link; the bracket shows while it is on.
    /// `name` names the toggle for AccessKit and is its tooltip.
    ///
    /// The toggle is a Tab stop after the two rows. Right to left the bracket sits at the left.
    /// The scope takes the room of the bracket from the control column, so the rows are narrower
    /// by a column gap and a button.
    ///
    /// ```
    /// # use craft_ui::composites::FormGrid;
    /// # use craft_ui::controls::NumberField;
    /// # use craft_ui::localized::LocaleNumbers;
    /// # egui::__run_test_ui(|ui| { let numbers = LocaleNumbers::from_tag("en"); let mut constrain = true;
    /// FormGrid::new("size").show(ui, |form| {
    ///     form.linked("Constrain proportions", &mut constrain, |form| {
    ///         form.row("Width:", |ui, _| NumberField::new("w", "Width", 1200.0, &numbers).show(ui));
    ///         form.row("Height:", |ui, _| NumberField::new("h", "Height", 800.0, &numbers).show(ui));
    ///     });
    /// });
    /// # });
    /// ```
    ///
    /// The response is the toggle's; `changed()` is set on the frame it flips.
    pub fn linked<R>(&mut self, name: impl Into<String>, on: &mut bool, add_rows: impl FnOnce(&mut Form<'_>) -> R) -> InnerResponse<R> {
        let side = self.tokens.component.icon_button.size.get(ControlSize::Md);
        debug_assert!(self.link.is_none(), "FormGrid::linked scopes do not nest");
        self.link = Some(Linked { reserve: self.style.column_gap + side, centres: Vec::new(), edge: None });
        let inner = add_rows(self);
        let (centres, edge) = self.link.take().map_or((Vec::new(), None), |l| (l.centres, l.edge));
        debug_assert_eq!(centres.len(), 2, "FormGrid::linked joins exactly two rows");
        let (Some(&first), Some(&last), Some(edge)) = (centres.first(), centres.last(), edge) else {
            return InnerResponse::new(inner, self.ui.allocate_rect(Rect::NOTHING, Sense::hover()));
        };

        let ui = &mut *self.ui;
        let fwd = direction::forward(ui);
        let x = edge + fwd * (self.style.column_gap + side / 2.0);
        let middle = pos2(x, (first + last) / 2.0);
        let button = Rect::from_center_size(middle, vec2(side, side));
        let name = name.into();
        let icon = if *on { Icon::Link } else { Icon::Unlink };
        let response = IconButton::new(icon, name).toggle(on).id_salt(("form-link", self.rows)).show_at(ui, button);
        if *on {
            let stroke = Stroke::new(craft_theme::Stroke::W1.width(ui.pixels_per_point()), self.tokens.color.line.mark.to_egui());
            let start = edge + fwd * self.style.column_gap;
            let painter = ui.painter();
            painter.line_segment([pos2(start, first), pos2(x, first)], stroke);
            painter.line_segment([pos2(start, last), pos2(x, last)], stroke);
            painter.line_segment([pos2(x, first), pos2(x, button.top().max(first))], stroke);
            painter.line_segment([pos2(x, last), pos2(x, button.bottom().min(last))], stroke);
        }
        ui.expand_to_include_rect(button);
        InnerResponse::new(inner, response)
    }

    /// Extra space before the next row.
    pub fn space(&mut self, height: f32) {
        self.ui.add_space(height);
    }

    fn cell<C: FieldControl>(
        &mut self,
        label: Option<String>,
        add_control: impl FnOnce(&mut Ui, &RowLabel) -> C,
        names_control: bool,
    ) -> InnerResponse<C> {
        let style = self.style;
        let ui = &mut *self.ui;
        let format = role_format(&self.tokens, style.label_role, style.label);
        let galley = label.map(|text| one_line(ui, text, format));
        let rect = ui.available_rect_before_wrap();
        let rtl = direction::is_rtl(ui);
        let column = measure::column(ui, TAG, galley.as_ref().map_or(0.0, |g| g.size().x)).0.max(style.label_min);
        let indent = column + style.column_gap;
        let reserve = self.link.as_ref().map_or(0.0, |l| l.reserve);
        let control_rect = if rtl {
            Rect::from_min_max(pos2(rect.left() + reserve, rect.top()), pos2(rect.right() - indent, rect.bottom()))
        } else {
            Rect::from_min_max(pos2(rect.left() + indent, rect.top()), pos2(rect.right() - reserve, rect.bottom()))
        };

        // The label comes first, so that the control closure has its response (scrubbing) and
        // AccessKit and Tab list it before the control. It is centred on the control's centre
        // as measured on the last pass; a change discards the pass.
        let centre_id = ui.make_persistent_id(("form-row-centre", self.rows));
        self.rows += 1;
        let centre = ui.data(|d| d.get_temp::<f32>(centre_id));
        let mut row = Rect::NOTHING;
        let label_response = galley.map(|galley| {
            let x = if rtl { rect.right() - column } else { rect.left() };
            let height = (2.0 * centre.unwrap_or(style.label_band / 2.0)).max(galley.size().y);
            let band = Rect::from_min_size(pos2(x, rect.top()), vec2(column, height));
            let layout = match self.align {
                LabelAlign::End => direction::trailing(ui),
                LabelAlign::Start => direction::row(ui),
            };
            ui.scope_builder(UiBuilder::new().max_rect(band).layout(layout), |ui| ui.add(Label::new(galley).selectable(false))).inner
        });
        if let Some(label) = &label_response {
            row = label.rect;
        }
        let mut cell = ui.new_child(UiBuilder::new().max_rect(control_rect).layout(direction::column(ui)));
        cell.spacing_mut().item_spacing = self.spacing;
        let control = direction::horizontal(&mut cell, |ui| add_control(ui, &RowLabel::new(label_response.as_ref(), true))).inner;
        row = row.union(cell.min_rect());
        if let Some(link) = &mut self.link {
            link.centres.push(control.control().rect.center().y);
            let end = direction::end_x(ui, cell.min_rect());
            link.edge = Some(link.edge.map_or(end, |e| if rtl { e.min(end) } else { e.max(end) }));
        }
        row.extend_with_x(direction::start_x(ui, rect));

        if label_response.is_some() {
            let measured = control.control().rect.center().y - rect.top();
            let used = centre.unwrap_or(style.label_band / 2.0);
            if centre.is_none_or(|c| (c - measured).abs() > 0.1) {
                ui.data_mut(|d| d.insert_temp(centre_id, measured));
            }
            if (used - measured).abs() > 0.1 {
                ui.ctx().request_discard("FormGrid label centre");
            }
        }
        if names_control && let Some(label) = &label_response {
            name_control(ui.ctx(), &control, label);
        }
        drop(cell);
        let response = ui.allocate_rect(row, Sense::hover());
        InnerResponse::new(control, response)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FormGrid"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/form-grid/image-size",
        Level::Composites,
        "FormGrid",
        "Image size",
        ImageSizeArgs { roomy: false, labels_at_start: false },
        show_image_size
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct ImageSizeArgs {
    pub(super) roomy: bool,
    #[arg(name = "labels at start")]
    pub(super) labels_at_start: bool,
}

const UNITS: [&str; 4] = ["Pixels", "Percent", "Inches", "Centimeters"];
const METHODS: [&str; 3] = ["Automatic", "Preserve Details", "Bicubic"];

/// The values of the Image Size story.
#[derive(Clone, Copy, Debug)]
struct Values {
    width: f64,
    height: f64,
    resolution: f64,
    unit: usize,
    constrain: bool,
    resample: bool,
    method: usize,
}

fn apply(event: Option<EditEvent<f64>>, value: &mut f64) {
    if let Some(EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. } | EditEvent::Commit { value: v, .. }) = event {
        *value = v;
    }
}

/// The content of PhotoCraft's Image Size dialog: two read-outs, Width and Height with a unit
/// menu each and a chain bracket linking them, Resolution with its unit, and the Resample check box with its method.
pub(super) fn show_image_size(ui: &mut Ui, args: &mut ImageSizeArgs) {
    let (roomy, start) = (args.roomy, args.labels_at_start);
    let state_id = ui.scope_id().with("form-grid-image-size");
    let mut v = ui.data(|d| d.get_temp::<Values>(state_id)).unwrap_or(Values {
        width: 1200.0,
        height: 800.0,
        resolution: 72.0,
        unit: 0,
        constrain: true,
        resample: true,
        method: 0,
    });
    let numbers = LocaleNumbers::from_tag("en");
    let unit_menu = |ui: &mut Ui, id: &'static str, label: &'static str, unit: &mut usize| {
        let mut select = Select::new(id, label, unit);
        for (i, name) in UNITS.into_iter().enumerate() {
            select = select.option(i, name);
        }
        select.show(ui)
    };
    FormGrid::new("form-grid-image-size")
        .roomy(roomy)
        .label_align(if start { LabelAlign::Start } else { LabelAlign::End })
        .min_label_width(90.0)
        .show(ui, |form| {
            let bytes = |w: f64, h: f64| format!("{:.1}M", w * h * 4.0 / (1024.0 * 1024.0));
            form.value("Image Size:", format!("{} (was {})", bytes(v.width, v.height), bytes(1200.0, 800.0)));
            form.value("Dimensions:", format!("{} px × {} px", v.width.round(), v.height.round()));
            form.space(6.0);
            let mut constrain = v.constrain;
            let (mut width, mut height) = (None, None);
            form.linked("Constrain proportions", &mut constrain, |form| {
                width = form
                    .row("Width:", |ui, _| {
                        let edit = NumberField::new("width", "Width", v.width, &numbers).range(1.0..=300000.0).decimals(0..=0).show(ui);
                        let _ = unit_menu(ui, "width-unit", "Width unit", &mut v.unit);
                        edit
                    })
                    .inner
                    .event;
                height = form
                    .row("Height:", |ui, _| {
                        let edit = NumberField::new("height", "Height", v.height, &numbers).range(1.0..=300000.0).decimals(0..=0).show(ui);
                        let _ = unit_menu(ui, "height-unit", "Height unit", &mut v.unit);
                        edit
                    })
                    .inner
                    .event;
            });
            v.constrain = constrain;
            apply(width, &mut v.width);
            apply(height, &mut v.height);
            let resolution = form
                .row("Resolution:", |ui, _| {
                    let edit = NumberField::new("resolution", "Resolution", v.resolution, &numbers).range(1.0..=30000.0).decimals(0..=0).show(ui);
                    ui.label("Pixels/Inch");
                    edit
                })
                .inner;
            apply(resolution.event, &mut v.resolution);
            form.space(4.0);
            form.unlabelled(|ui| {
                let check = Checkbox::bool("Resample", &mut v.resample).show(ui);
                if v.resample {
                    let mut select = Select::new("resample-method", "Resample method", &mut v.method);
                    for (i, name) in METHODS.into_iter().enumerate() {
                        select = select.option(i, name);
                    }
                    let _ = select.show(ui);
                }
                check
            });
        });
    ui.data_mut(|d| d.insert_temp(state_id, v));
}
