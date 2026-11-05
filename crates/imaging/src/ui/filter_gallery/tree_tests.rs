//! Tests of Filter Gallery's columns and workspace.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Color32, ColorImage, Key, Modifiers, Vec2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::{Param, ParamKind};
use craft_ui::theme::{Base, install};

use super::{
    EffectChange, FilterCatalog, FilterCategory, FilterGalleryEvent, FilterGalleryOpen, FilterGalleryOutput, FilterGallerySession,
    FilterGalleryWorkspace, GalleryFilter, GalleryThumb,
};

const WINDOW: Vec2 = vec2(1100.0, 640.0);

struct Catalog;

impl FilterCatalog for Catalog {
    fn categories(&self) -> Vec<FilterCategory> {
        let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
        vec![
            FilterCategory { key: "artistic".to_owned(), label: "Artistic".to_owned(), filters: keys(&["cutout", "sponge", "neon"]) },
            FilterCategory { key: "sketch".to_owned(), label: "Sketch".to_owned(), filters: keys(&["chalk"]) },
        ]
    }

    fn filter(&self, key: &str) -> Option<GalleryFilter> {
        let category = if key == "chalk" { "sketch" } else { "artistic" };
        let range = ParamKind::Range { min: 0.0, max: 40.0, default: 6.0, scale: Default::default() };
        Some(GalleryFilter {
            key: key.to_owned(),
            label: key.to_uppercase(),
            category: category.to_owned(),
            params: vec![Param::new("size", "Size", range)],
            uses_colors: key == "sponge",
        })
    }
}

/// A flat picture the size of the layer.
struct Flat;

impl RasterSource for Flat {
    fn size(&self) -> [u32; 2] {
        [400, 300]
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_gray(120));
    }
}

struct App {
    workspace: FilterGalleryWorkspace,
    outputs: Vec<FilterGalleryOutput>,
    /// Answers every ticket at once, as a quick render does.
    ready: bool,
}

fn harness(ready: bool) -> Harness<'static, App> {
    let session = FilterGallerySession::new("cutout", &Catalog);
    let workspace = FilterGalleryWorkspace::new(FilterGalleryOpen::new([400, 300], "Layer 1", session));
    let mut harness = HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = ui.ctx().clone();
                let thumbs = Thumbnails::<GalleryThumb>::in_context(&ctx, "gallery-test-thumbs", ThumbnailOptions::default());
                let out = app.workspace.show(&ctx, &Catalog, &thumbs, &FakeThumbSource::<GalleryThumb>::new(), &Flat);
                if app.ready
                    && let Some(ticket) = out.preview_wanted
                {
                    app.workspace.preview_ready(ticket);
                }
                app.outputs.push(out);
            },
            App { workspace, outputs: Vec::new(), ready },
        );
    // A render that has not answered repaints for ever (its progress bar), so the harness steps.
    if ready {
        harness.run();
    } else {
        steps(&mut harness);
    }
    harness
}

fn steps(harness: &mut Harness<'_, App>) {
    for _ in 0..4 {
        harness.step();
    }
}

fn events(app: &App) -> Vec<&FilterGalleryEvent> {
    app.outputs.iter().flat_map(|out| out.events.iter()).collect()
}

// OK and Cancel sit at the top of the settings column, in the platform's order.
#[test]
fn gallery_footer_is_at_the_top() {
    let harness = harness(true);
    let ok = harness.get_by_label("OK").rect();
    let cancel = harness.get_by_label("Cancel").rect();
    let select = harness.get_by_role_and_label(Role::ComboBox, "Filter").rect();
    let stack = harness.get_by_role_and_label(Role::ListBox, "Effect Layers").rect();
    assert!(ok.bottom() <= select.top() && cancel.bottom() <= select.top(), "{ok:?} {cancel:?} over {select:?}");
    assert!(select.bottom() <= stack.top());
    assert!((ok.center().y - cancel.center().y).abs() < 1.0, "one row");
    // The column is at the end edge.
    assert!(ok.left().min(cancel.left()) > WINDOW.x - 300.0 && ok.right().max(cancel.right()) > WINDOW.x - 30.0, "{ok:?} {cancel:?}");
}

// ⌘+ zooms the preview, not the canvas behind; the zoom bar's select shows the new percent.
#[test]
fn gallery_zoom_shortcuts_zoom_the_preview() {
    let mut harness = harness(true);
    assert!(harness.state().workspace.view.is_fit());
    let fitted = harness.state().workspace.view.zoom();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Plus);
    harness.run();
    let zoomed = harness.state().workspace.view.zoom();
    assert!(!harness.state().workspace.view.is_fit() && zoomed > fitted, "{zoomed} after {fitted}");
    let shown = harness.get_by_role_and_label(Role::ComboBox, "Zoom").accesskit_node().value();
    let percent = format!("{:.0}%", zoomed * 100.0);
    assert_eq!(shown.as_deref(), Some(percent.as_str()));
    assert!(harness.state().outputs.last().is_some_and(|out| out.preview_wanted.is_some()) || harness.state().workspace.visible().1 == zoomed);

    harness.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    harness.run();
    assert!((harness.state().workspace.view.zoom() - 1.0).abs() < 1e-9);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    harness.run();
    assert!(harness.state().workspace.view.is_fit());
}

