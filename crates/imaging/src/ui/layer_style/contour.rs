//! [`ContourField`]: an effect's Contour row, its preset picker and the Contour Editor (task
//! AD-10).

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Context, Id, IdSalt, Modifiers, Rect, Response, Sense, Stroke, Ui, Vec2, vec2};

use craft_theme::TypeRole;
use craft_ui::behavior::GridNav;
use craft_ui::composites::{Curve, CurveEditor, CurvePreview};
use craft_ui::controls::{Button, ButtonVariant, Checkbox, Select, TextField};
use craft_ui::dialog::{Dialog, DialogState, Footer, Outcome};
use craft_ui::direction;
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverCtx};
use craft_ui::primitives::{Content, Divider, Gap, Interactive, Row, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};

use super::labels::ContourFieldLabels;
use super::style::ContourFieldStyle;
use crate::layer_style::{Contour, ContourPoint};

/// Cells to a row of the preset picker (PhotoCraft's grid).
const COLUMNS: usize = 4;

/// What the picker's menu or the editor asked of the host's contour library. The library itself
/// (and its `.shc` files) is the host's.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ContourAction {
    /// Load Contours…: add a file's contours to the library.
    Load,
    /// Replace Contours…: replace the library with a file's.
    Replace,
    /// Save Contours…: save the library (only offered when it has contours).
    Save,
    /// Delete Contour on the host's contour at this index of
    /// [`user_contours`](ContourField::user_contours).
    Delete(usize),
    /// New… in the editor: the edited curve under the name the user typed, to add to the library.
    New(Contour),
}

/// What a [`ContourField`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ContourFieldResponse {
    /// The thumbnail.
    pub response: Response,
    /// The contour's edits: a preset or library pick is one `Set`; the Contour Editor is one
    /// gesture from opening (`Begin`) through its edits (`Update`, so the preview follows) to OK
    /// (`Commit`) or Cancel and Esc (`Cancel`: the host restores the contour it had at `Begin`).
    /// An edited built-in preset is named "Custom" ([`Contour::edited`]).
    pub edit: Option<EditEvent<Contour>>,
    /// The Anti-aliased check's new value, when clicked.
    pub anti_alias: Option<bool>,
    /// A library request from the picker's menu or the editor.
    pub action: Option<ContourAction>,
}

/// An effect's Contour row: the label, a thumbnail of the curve (a click opens the Contour
/// Editor), the preset picker's arrow and, for effects that have one, the Anti-aliased check.
///
/// - Picker (the arrow; a popover): the built-in presets ([`Contour::presets`], Photoshop's
///   order) then the host's contours, as `layer_style.contour_cell` cells with the curve drawn in
///   each and the name as tooltip; arrows move, Enter or a click picks (one `Set`) and closes. A
///   menu at its end offers Load Contours…, Replace Contours…, Save Contours… (disabled with no
///   host contours); a host contour's context menu offers Delete Contour.
/// - Contour Editor (a modal over the dialog): Preset select (the built-in names), a
///   `layer_style.contour_graph` square `CurveEditor` with corner points (levels 0..=255, at most
///   [`Contour::MAX_POINTS`] points), Input and Output fields in percent and a Corner check for
///   the selected point, OK and Cancel, New… (asks a name, then [`ContourAction::New`]), Load…
///   and Save… (as the picker's menu).
/// - AccessKit: the thumbnail is a `Button` named [`ContourFieldLabels::editor`] whose value is
///   the contour's name; the arrow a `Button` named [`ContourFieldLabels::presets`] with a popup.
#[must_use = "show the field with `show`"]
pub struct ContourField<'a> {
    id_salt: IdSalt,
    label: String,
    contour: &'a Contour,
    anti_alias: Option<bool>,
    user_contours: &'a [Contour],
    labels: Option<ContourFieldLabels>,
    enabled: bool,
}

