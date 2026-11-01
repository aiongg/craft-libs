//! `BlendRows`: Blend mode, Opacity, Lock and Fill as one block (F11.04).

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;

use craft_theme::{ControlSize, TypeRole};
use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::Content;
use craft_ui::primitives::Text;
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens};
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Label, Rect, Response, Sense, Ui, UiBuilder, vec2};

use super::panel::LayersStyle;
use super::tree::{BlendChoice, LayerLocks, LayerProperties, LockKind};

/// A drag on the "Opacity:" or "Fill:" label moves the value this many percentage points per
/// screen point.
// pc: crates/ui-egui/src/panels/options_bar.rs:698 `scrub_pct_label`.
const PERCENT_PER_POINT: f64 = 0.5;

/// The room kept free between the "Opacity:" label and the field.
// pc: crates/ui-egui/src/panels/layers.rs:39-46 `opacity_row_layout`.
const LABEL_SLACK: f32 = 16.0;

/// The blend select's least width when the row is too narrow for anything else.
// pc: crates/ui-egui/src/panels/layers.rs:39-46 `opacity_row_layout`.
const LEAST_BLEND_WIDTH: f32 = 40.0;

/// An edit of the active layer's properties, which the app applies to every selected layer (the
/// Background left out of a multi-layer change, as PhotoCraft).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayerPropertyEdit {
    /// The blend mode, by the app's key: a pick in the open list is a gesture that commits; an
    /// arrow key or a wheel notch over the closed select is a `Set`.
    Blend(EditEvent<String>),
    /// The mode under the pointer in the open blend list (`None` when it leaves the list): the
    /// app previews it on the canvas. Not an edit.
    BlendHover(Option<String>),
    /// Opacity, 0 to 100: a scrub or slider drag is a gesture (`Begin`, `Update`, `Commit` or
    /// `Cancel`, Esc restores), typing, an arrow step and a wheel step are `Set`.
    Opacity(EditEvent<f64>),
    /// Fill, 0 to 100, as Opacity.
    Fill(EditEvent<f64>),
    /// A lock button: the lock `kind` asks to be `on`.
    Lock {
        /// The lock.
        kind: LockKind,
        /// The state asked for.
        on: bool,
    },
}

craft_ui::words! {
    /// The words of the blend, opacity, lock and fill rows.
    pub struct BlendRowsLabels {
        /// The blend select's name for a screen reader.
        blend_mode: "layers.blendMode" = "Blend Mode";
        opacity: "layers.opacity" = "Opacity:";
        fill: "layers.fill" = "Fill:";
        lock: "layers.lock" = "Lock:";
        /// The lock buttons' tooltips and names, in [`LockKind::ALL`] order.
        lock_transparency: "layers.lockTransparency" = "Lock transparent pixels";
        lock_pixels: "layers.lockPixels" = "Lock image pixels";
        lock_position: "layers.lockPosition" = "Lock position";
        lock_artboard: "layers.lockArtboard" = "Prevent auto-nesting in and out of Artboards and Frames";
        lock_all: "layers.lockAll" = "Lock all";
        /// Why Lock transparent pixels is disabled, for a screen reader.
        transparency_included: "layers.lockTransparencyIncluded" = "Included in Lock image pixels";
    }
}

