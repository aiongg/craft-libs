//! The Layers panel: the selected layer's blend, opacity, lock and fill above a list of layers,
//! and the layer actions in the footer.

use craft_theme::ControlSize;
use craft_ui::behavior::{Axis, DropPosition, Reorder, RowInfo, Selection};
use craft_ui::composites::PanelFooter;
use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ThumbSize, ToggleColumn};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::Panel;
use egui::{Color32, Painter, Rect, Ui, pos2};

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
    let panel = Panel::new("layers").title("Layers").scroll(false).footer_label("Layer actions").footer(|ui| {
        if PanelFooter::button(ui, Icon::Trash, "Delete layer").clicked() {
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
        Some(Action::Delete) => {
            let ids = mock.doc.selected.clone();
            if mock.doc.delete(&ids) > 0 {
                mock.record("Delete layer");
            }
        }
        None => {}
    }
}

/// A percent field showing `value`; the edited value when it changed.
fn percent(ui: &mut Ui, id: &str, caption: &str, numbers: &LocaleNumbers, value: u8, enabled: bool) -> Option<u8> {
    let field = NumberField::new(id, caption, f64::from(value), numbers)
        .range(0.0..=100.0)
        .decimals(0..=0)
        .unit(Unit::new("{n}%"))
        .size(ControlSize::Sm)
        .enabled(enabled)
        .show(ui);
    field.event.as_ref().and_then(|e| e.value()).map(|v| v.round() as u8)
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
            edits.push(Edit::Blend(shown));
        }
        ui.label("Opacity:");
        if let Some(v) = percent(ui, "layers.opacity", "Opacity", &numbers, opacity, enabled) {
            edits.push(Edit::Opacity(v));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Lock:");
        let lock_icons = [(Icon::Grid, "Lock transparent pixels"), (Icon::Brush, "Lock image pixels"), (Icon::Move, "Lock position")];
        for ((icon, label), on) in lock_icons.into_iter().zip(&mut state.partial_locks) {
            if ui.add(IconButton::new(icon, label).selected(*on).size(ControlSize::Sm).enabled(enabled)).clicked() {
                *on = !*on;
            }
        }
        let all = ui.add(IconButton::new(Icon::Lock, "Lock all").selected(locked).size(ControlSize::Sm).enabled(enabled));
        if all.clicked() {
            edits.push(Edit::Locked(!locked));
        }
        ui.label("Fill:");
        if let Some(v) = percent(ui, "layers.fill", "Fill", &numbers, fill, enabled) {
            edits.push(Edit::Fill(v));
        }
    });
    let Some(layer) = id.and_then(|id| mock.doc.layer_mut(id)) else { return };
    let changed = !edits.is_empty();
    for edit in edits {
        match edit {
            Edit::Blend(b) => layer.blend = b,
            Edit::Opacity(v) => layer.opacity = v,
            Edit::Fill(v) => layer.fill = v,
            Edit::Locked(on) => layer.locked = on,
        }
    }
    mock.doc.modified |= changed;
}

enum Edit {
    Blend(Blend),
    Opacity(u8),
    Fill(u8),
    Locked(bool),
}

/// The layer's thumbnail: its colour as the document shows it, or a checkerboard when it paints
/// nothing.
fn thumb(painter: &Painter, rect: Rect, shown: Option<[u8; 3]>) {
    match shown {
        Some([r, g, b]) => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(r, g, b));
        }
        None => {
            let (light, dark) = (Color32::from_gray(200), Color32::from_gray(150));
            painter.rect_filled(rect, 0.0, light);
            let half = pos2(rect.center().x, rect.center().y);
            painter.rect_filled(Rect::from_min_max(rect.min, half), 0.0, dark);
            painter.rect_filled(Rect::from_min_max(half, rect.max), 0.0, dark);
        }
    }
}

fn list(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    let State { selection, kind, .. } = state;
    let doc = &mock.doc;
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
            ListRow::new(id, layer.name.clone())
                .toggle(ToggleColumn::eye("Visibility {row}", layer.visible))
                .thumb(ThumbSize::Medium, move |painter, rect| thumb(painter, rect, shown))
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