impl<'a> ContourField<'a> {
    /// The row named `label` ("Contour", "Gloss Contour"; empty for Bevel & Emboss's contour
    /// under its check) over `contour`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, contour: &'a Contour) -> Self {
        ContourField {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            contour,
            anti_alias: None,
            user_contours: &[],
            labels: None,
            enabled: true,
        }
    }

    /// Shows the Anti-aliased check with this value; hidden by default.
    pub fn anti_alias(mut self, on: Option<bool>) -> Self {
        self.anti_alias = on;
        self
    }

    /// The host's contour library, listed after the built-in presets.
    pub fn user_contours(mut self, contours: &'a [Contour]) -> Self {
        self.user_contours = contours;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ContourFieldLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the row, and the picker and the editor while open (their state is the field's, in
    /// egui memory under its id).
    pub fn show(self, ui: &mut Ui) -> ContourFieldResponse {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:93-222 `row` (thumbnail 36, arrow,
        // picker grid of 40 cells, Load/Replace/Save Contours…, Delete Contour, Anti-aliased),
        // :323-516 `editor` (Preset select, 220 graph, Input/Output, Corner, OK, Cancel, New…, Load…,
        // Save…), :306-320 `follow_corners`.
        let Self { id_salt, label, contour, anti_alias, user_contours, labels, enabled } = self;
        let tokens = Tokens::of(ui);
        let style = ContourField::style(&tokens);
        let accent = tokens.color.accent.strong.to_egui();
        let labels = labels.unwrap_or_else(|| ContourFieldLabels::of(ui));
        let id = ui.make_persistent_id(id_salt);
        let enabled = enabled && ui.is_enabled();
        let presets = Contour::presets();
        let mut out_edit = None;
        let mut out_action = None;
        let mut out_anti_alias = None;

        let thumb = Row::new(Gap::Inline)
            .show(ui, |ui| {
                if !label.is_empty() {
                    ui.add(Text::new(label.as_str()).role(TypeRole::Label).content(Content::Muted));
                }
                let thumb = thumbnail(ui, id, &style, &labels, contour, accent, enabled);
                let arrow = arrow_button(ui, id, &style, &labels, &tokens, enabled);
                let open_id = id.with("picker-open");
                let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
                if arrow.clicked() {
                    open = !open;
                }
                let input =
                    PickerInput { id, style: &style, labels: &labels, current: contour, presets: &presets, user: user_contours, accent, enabled };
                let picked = Popover::new(id.with("picker"), PopoverAnchor::Control(arrow.clone()))
                    .toggle(arrow.id)
                    .label(labels.presets.as_str())
                    .show(ui, &mut open, |ui, pop| picker(ui, pop, &input))
                    .map(|shown| shown.inner);
                ui.data_mut(|d| d.insert_temp(open_id, open));
                if let Some(picked) = picked {
                    if let Some(pick) = picked.contour {
                        out_edit = Some(EditEvent::set(pick, Modifiers::NONE));
                    }
                    out_action = picked.action.or(picked.delete.map(ContourAction::Delete));
                }
                if let Some(on) = anti_alias {
                    let mut value = on;
                    let edit = Checkbox::bool(labels.anti_aliased.as_str(), &mut value).id_salt((id, "anti-alias")).enabled(enabled).show(ui);
                    if edit.event.is_some() {
                        out_anti_alias = Some(value);
                    }
                }
                thumb
            })
            .inner;

        let editor_key = id.with("editor");
        let ctx = ui.ctx().clone();
        let mut editor = ctx.data(|d| d.get_temp::<Option<Editor>>(editor_key)).flatten();
        if thumb.clicked() && enabled && editor.is_none() {
            editor = Some(Editor::open(contour));
        }
        if let Some(mut open) = editor.take() {
            let numbers = LocaleNumbers::of_ctx(&ctx);
            let shown = editor_dialog(&ctx, id, &style, &labels, &numbers, &mut open);
            if shown.edit.is_some() {
                out_edit = shown.edit;
            }
            if shown.action.is_some() {
                out_action = shown.action;
            }
            if shown.closed {
                ctx.data_mut(|d| d.insert_temp(editor_key, None::<Editor>));
            } else {
                ctx.data_mut(|d| d.insert_temp(editor_key, Some(open)));
            }
        }
        ContourFieldResponse { response: thumb, edit: out_edit, anti_alias: out_anti_alias, action: out_action }
    }

    /// `contour` as the curve the editor edits: points scaled to levels 0..=255, corner flags
    /// kept.
    pub fn to_curve(contour: &Contour) -> Curve {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:29-37 `levels_of`.
        let points = contour.points.iter().map(|p| [p.input * 255.0, p.output * 255.0]).collect();
        Curve::from_parts(points, contour.points.iter().map(|p| p.corner).collect())
    }

    /// The editor's `curve` back as a contour named `name`: points scaled to 0..=1 and validated
    /// by [`Contour::from_points`]; `None` for a drawn (`Curve::Map`) or invalid curve.
    pub fn from_curve(curve: &Curve, name: &str) -> Option<Contour> {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:518-525 `shown_contour`.
        let points = curve.points()?;
        let points =
            points.iter().enumerate().map(|(i, p)| ContourPoint { input: p[0] / 255.0, output: p[1] / 255.0, corner: curve.is_corner(i) }).collect();
        Contour::from_points(name, points)
    }
}

/// The open Contour Editor, kept in egui memory under the field's id.
#[derive(Clone)]
struct Editor {
    dialog: DialogState<Contour>,
    /// The edited contour.
    fields: Contour,
    /// The contour at `Begin`: what OK compares with.
    original: Contour,
    /// The last value reported to the host.
    reported: Contour,
    gesture: Option<GestureId>,
    /// The name being typed for New…, while that row shows.
    naming: Option<String>,
    /// The name field takes the focus on the pass it first shows.
    name_focus: bool,
}

impl Editor {
    fn open(contour: &Contour) -> Self {
        Editor {
            dialog: DialogState::new(),
            fields: contour.clone(),
            original: contour.clone(),
            reported: contour.clone(),
            gesture: None,
            naming: None,
            name_focus: false,
        }
    }
}

/// Everything the picker's content reads.
struct PickerInput<'a> {
    id: Id,
    style: &'a ContourFieldStyle,
    labels: &'a ContourFieldLabels,
    current: &'a Contour,
    presets: &'a [Contour],
    user: &'a [Contour],
    accent: Color32,
    enabled: bool,
}

/// What the picker asked this pass.
#[derive(Default)]
struct Picked {
    contour: Option<Contour>,
    delete: Option<usize>,
    action: Option<ContourAction>,
}

/// The curve of `contour` in a square of `side`, read-only and without an accessible node of its
/// own: the control that carries it names it.
fn curve_square(ui: &mut Ui, contour: &Contour, side: f32) -> Rect {
    let curve = ContourField::to_curve(contour);
    let shown = CurvePreview::new(&curve, Vec2::splat(side), "").show(ui);
    ui.ctx().accesskit_node_builder(shown.id, |node| node.set_hidden());
    shown.rect
}

/// The thumbnail: a button named "Contour Editor" valued by the contour's name.
fn thumbnail(
    ui: &mut Ui,
    id: Id,
    style: &ContourFieldStyle,
    labels: &ContourFieldLabels,
    contour: &Contour,
    accent: Color32,
    enabled: bool,
) -> Response {
    let rect = curve_square(ui, contour, style.thumb);
    let thumb = Interactive::with_id(id.with("thumb"), Role::Button, labels.editor.as_str())
        .value(contour.name.as_str())
        .enabled(enabled)
        .state_layer(false)
        .interact_at(ui, rect)
        .finish(ui);
    if thumb.hovered() {
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(craft_theme::Stroke::W1.width(1.0), accent), egui::StrokeKind::Outside);
    }
    thumb.on_hover_text(labels.editor.as_str())
}