/// Blend mode, Opacity, Lock and Fill as one block: the two rows above the Layers panel's list,
/// usable alone (a Properties panel).
///
/// ```ignore
/// let tree = DocLayers::new(&doc);
/// let props = tree.properties();
/// let modes = tree.blend_modes(props.as_ref().is_some_and(|p| p.group), props.as_ref().map_or("normal", |p| &p.blend_key));
/// for edit in BlendRows::new("blend-rows", props.as_ref(), &modes).show(ui).events {
///     app.set_properties(edit); // every selected layer
/// }
/// ```
///
/// - **Row 1**: the blend [`Select`](craft_ui::controls::Select) (Small) with `blend_modes` and
///   their separators, then "Opacity:" as a scrub label and a Small
///   [`NumberField`](craft_ui::controls::NumberField) 0 to 100 % with its pop-up slider,
///   `layers.percent_field_width` wide. A wheel notch over the closed select steps one mode
///   (`Set` each); hovering a mode in the open list reports `BlendHover`. The "Opacity:" label
///   hides when `available < label + field + 2 gaps + 16 + layers.blend.min_width`, and the
///   select takes the rest of the row (at least 40).
/// - **Row 2** (with [`locks`](Self::locks)): "Lock:", five Small toggle
///   [`IconButton`](craft_ui::controls::IconButton)s in [`LockKind::ALL`] order (Lock image pixels
///   disables Lock transparent pixels), then "Fill:" and its field as Opacity's, right-aligned in
///   one column with it. The "Lock:" label drops before the Fill label would run over the lock
///   icons.
/// - **States**: with `properties` `None` the rows show Normal, 100 %, unlocked, all greyed. With
///   `editable` false (the Background) the select, Opacity and Fill grey; the locks stay.
/// - **Values**: in the pass after an edit the app passes back its rounded copy of the value
///   (the shared rule L19); in other passes a changed value is an outside change.
/// - **Keyboard**: each control is a Tab stop; the fields take arrows (with Shift by 10) and
///   typing; Esc during a scrub cancels it.
/// - **AccessKit**: a `ComboBox` "Blend Mode"; spin buttons "Opacity" and "Fill" with the value
///   and the % unit; the lock buttons are toggle buttons named by their tooltip with their pressed
///   state, the disabled one described by [`BlendRowsLabels::transparency_included`].
/// - Right to left, the rows mirror; number text keeps its digits' direction.
#[must_use = "show the rows with `show`"]
pub struct BlendRows<'a> {
    id_salt: IdSalt,
    properties: Option<&'a LayerProperties>,
    blend_modes: &'a [BlendChoice],
    locks: bool,
    labels: Option<BlendRowsLabels>,
}

/// What [`BlendRows::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the edits"]
pub struct BlendRowsOutput {
    /// The edits of this pass, in order.
    pub events: Vec<LayerPropertyEdit>,
    /// The block.
    pub response: Response,
}

