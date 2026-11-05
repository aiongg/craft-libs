//! The style chip and picker in a harness.

use craft_ui::theme::{Base, install};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;

struct App {
    styles: Vec<NamedStyleRow>,
    applied: Option<Option<u64>>,
    view: StylePickerView,
    last: Option<(Option<u64>, bool, bool, Option<u64>)>,
    picked: Vec<u64>,
}

fn style(id: u64, name: &str) -> NamedStyleRow {
    NamedStyleRow { id, name: name.to_owned(), meta: format!("meta {id}"), preview: StylePreview::Color(egui::Color32::RED) }
}

fn harness(applied: Option<Option<u64>>) -> Harness<'static, App> {
    let app = App {
        styles: vec![style(3, "Brand/Red"), style(1, "Brand/Blue/500"), style(2, "Body"), style(4, "Lead")],
        applied,
        view: StylePickerView::List,
        last: None,
        picked: Vec::new(),
    };
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(320.0, 480.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let r = StyleChip::new("chip", &app.styles, app.applied, &mut app.view).show(ui);
                app.picked.extend(r.picked);
                app.last = Some((r.picked, r.detach, r.create, r.edit));
                if r.detach {
                    app.applied = None;
                }
            },
            app,
        )
}

// The chip reads the applied style's leaf name, "Mixed" for Some(None); the detach button reports detach.
#[test]
fn chip_shows_applied_mixed_or_nothing_and_detaches() {
    let labels = StylePickerLabels::english();
    let mut harness = harness(Some(Some(1)));
    harness.run();
    harness.get_by_label("500");
    assert!(harness.query_by_label("Brand/Blue/500").is_none());

    harness.get_by_label(&labels.detach).click();
    harness.run();
    assert_eq!(harness.state().applied, None);
    assert!(harness.query_by_label(&labels.detach).is_none());
    harness.get_by_label(&labels.apply_style);

    harness.state_mut().applied = Some(None);
    harness.run();
    harness.get_by_label(&labels.mixed);
}

// The popover lists top-level styles first, then "Brand" and "Brand/Blue" captions; Enter on the second row reports picked with its id.
#[test]
fn picker_groups_by_folder_and_applies() {
    let mut harness = harness(Some(Some(1)));
    harness.run();
    harness.get_by_label("500").click();
    harness.run();

    let top = |label: &str| harness.get_by_label(label).rect().top();
    let order = [top("Body, meta 2"), top("Lead, meta 4"), top("Brand"), top("Red, meta 3"), top("Brand/Blue"), top("500, meta 1")];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");

    harness.get_by_label(&StylePickerLabels::english().search).focus();
    harness.run();
    for key in [egui::Key::ArrowDown, egui::Key::Home, egui::Key::ArrowDown, egui::Key::Enter] {
        harness.key_press(key);
        harness.run();
    }
    assert_eq!(harness.state().picked, [4]);
}

// The grid view moves over the tiles with the arrows from the search field and Enter picks the
// active tile; the tiles are in list order, top-level styles first.
#[test]
fn grid_view_picks_with_the_keyboard() {
    let labels = StylePickerLabels::english();
    let mut harness = harness(Some(Some(1)));
    harness.state_mut().view = StylePickerView::Grid;
    harness.run();
    harness.get_by_label("500").click();
    harness.run();
    harness.get_by_label(&labels.search).focus();
    harness.run();
    for key in [egui::Key::ArrowDown, egui::Key::ArrowRight, egui::Key::Enter] {
        harness.key_press(key);
        harness.run();
    }
    assert_eq!(harness.state().picked, [4]);
}

/// Every shape of the last pass, layers and clips flattened.
fn shapes(harness: &Harness<'_, App>) -> Vec<egui::Shape> {
    fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(inner) => inner.iter().for_each(|shape| flatten(shape, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        flatten(&clipped.shape, &mut out);
    }
    out
}

// The grid draws a swatch over the whole tile (FigCraft's SWATCH is its cell), and a folder's
// caption in the Caption role.
#[test]
fn grid_tiles_are_filled_and_folders_use_the_caption_role() {
    let mut harness = harness(Some(Some(1)));
    harness.state_mut().view = StylePickerView::Grid;
    harness.run();
    harness.get_by_label("500").click();
    harness.run();

    let tokens = craft_ui::theme::current(&harness.ctx);
    let side = StyleChip::style(&tokens).tile_side;
    let swatches: Vec<f32> = shapes(&harness)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::RED => Some(rect.rect.width()),
            _ => None,
        })
        .collect();
    // Four styles in four tiles; the chip's own preview square is another size.
    assert_eq!(swatches.iter().filter(|width| (**width - side).abs() < 0.01).count(), 4, "{swatches:?} against {side}");

    let caption = tokens.families().font_id(tokens.typography.get(craft_theme::TypeRole::Caption)).size;
    let sizes: Vec<f32> = shapes(&harness)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text() == "Brand" => text.galley.job.sections.first().map(|s| s.format.font_id.size),
            _ => None,
        })
        .collect();
    assert_eq!(sizes, [caption]);
}

// `opened(true)` shows the picker without a click; the next pass without it keeps it open.
#[test]
fn opened_shows_the_picker_on_that_pass() {
    let labels = StylePickerLabels::english();
    let mut harness = HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(320.0, 480.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (view, first): &mut (StylePickerView, bool)| {
                let styles = [style(1, "Body")];
                StyleChip::new("chip", &styles, Some(Some(1)), view).opened(*first).show(ui);
                *first = false;
            },
            (StylePickerView::List, true),
        );
    harness.run();
    harness.get_by_label(&labels.search);
}
