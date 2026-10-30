//! The Layers panel: the selected layer's blend, opacity, lock and fill above a list of layers,
//! and the layer actions in the footer.

use craft_theme::ControlSize;
use craft_theme::TypeRole;
use craft_ui::behavior::{Axis, DropPosition, Reorder, RowInfo, Selection};
use craft_ui::composites::PanelFooter;
use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::controls::{Swatch, checkerboard};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ThumbSize, ToggleColumn};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::Panel;
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::text::LayoutJob;
use egui::{Color32, CornerRadius, Label, Painter, Rect, Response, Ui, vec2};

use super::super::model::{Blend, LayerId, LayerKind, Mock};

/// Which layers the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    All,
    Pixel,
    Adjustment,
}

impl Kind {
    const ALL: [Kind; 3] = [Kind::All, Kind::Pixel, Kind::Adjustment];

    fn label(self) -> &'static str {
        match self {
            Kind::All => "Kind",
            Kind::Pixel => "Pixel",
            Kind::Adjustment => "Adjustment",
        }
    }

    fn shows(self, kind: &LayerKind) -> bool {
        match self {
            Kind::All => true,
            Kind::Pixel => !matches!(kind, LayerKind::Adjustment { .. }),
            Kind::Adjustment => matches!(kind, LayerKind::Adjustment { .. }),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    selection: Selection<LayerId>,
    kind: Kind,
    /// Lock transparent pixels, image pixels and position: the mock layers have no such locks, so
    /// the buttons only keep their state.
    partial_locks: [bool; 3],
}

/// What the footer asked for.
enum Action {
    NewLayer,
    Adjustment,
    Delete,
}

pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    if state.selection.items != mock.doc.selected {
        let lead = mock.doc.selected.first().copied();
        state.selection = Selection { items: mock.doc.selected.clone(), lead, anchor: lead };
    }
    let mut action = None;
    let deletable = mock.doc.selected.iter().any(|id| mock.doc.layer(*id).is_some_and(|l| !l.locked));
    let panel = Panel::new("layers").title("Layers").scroll(false).footer_label("Layer actions").footer(|ui| {
        if ui.add_enabled_ui(deletable, |ui| PanelFooter::button(ui, Icon::Trash, "Delete layer")).inner.clicked() {
            action = Some(Action::Delete);
        }
        if PanelFooter::button(ui, Icon::NewLayer, "New layer").clicked() {
            action = Some(Action::NewLayer);
        }
        ui.add_enabled_ui(false, |ui| PanelFooter::button(ui, Icon::NewFolder, "New group"));
        if PanelFooter::button(ui, Icon::AdjustmentLayer, "New adjustment layer").clicked() {
            action = Some(Action::Adjustment);
        }
        ui.add_enabled_ui(false, |ui| PanelFooter::button(ui, Icon::Link, "Link layers"));
    });
    panel.show(ui, |ui| {
        header(ui, mock, state);
        list(ui, mock, state);
    });
    match action {
        Some(Action::NewLayer) => mock.add_layer(),
        Some(Action::Adjustment) => mock.add_adjustment("Brightness/Contrast"),
        Some(Action::Delete) => mock.delete_selected(),
        None => {}
    }
}

/// A caption in the options bar's style, drawn before the control it names.
fn caption(ui: &mut Ui, text: &str) -> Response {
    let tokens = Tokens::of(ui);
    let mut format = Text::format(&tokens, TypeRole::Body, Content::Muted);
    format.valign = ui.text_valign();
    ui.add(Label::new(LayoutJob::single_section(text.to_owned(), format)).selectable(false))
}

/// A percent field named `caption`, at the end edge of the row, scrubbed by its caption. Returns
/// the value it took and whether that edit is committed (a typed entry or the end of a scrub).
fn percent(ui: &mut Ui, id: &str, text: &str, numbers: &LocaleNumbers, value: u8, enabled: bool) -> Option<(u8, bool)> {
    let tokens = Tokens::of(ui);
    let style = OptionsBar::style(&tokens, false);
    let width = {
        let font = tokens.font(TypeRole::Body);
        let label = ui.fonts_mut(|fonts| fonts.layout_no_wrap(format!("{text}:"), font, Color32::PLACEHOLDER).size().x);
        label + style.gap + style.fields[1]
    };
    drop(tokens);
    let mut edit = None;
    ui.with_layout(craft_ui::direction::trailing(ui), |ui| {
        ui.allocate_ui_with_layout(vec2(width, ui.available_height()), craft_ui::direction::row(ui), |ui| {
            OptionsBar::cell(ui, &format!("{text}:"), CellWidth::Md, |ui, label| {
                let field = NumberField::new(id, text, f64::from(value), numbers)
                    .range(0.0..=100.0)
                    .decimals(0..=0)
                    .unit(Unit::new("{n}%"))
                    .size(ControlSize::Sm)
                    .enabled(enabled)
                    .scrub_by(label)
                    .show(ui);
                edit = match field.event {
                    Some(EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. }) => Some((value.round() as u8, true)),
                    Some(EditEvent::Update { value, .. }) => Some((value.round() as u8, false)),
                    _ => None,
                };
            });
        });
    });
    edit
}