/// The picker's arrow: a button named "Contour presets" that opens the popover.
fn arrow_button(ui: &mut Ui, id: Id, style: &ContourFieldStyle, labels: &ContourFieldLabels, tokens: &UiTokens, enabled: bool) -> Response {
    let arrow = Interactive::with_id(id.with("arrow"), Role::Button, labels.presets.as_str())
        .enabled(enabled)
        .state_layer(false)
        .show(ui, vec2(style.arrow, style.thumb));
    // A chevron a quarter of the arrow's width across.
    let half = style.arrow / 4.0;
    let centre = arrow.rect.center();
    let colour = if arrow.hovered() { tokens.color.text.normal.to_egui() } else { tokens.color.text.muted.to_egui() };
    let stroke = Stroke::new(craft_theme::Stroke::W1.width(1.0), colour);
    ui.painter().line_segment([centre + vec2(-half, -half / 2.0), centre + vec2(0.0, half / 2.0)], stroke);
    ui.painter().line_segment([centre + vec2(0.0, half / 2.0), centre + vec2(half, -half / 2.0)], stroke);
    arrow
}

/// The popover's content: the built-in presets, then the host's contours, then the library menu.
fn picker(ui: &mut Ui, pop: &PopoverCtx, input: &PickerInput<'_>) -> Picked {
    let mut picked = Picked::default();
    let all: Vec<&Contour> = input.presets.iter().chain(input.user).collect();
    let ids: Vec<Id> = (0..all.len()).map(|i| input.id.with(("cell", i))).collect();
    let rows = |n: usize| (0..n.div_ceil(COLUMNS)).map(move |r| COLUMNS.min(n - r * COLUMNS));
    let nav = GridNav::ragged(rows(input.presets.len()).chain(rows(input.user.len()))).rtl(direction::is_rtl(ui));
    let current = all.iter().position(|c| *c == input.current);
    let (stop, _) = GridNav::roving_move_in(ui, &ids, &nav, current, input.enabled);
    pop.initial_focus(ids.get(stop).copied().unwrap_or(input.id));
    let mut at = 0;
    for (group, contours) in [(0, input.presets), (1, input.user)] {
        if group == 1 && !contours.is_empty() {
            ui.add_space(ui.spacing().item_spacing.y);
        }
        for row in contours.chunks(COLUMNS) {
            ui.horizontal(|ui| {
                for contour in row {
                    let rect = curve_square(ui, contour, input.style.cell);
                    let mut cell = Interactive::with_id(ids[at], Role::Button, contour.name.as_str())
                        .enabled(input.enabled)
                        .state_layer(false)
                        .selected(current == Some(at));
                    if at != stop {
                        cell = cell.sense(Sense::CLICK);
                    }
                    let response = cell.interact_at(ui, rect).finish(ui);
                    if current == Some(at) || response.hovered() {
                        let stroke = Stroke::new(craft_theme::Stroke::W1_5.width(1.0), input.accent);
                        ui.painter().rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
                    }
                    if group == 1 {
                        let entries = [MenuEntry::Item(MenuItem::new(input.labels.delete_contour.as_str(), ()))];
                        if ContextMenu::new(&entries).show(ui, &response).chosen.is_some() {
                            picked.delete = Some(at - input.presets.len());
                        }
                    }
                    if response.clicked() && input.enabled {
                        picked.contour = Some((*contour).clone());
                        pop.close();
                    }
                    response.on_hover_text(contour.name.as_str());
                    at += 1;
                }
            });
        }
    }
    Divider::horizontal().show(ui);
    let labels = input.labels;
    let library = [
        (&labels.load_contours, ContourAction::Load, true),
        (&labels.replace_contours, ContourAction::Replace, true),
        (&labels.save_contours, ContourAction::Save, !input.user.is_empty()),
    ];
    for (label, action, available) in library {
        let button =
            Button::new(label.as_str()).variant(ButtonVariant::Ghost).enabled(input.enabled && available).id_salt((input.id, label.as_str()));
        if ui.add(button).clicked() {
            picked.action = Some(action);
            pop.close();
        }
    }
    picked
}