// Clicking a thumbnail picks its filter; arrows and Enter do the same from the keyboard.
#[test]
fn gallery_thumbnails_pick_by_pointer_and_keyboard() {
    let mut harness = harness(true);
    harness.get_by_role_and_label(Role::Button, "SPONGE").click();
    harness.run();
    assert_eq!(harness.state().workspace.session().effects[0].filter, "sponge");
    assert!(
        events(harness.state()).iter().any(|e| matches!(e, FilterGalleryEvent::Stack(EffectChange::SetFilter { filter, .. }) if filter == "sponge"))
    );

    harness.get_by_role_and_label(Role::Button, "SPONGE").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().workspace.session().effects[0].filter, "neon");
    // One undo step per pick.
    assert!(harness.state_mut().workspace.step(craft_ui::dialog::Step::Undo));
    assert_eq!(harness.state().workspace.session().effects[0].filter, "sponge");
}

// While a render runs the zoom bar shows its progress; a result for an older ticket is dropped.
#[test]
fn gallery_preview_shows_progress_and_drops_stale_results() {
    let mut harness = harness(false);
    let Some(first) = harness.state().outputs.iter().find_map(|out| out.preview_wanted) else { panic!("the first pass asks for a preview") };
    harness.state_mut().workspace.preview_progress(first, 0.5);
    steps(&mut harness);
    assert!(harness.query_by_role(Role::ProgressIndicator).is_some());

    // A change asks for a newer ticket; the older result is dropped and the render goes on.
    harness.get_by_role_and_label(Role::Button, "NEON").click();
    steps(&mut harness);
    let Some(second) = harness.state().outputs.iter().rev().find_map(|out| out.preview_wanted) else { panic!("the change asks again") };
    assert_ne!(first, second);
    harness.state_mut().workspace.preview_ready(first);
    harness.state_mut().workspace.preview_progress(second, 0.25);
    steps(&mut harness);
    assert!(harness.query_by_role(Role::ProgressIndicator).is_some(), "the stale result did not end the render");

    harness.state_mut().workspace.preview_ready(second);
    harness.state_mut().workspace.set_note(Some("Preview 12 ms".to_owned()));
    harness.run();
    assert!(harness.query_by_role(Role::ProgressIndicator).is_none());
    assert!(harness.query_by_label("Preview 12 ms").is_some());
}

// ⌘Z steps the stack while a number field has the keyboard, and the field keeps its text: the
// workspace reads the key before the body.
#[test]
fn gallery_undo_works_while_a_number_field_has_focus() {
    let mut harness = harness(true);
    harness.get_by_role_and_label(Role::Button, "SPONGE").click();
    harness.run();
    assert_eq!(harness.state().workspace.session().effects[0].filter, "sponge");

    harness.get_by_role_and_label(Role::SpinButton, "Size").click();
    harness.run();
    let before = harness.get_by_role_and_label(Role::SpinButton, "Size").accesskit_node().value();
    assert!(harness.ctx.text_edit_focused(), "the number field has the keyboard");

    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(harness.state().workspace.session().effects[0].filter, "cutout", "the pick is undone");
    assert_eq!(harness.get_by_role_and_label(Role::SpinButton, "Size").accesskit_node().value(), before, "the field keeps its text");

    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    harness.run();
    assert_eq!(harness.state().workspace.session().effects[0].filter, "sponge", "redo brings it back");
}

// The two columns have their own names, and the stack's footer buttons are Tab stops in reading
// order: Move up, Move down, New, Delete.
#[test]
fn gallery_columns_are_named_and_footer_buttons_tab_in_reading_order() {
    let mut harness = harness(true);
    assert!(harness.query_all_by_label("Settings").next().is_some() && harness.query_all_by_label("Filters").next().is_some());
    // With the new top layer selected, Move up is disabled and skipped by Tab.
    harness.get_by_role_and_label(Role::Button, "New effect layer").click();
    harness.run();
    let names = ["Move effect layer up", "Move effect layer down", "New effect layer", "Delete effect layer"];
    let rects: Vec<_> = names.iter().map(|name| harness.get_by_label(name).rect()).collect();
    assert!(rects.windows(2).all(|pair| pair[0].left() < pair[1].left()), "left to right: {rects:?}");
    harness.get_by_label(names[1]).focus();
    harness.run();
    for name in &names[2..] {
        harness.key_press(Key::Tab);
        harness.run();
        assert!(harness.get_by_label(name).accesskit_node().is_focused(), "Tab reaches {name}");
    }
}