impl<'a> BlendRows<'a> {
    /// The rows for the active layer's `properties` (`None`: no active layer), offering
    /// `blend_modes`; `id_salt` keeps the fields' gestures.
    pub fn new(id_salt: impl AsIdSalt, properties: Option<&'a LayerProperties>, blend_modes: &'a [BlendChoice]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), properties, blend_modes, locks: true, labels: None }
    }

    /// Shows the Lock and Fill row (default true).
    pub fn locks(mut self, locks: bool) -> Self {
        self.locks = locks;
        self
    }

    /// The rows' words; the installed words by default.
    pub fn labels(mut self, labels: BlendRowsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the rows over the available width.
    pub fn show(self, ui: &mut Ui) -> BlendRowsOutput {
        // pc: crates/ui-egui/src/panels/layers.rs:104-235 (blend and Opacity row, Lock and Fill row);
        // pc: crates/ui-egui/src/panels/layers.rs:51-102 `inactive_blend_rows` (no layer).
        let Self { id_salt, properties, blend_modes, locks, labels } = self;
        let labels = labels.unwrap_or_else(|| BlendRowsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = LayersStyle::of(&tokens);
        let numbers = LocaleNumbers::of(ui);
        let live = properties.is_some();
        let fallback = LayerProperties {
            blend_key: String::from("normal"),
            group: false,
            opacity: 100.0,
            fill: 100.0,
            locks: LayerLocks::default(),
            editable: true,
        };
        let props = properties.unwrap_or(&fallback);
        let editable = live && props.editable;

        let size = ControlSize::Sm;
        let side = IconButton::style(&tokens, size).side;
        let height = NumberField::style(&tokens, size).field.height.max(side);
        let gap = ui.spacing().item_spacing.x;
        let avail = ui.available_width();
        let field_width = style.percent_field_width;

        let opacity_text = labels.opacity.as_str();
        let fill_text = labels.fill.as_str();
        let lock_text = labels.lock.as_str();
        let (opacity_job, fill_job, lock_job) =
            (label_job(&tokens, ui, opacity_text), label_job(&tokens, ui, fill_text), label_job(&tokens, ui, lock_text));
        let width_of = |ui: &Ui, job: &LayoutJob| ui.fonts_mut(|fonts| fonts.layout_job(job.clone())).size().x;
        let (opacity_width, fill_width, lock_width) = (width_of(ui, &opacity_job), width_of(ui, &fill_job), width_of(ui, &lock_job));

        let mut events = Vec::new();
        let block = ui.vertical(|ui| {
            // Row 1: the blend select, then "Opacity:" and its field at the end.
            let (blend_width, show_opacity) = opacity_row_layout(avail, opacity_width, field_width, gap, style.blend_min_width);
            ui.add_enabled_ui(editable, |ui| {
                let row = row_rect(ui, avail, height);
                let mut key = props.blend_key.clone();
                let mut hovered: Option<String> = None;
                let mut select = Select::new((id_salt, "blend"), labels.blend_mode.as_str(), &mut key)
                    .wheel_steps(true)
                    .highlighted(&mut hovered)
                    .width(blend_width)
                    .size(size);
                for choice in blend_modes {
                    if choice.separator_before {
                        select = select.separator();
                    }
                    select = select.option(choice.key.clone(), choice.label.as_str());
                }
                let picked = place(ui, span(ui, row, 0.0, blend_width), |ui| select.show(ui));
                events.extend(picked.event.map(LayerPropertyEdit::Blend));
                let hover_id = ui.make_persistent_id((id_salt, "blend-hover"));
                let before = ui.data(|d| d.get_temp::<Option<String>>(hover_id)).flatten();
                if before != hovered {
                    events.push(LayerPropertyEdit::BlendHover(hovered.clone()));
                    ui.data_mut(|d| d.insert_temp(hover_id, hovered));
                }
                let spec = PercentField {
                    salt: (id_salt, "opacity"),
                    name: opacity_text,
                    caption: show_opacity.then_some((&opacity_job, opacity_width)),
                    value: props.opacity,
                    gap,
                    field_width,
                };
                let edit = percent_field(ui, spec, &numbers, row);
                events.extend(edit.map(LayerPropertyEdit::Opacity));
            });
            if !locks {
                return;
            }
            // Row 2: "Lock:" and the lock buttons, then "Fill:" and its field in Opacity's column.
            let row = row_rect(ui, avail, height);
            let icons_width = 5.0 * side;
            let fits = lock_width + icons_width + gap + fill_width + field_width + 2.0 * gap <= avail;
            let icons_at = if fits {
                place(ui, span(ui, row, 0.0, lock_width), |ui| ui.add(Label::new(lock_job.clone()).selectable(false)));
                lock_width + gap
            } else {
                0.0
            };
            let described = labels.transparency_included.as_str();
            let lock_labels = [&labels.lock_transparency, &labels.lock_pixels, &labels.lock_position, &labels.lock_artboard, &labels.lock_all];
            let icons = [Icon::Grid, Icon::Brush, Icon::Move, Icon::Scan, Icon::Lock];
            for (i, ((kind, icon), text)) in LockKind::ALL.into_iter().zip(icons).zip(lock_labels).enumerate() {
                let was = props.locks.get(kind);
                let mut on = was;
                let enabled = live && (kind != LockKind::Transparency || !props.locks.pixels);
                let button = IconButton::new(icon, text.as_str()).size(size).toggle(&mut on).enabled(enabled).id_salt((id_salt, "lock", i));
                let response = place(ui, span(ui, row, icons_at + i as f32 * side, side), |ui| ui.add(button));
                if live && !enabled && response.ctx.has_accesskit_node(response.id) {
                    response.ctx.accesskit_node_builder(response.id, |node| node.set_description(described));
                }
                if on != was {
                    events.push(LayerPropertyEdit::Lock { kind, on });
                }
            }
            ui.add_enabled_ui(editable, |ui| {
                let spec = PercentField {
                    salt: (id_salt, "fill"),
                    name: fill_text,
                    caption: Some((&fill_job, fill_width)),
                    value: props.fill,
                    gap,
                    field_width,
                };
                let edit = percent_field(ui, spec, &numbers, row);
                events.extend(edit.map(LayerPropertyEdit::Fill));
            });
        });
        BlendRowsOutput { events, response: block.response }
    }
}

/// The label of a row, in the body role and the muted content colour.
fn label_job(tokens: &UiTokens, ui: &Ui, text: &str) -> LayoutJob {
    let mut format = Text::format(tokens, TypeRole::Body, Content::Muted);
    format.valign = ui.text_valign();
    LayoutJob::single_section(text.to_owned(), format)
}

/// The room of one row: the available width and `height`.
fn row_rect(ui: &mut Ui, width: f32, height: f32) -> Rect {
    ui.allocate_exact_size(vec2(width, height), Sense::hover()).0
}

/// The part of `row` that starts `offset` from the reading start and is `width` wide.
fn span(ui: &Ui, row: Rect, offset: f32, width: f32) -> Rect {
    let (min, max) =
        if direction::is_rtl(ui) { (row.right() - offset - width, row.right() - offset) } else { (row.left() + offset, row.left() + offset + width) };
    Rect::from_min_max(egui::pos2(min, row.top()), egui::pos2(max, row.bottom()))
}

/// Shows `add` in `rect`, in reading order and centred vertically.
fn place<R>(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layout = direction::row(ui);
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), add).inner
}