fn header(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    let numbers = LocaleNumbers::of(ui);
    let id = mock.doc.selected_layer().map(|l| l.id);
    let (blend, opacity, fill, locked) =
        mock.doc.selected_layer().map_or((Blend::Normal, 100, 100, false), |l| (l.blend, l.opacity, l.fill, l.locked));
    let enabled = id.is_some();
    let mut edits = Vec::new();
    ui.horizontal(|ui| {
        let mut select = Select::new("layers.kind", "Kind", &mut state.kind).size(ControlSize::Sm).fit(true);
        for kind in Kind::ALL {
            select = select.option(kind, kind.label());
        }
        let _ = select.show(ui);
    });
    ui.horizontal(|ui| {
        let mut shown = blend;
        let mut select = Select::new("layers.blend", "Blend mode", &mut shown).size(ControlSize::Sm).fit(true).enabled(enabled);
        for b in Blend::ALL {
            select = select.option(b, b.label());
        }
        if select.show(ui).response.changed() {
            edits.push(Edit { change: Change::Blend(shown), commit: true });
        }
        if let Some((v, commit)) = percent(ui, "layers.opacity", "Opacity", &numbers, opacity, enabled) {
            edits.push(Edit { change: Change::Opacity(v), commit });
        }
    });
    ui.horizontal(|ui| {
        caption(ui, "Lock:");
        let lock_icons = [(Icon::Grid, "Lock transparent pixels"), (Icon::Brush, "Lock image pixels"), (Icon::Move, "Lock position")];
        for ((icon, label), on) in lock_icons.into_iter().zip(&mut state.partial_locks) {
            ui.add(IconButton::new(icon, label).toggle(on).size(ControlSize::Sm).enabled(enabled));
        }
        let mut all = locked;
        ui.add(IconButton::new(Icon::Lock, "Lock all").toggle(&mut all).size(ControlSize::Sm).enabled(enabled));
        if all != locked {
            edits.push(Edit { change: Change::Locked(all), commit: true });
        }
        if let Some((v, commit)) = percent(ui, "layers.fill", "Fill", &numbers, fill, enabled) {
            edits.push(Edit { change: Change::Fill(v), commit });
        }
    });
    mock.doc.modified |= !edits.is_empty();
    let Some(layer) = id.and_then(|id| mock.doc.layer_mut(id)) else { return };
    let mut history = Vec::new();
    for Edit { change, commit } in edits {
        let name = match change {
            Change::Blend(b) => {
                layer.blend = b;
                "Blending Change"
            }
            Change::Opacity(v) => {
                layer.opacity = v;
                "Opacity Change"
            }
            Change::Fill(v) => {
                layer.fill = v;
                "Fill Opacity Change"
            }
            Change::Locked(on) => {
                layer.locked = on;
                "Lock Layers"
            }
        };
        if commit {
            history.push(name);
        }
    }
    for name in history {
        mock.record(name);
    }
}

/// One change to the selected layer; `commit` is false while a drag is still going.
struct Edit {
    change: Change,
    commit: bool,
}

enum Change {
    Blend(Blend),
    Opacity(u8),
    Fill(u8),
    Locked(bool),
}

/// The checkerboard colours and cell side of a swatch.
struct Checker {
    colors: [Color32; 2],
    cell: f32,
}

/// The layer's thumbnail: its colour as the document shows it, or a checkerboard when it paints
/// nothing.
fn thumb(painter: &Painter, rect: Rect, shown: Option<[u8; 3]>, checker: &Checker) {
    match shown {
        Some([r, g, b]) => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(r, g, b));
        }
        None => {
            painter.extend(checkerboard(rect, CornerRadius::ZERO, checker.colors, checker.cell));
        }
    }
}

fn list(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    let State { selection, kind, .. } = state;
    let doc = &mock.doc;
    let swatch = Swatch::style(&Tokens::of(ui), ControlSize::Sm);
    let checker = std::sync::Arc::new(Checker { colors: swatch.checker, cell: swatch.checker_cell });
    let order: Vec<RowInfo<LayerId>> = doc.layers.iter().filter(|l| kind.shows(&l.kind)).map(|l| RowInfo::leaf(l.id)).collect();
    let unlocked: Vec<LayerId> = doc.layers.iter().filter(|l| !l.locked).map(|l| l.id).collect();
    let out = List::new("layers.list", "Layers", order.as_slice())
        .selection(selection)
        .renamable(true)
        .can_rename(|id| unlocked.contains(id))
        .thumbnails(ThumbSize::Medium)
        .reorder(Reorder::new(Axis::Vertical))
        .show(ui, |_, &id| {
            let at = doc.index_of(id).unwrap_or_default();
            let layer = &doc.layers[at];
            let shown = doc.shown_color(at);
            let checker = checker.clone();
            ListRow::new(id, layer.name.clone())
                .toggle(ToggleColumn::eye("Visibility {row}", layer.visible))
                .thumb(ThumbSize::Medium, move |painter, rect| thumb(painter, rect, shown, &checker))
                .dimmed(!layer.visible)
                .layer(true)
        });
    mock.doc.selected = selection.items.clone();
    for event in out.events {
        match event {
            ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. })) => {
                mock.set_layer_visible(value.row, value.on)
            }
            ListEvent::Row(RowEvent::Renamed { row, text }) => mock.rename_layer(row, &text),
            ListEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                mock.reorder_layers(&value.items, value.target, value.position == DropPosition::After);
            }
            _ => {}
        }
    }
}