/// What the Contour Editor reported this pass.
struct EditorShown {
    edit: Option<EditEvent<Contour>>,
    action: Option<ContourAction>,
    /// The dialog was answered and closes.
    closed: bool,
}

/// The Contour Editor: a modal over the dialog with the preset select, the graph, the Input and
/// Output fields and the Corner check, and OK, Cancel, Load…, Save… and New… in a side column.
fn editor_dialog(
    ctx: &Context,
    id: Id,
    style: &ContourFieldStyle,
    labels: &ContourFieldLabels,
    numbers: &LocaleNumbers,
    editor: &mut Editor,
) -> EditorShown {
    let Editor { dialog, fields, original, reported, gesture, naming, name_focus } = editor;
    let colon = [':', '\u{ff1a}'];
    let mut name_done = false;
    let focus_name = std::mem::take(name_focus);
    let footer = Footer::side()
        .ok(labels.ok.as_str())
        .cancel(labels.cancel.as_str())
        .extra(labels.load.as_str())
        .extra(labels.save.as_str())
        .extra(labels.new.as_str());
    let shown = Dialog::new(id.with("editor"), labels.editor.as_str()).outer_gesture(true).history(true).footer(footer).show(
        ctx,
        dialog,
        fields,
        |ui, fields| {
            Row::new(Gap::Inline).show(ui, |ui| {
                ui.add(Text::new(labels.preset.as_str()).role(TypeRole::Label).content(Content::Muted));
                let mut name = fields.name.clone();
                let mut select = Select::new((id, "preset"), labels.preset.trim_end_matches(colon), &mut name).placeholder(fields.name.as_str());
                for preset in Contour::presets() {
                    select = select.option(preset.name.clone(), preset.name);
                }
                let picked = select.show(ui);
                if let Some(preset) = picked.event.as_ref().and_then(EditEvent::value).and_then(|name| Contour::preset(name)) {
                    *fields = preset;
                }
            });
            let curve = ContourField::to_curve(fields);
            let graph_name = if fields.name.is_empty() { labels.editor.clone() } else { fields.name.clone() };
            let between = |ui: &mut Ui| {
                let Some(name) = naming.as_mut() else { return };
                Row::new(Gap::Inline).show(ui, |ui| {
                    ui.add(Text::new(labels.name.as_str()).role(TypeRole::Label).content(Content::Muted));
                    let field = TextField::new((id, "name"), labels.name.trim_end_matches(colon), name.as_str()).show(ui);
                    if focus_name {
                        field.response.request_focus();
                    }
                    if let Some(text) = field.event.as_ref().and_then(EditEvent::value) {
                        name.clone_from(text);
                    }
                    let ok = ui.add(Button::new(labels.ok.as_str()).variant(ButtonVariant::Primary).id_salt((id, "name-ok")));
                    name_done |= ok.clicked() || field.submitted;
                });
            };
            let edit = CurveEditor::new((id, "graph"), graph_name, &curve, numbers)
                .levels(255)
                .max_points(Contour::MAX_POINTS)
                .side(style.graph)
                .bars(false)
                .percent()
                .corners(true)
                .between(between)
                .show(ui);
            if let Some(curve) = edit.event.as_ref().and_then(EditEvent::value)
                && let Some(next) = ContourField::from_curve(curve, &fields.name)
            {
                *fields = next.edited();
            }
        },
    );

    let mut out = EditorShown { edit: None, action: None, closed: shown.closed };
    if shown.opened {
        *gesture = shown.outer.map(|outer| outer.id());
    }
    if name_done && let Some(typed) = naming.take() {
        let name = if typed.trim().is_empty() { fields.name.clone() } else { typed };
        let new = Contour { name, points: fields.points.clone() };
        fields.clone_from(&new);
        out.action = Some(ContourAction::New(new));
    }
    match shown.extra {
        Some(0) => out.action = Some(ContourAction::Load),
        Some(1) => out.action = Some(ContourAction::Save),
        Some(2) if naming.is_none() => {
            // The suggestion is the contour's own name; the user types the new one over it.
            *naming = Some(fields.name.clone());
            *name_focus = true;
        }
        _ => {}
    }
    let Some(gesture) = *gesture else { return out };
    out.edit = match shown.outcome {
        Some(Outcome::Ok) => Some(EditEvent::commit(gesture, fields.clone(), fields != original, Modifiers::NONE)),
        Some(Outcome::Cancel | Outcome::Close) => Some(EditEvent::cancel(gesture, CancelReason::Dismissed)),
        _ if shown.opened => Some(EditEvent::begin(gesture)),
        _ if fields != reported => Some(EditEvent::update(gesture, fields.clone(), Modifiers::NONE)),
        _ => None,
    };
    if let Some(EditEvent::Update { value, .. }) = &out.edit {
        reported.clone_from(value);
    }
    out
}