/// What [`percent_field`] shows.
struct PercentField<'a, S> {
    salt: S,
    /// The caption as the app words it ("Opacity:"), whose trailing colon the field's name drops.
    name: &'a str,
    /// The caption that scrubs the field and its width; `None` while it is hidden.
    caption: Option<(&'a LayoutJob, f32)>,
    value: f64,
    /// Between the caption and the field.
    gap: f32,
    field_width: f32,
}

/// A 0 to 100 % field with its pop-up slider at the end of `row`, and its caption scrubbing it
/// just before; returns the field's edit.
fn percent_field(ui: &mut Ui, spec: PercentField<'_, impl AsIdSalt>, numbers: &LocaleNumbers, row: Rect) -> Option<EditEvent<f64>> {
    let PercentField { salt, name, caption, value, gap, field_width } = spec;
    let name = name.trim_end_matches([':', '\u{ff1a}']);
    let field_at = row.width() - field_width;
    let caption = caption.map(|(job, width)| {
        let rect = span(ui, row, field_at - gap - width, width);
        place(ui, rect, |ui| ui.add(Label::new(job.clone()).selectable(false)))
    });
    let mut number = NumberField::new(salt, name, value, numbers)
        .range(0.0..=100.0)
        .decimals(0..=0)
        .unit(Unit::new("{n}%"))
        .size(ControlSize::Sm)
        .width(field_width)
        .scrub_rate(PERCENT_PER_POINT)
        .popup_slider(name);
    if let Some(caption) = &caption {
        number = number.scrub_by(caption);
    }
    place(ui, span(ui, row, field_at, field_width), |ui| number.show(ui)).event
}

/// The width of the blend select and whether the "Opacity:" label shows, for an `available`
/// row width, the label's width, the field's width (`layers.percent_field_width`, the whole box with its pop-up
/// arrow), the gap and the select's least width (`layers.blend.min_width`, 80): the label shows
/// while `available >= label + field + 2 × gap + 16 + min_blend`; the select gets
/// `available - field - gap - (label + gap + 16 when shown)`, at least 40.
pub(crate) fn opacity_row_layout(available: f32, label_width: f32, field_width: f32, gap: f32, min_blend: f32) -> (f32, bool) {
    // pc: crates/ui-egui/src/panels/layers.rs:39-46 `opacity_row_layout`.
    let show_label = available >= label_width + field_width + 2.0 * gap + LABEL_SLACK + min_blend;
    let reserved = field_width + gap + if show_label { label_width + gap + LABEL_SLACK } else { 0.0 };
    ((available - reserved).max(LEAST_BLEND_WIDTH), show_label)
}
