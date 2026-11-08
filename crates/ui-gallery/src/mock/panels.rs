//! The panels of the dock: the canvas group the host draws, the Navigator and the layer list.

use super::model::{Document, Panel};
use craft_behavior::dock::Permissions;
use craft_behavior::kurbo::Rect as DocRect;
use craft_ui::buttons::Button;
use craft_ui::canvas::{Canvas, PageFrame, Pasteboard, RulerText, Rulers, ToolPath};
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, BodyCx, Props as _, Show as _, ShowContainer as _};
use craft_ui::docking::{DockPanels, PanelLimits, PanelSpec};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::patterns::canvas::NavigatorPanel;
use craft_ui::runtime::env::UnitKind;
use craft_ui::shell::CanvasPanel;
use craft_ui::tabs::{DocumentTab, DocumentTabs};
use craft_behavior::RulerUnits;
use egui::{Rect, Ui};

/// The dock's panels over the open document.
pub(super) struct Panels<'a> {
    pub(super) document: &'a mut Document,
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
        CanvasPanel::new()
            .tabs(move |ui, _| {
                let tabs = [DocumentTab::new(0_usize, title.clone()).modified(modified)];
                let _ = DocumentTabs::new("Documents", &tabs).selected(0).show(ui);
            })
            .show(ui, |ui, _| canvas(ui, document));
        true
    }
}

/// The document area: the pasteboard and the rulers on a canvas whose view the navigator and the view commands edit.
fn canvas(ui: &mut Ui, document: &mut Document) {
    let full = ui.available_rect_before_wrap();
    document.view.rect = document.content.unwrap_or(full);
    let units = RulerText::new(RulerUnits::pixels(), UnitKind::Pixels);
    let page = DocRect::new(0.0, 0.0, document.size.width, document.size.height);
    let pages = [PageFrame::new(page)];
    let (view, zero) = (document.view, document.zero);
    let mut moved_zero = zero;
    let shown = Canvas::new("Document", ToolPath::Freehand).view(view).document(document.size).rulers(units, units, zero).show(ui, |ui, body| {
        let _ = Pasteboard::new("Pasteboard", &pages).show_at(ui, body.rect);
        Rulers::new("Rulers", Bind::bound(&mut moved_zero)).show_at(ui, body.rect)
    });
    if let Some(rulers) = shown.inner {
        document.content = Some(rulers.content_rect());
    }
    document.zero = moved_zero;
    if let Some(next) = shown.output.view {
        document.view = next;
    }
}

/// The layer list, top layer first: a visibility check box and a button that makes the layer current.
fn layers(ui: &mut Ui, document: &mut Document) {
    let mut set_visible = None;
    let mut pick = None;
    Stack::new().show(ui, |ui, _| {
        for (at, layer) in document.layers.iter().enumerate().rev() {
            Row::new().show(ui, |ui, _| {
                let mut visible = layer.visible;
                Checkbox::new(Bind::bound(&mut visible)).label(format!("{} visible", layer.name)).show(ui);
                if visible != layer.visible {
                    set_visible = Some((at, visible));
                }
                if Button::new(layer.name.clone()).show(ui).clicked() {
                    pick = Some(at);
                }
            });
        }
    });
    if let Some((at, visible)) = set_visible {
        document.set_visible(at, visible);
    }
    if let Some(at) = pick {
        document.current = at;
    }
}
