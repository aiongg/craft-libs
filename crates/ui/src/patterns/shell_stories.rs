//! Stories of the step 3 shell patterns: the tour, the status readout and the toolbar drawer.

use std::time::Duration;

use egui::{Rect, Ui, vec2};

use super::rail::tool::story_slots;
use super::{DrawerGroup, StatusBar, StatusItem, StatusReadout, StatusValues, ToolbarDrawer, Tour, TourEvent, TourStep};
use crate::story::{Level, NoArgs, Story, Surface};
use crate::typed_story;

/// The components these stories cover.
pub const COMPONENTS: &[&str] = &["Tour", "StatusReadout", "ToolbarDrawer"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/shell/tour", Level::Patterns, "Tour", "Tour", NoArgs {}, show_tour).on(Surface::Chrome),
        typed_story!("patterns/shell/status-readout", Level::Patterns, "StatusReadout", "Readout", NoArgs {}, show_readout).on(Surface::Chrome),
        typed_story!("patterns/shell/toolbar-drawer", Level::Patterns, "ToolbarDrawer", "Drawer", NoArgs {}, show_drawer).on(Surface::Chrome),
    ]
}

/// A three-step tour over a mock window: a step with a target, one without, and the last step.
fn show_tour(ui: &mut Ui, _args: &mut NoArgs) {
    let area = ui.allocate_space(vec2(ui.available_width().min(520.0), 280.0)).1;
    let target = Rect::from_min_size(area.min + vec2(24.0, 40.0), vec2(120.0, 160.0));
    ui.painter().rect_stroke(area, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);
    ui.painter().rect_stroke(target, 2.0, ui.visuals().widgets.noninteractive.fg_stroke, egui::StrokeKind::Inside);
    let steps = [
        TourStep { id: "layers-panel".into(), title: "Layers".into(), body: "Layers stack the parts of a document.".into(), target: Some(target) },
        TourStep { id: "welcome".into(), title: "Welcome".into(), body: "A step without a target is centred.".into(), target: None },
        TourStep {
            id: "done".into(),
            title: "That is all".into(),
            body: "The last step offers Done.".into(),
            target: Some(target.translate(vec2(160.0, 0.0))),
        },
    ];
    let key = ui.scope_id().with("tour-story");
    let mut index = ui.data(|d| d.get_temp::<Option<usize>>(key)).unwrap_or(Some(0));
    if let Some(step) = index {
        match Tour::new("tour-story").show(ui.ctx(), &steps, step) {
            Some(TourEvent::Next) => index = Some(step + 1),
            Some(TourEvent::Back) => index = Some(step.saturating_sub(1)),
            Some(TourEvent::Finished | TourEvent::Skipped) => index = None,
            _ => {}
        }
    } else if ui.add(crate::controls::Button::new("Restart tour")).clicked() {
        index = Some(0);
    }
    ui.data_mut(|d| d.insert_temp(key, index));
}

/// The readout in a status bar with every item to choose from.
fn show_readout(ui: &mut Ui, _args: &mut NoArgs) {
    let key = ui.scope_id().with("readout-story");
    let (mut zoom, mut item) = ui.data(|d| d.get_temp::<(f32, StatusItem)>(key)).unwrap_or((100.0, StatusItem::DocumentSizes));
    let values = StatusValues {
        document_sizes: Some((26_100_000, 74_500_000)),
        document_profile: Some("sRGB IEC61966-2.1".to_owned()),
        document_dimensions: Some("4000 px × 3000 px (300 ppi)".to_owned()),
        measurement_scale: Some("1 px = 1 px".to_owned()),
        scratch_sizes: Some((1_300_000_000, 8_000_000_000)),
        efficiency: Some(100),
        timing: Some(Duration::from_millis(400)),
        current_tool: Some("Brush".to_owned()),
        layer_count: Some(12),
    };
    StatusBar::new("readout-story").show(ui, |bar| {
        bar.leading(|ui| {
            let out = StatusReadout::new("readout").show(ui, zoom, item, &values);
            if let Some(next) = out.zoom {
                zoom = next;
            }
            if let Some(next) = out.item {
                item = next;
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(key, (zoom, item)));
}

/// A tool rail's column with the drawer open beside it.
fn show_drawer(ui: &mut Ui, _args: &mut NoArgs) {
    let area = ui.allocate_space(vec2(ui.available_width().min(520.0), 360.0)).1;
    let rail = Rect::from_min_size(area.min, vec2(40.0, area.height()));
    let slots = story_slots();
    let groups = [
        DrawerGroup { label: "Select".to_owned(), tools: slots[1].tools.iter().chain(&slots[2].tools).cloned().collect() },
        DrawerGroup { label: "Paint".to_owned(), tools: slots[5].tools.iter().chain(&slots[8].tools).cloned().collect() },
    ];
    let toolbar = &slots[..3];
    ToolbarDrawer::new("drawer-story").show(ui, rail, &groups, toolbar);
}
