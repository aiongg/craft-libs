//! The panels of the dock: the canvas group the host draws, the Navigator and the layer table.

use super::model::{Document, Panel, Tool};
use super::tools::{Freehand, Tools};
use craft_behavior::dock::Permissions;
use craft_behavior::kurbo::Rect as DocRect;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, ItemSource, RulerUnits, Selection, SelectionModel as _};
use craft_ui::canvas::{Canvas, PageFrame, Pasteboard, RulerText, Rulers, ToolHost};
use craft_ui::contract::{
    Bind, BodyCx, CollectionEvent, ColumnKind, ColumnSpec, SelectMode, Show as _, ShowCollection as _, ShowContainer as _, ToggleColumn,
};
use craft_ui::docking::{DockPanels, PanelLimits, PanelSpec};
use craft_ui::list::{ListRow, ToggleCell};
use craft_ui::patterns::canvas::NavigatorPanel;
use craft_ui::runtime::env::UnitKind;
use craft_ui::shell::CanvasPanel;
use craft_ui::table::Table;
use craft_ui::tabs::{DocumentTab, DocumentTabs};
use egui::{Rect, Ui};
use std::borrow::Cow;

const NAME: ColumnKey = ColumnKey::new("name");
const VISIBLE: ColumnKey = ColumnKey::new("visible");

/// The dock's panels over the open document.
pub(super) struct Panels<'a> {
    pub(super) document: &'a mut Document,
    pub(super) tool: Tool,
    pub(super) tools: &'a mut Tools,
}

/// What the dock may do with `panel`: the canvas stays where it is.
pub(super) fn permissions(panel: &Panel) -> Permissions {
    if *panel == Panel::Canvas { Permissions::PROTECTED } else { Permissions::ALL }
}

fn name(panel: Panel) -> &'static str {
    match panel {
        Panel::Canvas => "Canvas",
        Panel::Navigator => "Navigator",
        Panel::Layers => "Layers",
    }
}

impl DockPanels<Panel> for Panels<'_> {
    fn spec(&self, panel: &Panel) -> PanelSpec {
        let spec = PanelSpec::new(name(*panel)).limits(PanelLimits::default());
        if *panel == Panel::Canvas { spec.canvas() } else { spec }
    }

    fn permissions(&self, panel: &Panel) -> Permissions {
        permissions(panel)
    }

    fn body(&mut self, panel: &Panel, ui: &mut Ui, _: &mut BodyCx<'_>) {
        match panel {
            Panel::Navigator => {
                let size = self.document.size;
                let _ = NavigatorPanel::new("Navigator", Bind::bound(&mut self.document.view), size).show(ui);
            }
            Panel::Layers => layers(ui, self.document),
            Panel::Canvas => {}
        }
    }

    fn group_view(&mut self, panels: &[Panel], _: usize, ui: &mut Ui, _: Rect, _: &mut BodyCx<'_>) -> bool {
        if !panels.contains(&Panel::Canvas) {
            return false;
        }
        let title = self.document.title.clone();
        let modified = self.document.dirty;
        let document = &mut *self.document;
        let (tool, tools) = (self.tool, &mut *self.tools);
        CanvasPanel::new()
            .tabs(move |ui, _| {
                let tabs = [DocumentTab::new(0_usize, title.clone()).modified(modified)];
                let _ = DocumentTabs::new("Documents", &tabs).selected(0).show(ui);
            })
            .show(ui, |ui, _| canvas(ui, document, tool, tools));
        true
    }
}

/// The document area: the pasteboard and the rulers on a canvas whose tool host runs the current tool and applies
/// the view it reports.
fn canvas(ui: &mut Ui, document: &mut Document, tool: Tool, tools: &mut Tools) {
    let full = ui.available_rect_before_wrap();
    document.view.rect = document.content.unwrap_or(full);
    let units = RulerText::new(RulerUnits::pixels(), UnitKind::Pixels);
    let page = DocRect::new(0.0, 0.0, document.size.width, document.size.height);
    let pages = [PageFrame::new(page)];
    let (view, zero) = (document.view, document.zero);
    let mut moved_zero = zero;
    let shown = Canvas::new("Document").view(view).document(document.size).rulers(units, units, zero).show(ui, |ui, body| {
        let _ = Pasteboard::new("Pasteboard", &pages).show_at(ui, body.rect);
        Rulers::new("Rulers", Bind::bound(&mut moved_zero)).show_at(ui, body.rect)
    });
    if let Some(rulers) = shown.inner {
        document.content = Some(rulers.content_rect());
    }
    document.zero = moved_zero;
    let host = ToolHost::new().momentary(&mut tools.momentary);
    let reported = match tool {
        Tool::Hand => host.run(ui, &shown.output, &mut tools.hand),
        Tool::Zoom => host.run(ui, &shown.output, &mut tools.zoom),
        _ => host.run(ui, &shown.output, &mut Freehand),
    };
    if let Some(next) = reported.view {
        document.view = next;
    }
}

/// The layers as a collection source, keyed by position, bottom layer first.
struct LayerRows<'a>(&'a Document);

impl DisplayOrder<usize> for LayerRows<'_> {
    fn len(&self) -> usize {
        self.0.layers.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.layers.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.0.layers.get(at).map(|layer| Cow::Borrowed(layer.name.as_str()))
    }
}

impl ItemSource<usize> for LayerRows<'_> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        *column == VISIBLE && self.0.layers.get(at).is_some_and(|layer| layer.visible)
    }
}

/// The layer table: a name column and a visibility toggle column over a single selection that is the current layer.
fn layers(ui: &mut Ui, document: &mut Document) {
    let columns = vec![
        ColumnSpec::new(NAME, "Layer", ColumnWidth::fill(1.0)),
        ColumnSpec::new(VISIBLE, "Visible", ColumnWidth::Controls(1.5)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
    ];
    let mut selection = Selection::single(document.current);
    let events = {
        let source = LayerRows(document);
        let layers = &document.layers;
        Table::<usize>::new("Layers", columns)
            .select(SelectMode::Single)
            .selection(Bind::bound(&mut selection))
            .show_items(ui, &source, &mut |key, icx| {
                ListRow::new(layers[*key].name.clone()).cells(icx.cells()).toggle(ToggleCell::eye(VISIBLE, layers[*key].visible, "Visibility {row}"))
            })
            .events
    };
    for event in events {
        let CollectionEvent::Toggle(edit) = event else { continue };
        if let Some(toggle) = edit.value() {
            document.set_visible(toggle.key, toggle.on);
        }
    }
    if let Some(current) = selection.lead() {
        document.current = *current;
    }
}
