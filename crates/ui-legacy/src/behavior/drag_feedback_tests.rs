//! Drag feedback in a harness.

use std::cell::Cell;
use std::path::PathBuf;

use egui::accesskit::Role;
use egui::{Context, HoveredFile, IdSalt, Order, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::drag_feedback::layer_id;
use super::{DragBadge, DropVeil, HoveredFiles, set_drop_hint};
use crate::theme::{Base, install};

fn builder() -> HarnessBuilder<()> {
    Harness::builder().with_size([400.0, 300.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
}

// With `set_drop_hint` in a frame, the badge reads the hint; the next frame without it, the count.
#[test]
fn the_badge_shows_the_hint_of_this_frame_only() {
    let hint = Cell::new(true);
    let mut harness = builder().build_ui(|ui| {
        if hint.get() {
            set_drop_hint(ui.ctx(), "Add to “Trips”");
        }
        DragBadge::count("badge", 3).show(ui.ctx());
    });
    harness.hover_at(pos2(100.0, 100.0));
    harness.step();
    harness.step();
    assert!(harness.query_by_label("Add to “Trips”").is_some());
    hint.set(false);
    harness.step();
    harness.step();
    assert!(harness.query_by_label("Add to “Trips”").is_none());
    assert!(harness.query_by_label("3 items").is_some());
}

// The badge's rect starts at the panel padding right of and a stack gap below the pointer, on the tooltip layer, and takes no input.
#[test]
fn the_badge_follows_the_pointer() {
    let mut harness = builder().build_ui(|ui| DragBadge::new("badge", "Paris.jpg").show(ui.ctx()));
    let tokens = crate::theme::current(&harness.ctx);
    let offset = vec2(tokens.space.pad.panel, tokens.space.gap.stack);
    for at in [pos2(100.0, 100.0), pos2(150.0, 120.0)] {
        harness.hover_at(at);
        harness.step();
        harness.step();
        let id = layer_id("badge", IdSalt::new("badge"));
        let rect = harness.ctx.memory(|m| m.area_rect(id)).expect("the badge's area");
        assert_eq!(rect.min, at + offset);
        let layer = harness.ctx.layer_id_at(at + offset + vec2(6.0, 5.0));
        assert!(layer.is_none_or(|l| l.order != Order::Tooltip), "the badge takes no pointer input: {layer:?}");
    }
}

// `HoveredFiles::of` keeps non-empty paths and counts the files without one.
#[test]
fn hovering_files_list_their_paths_and_count_the_unnamed() {
    let seen = Cell::new(HoveredFiles::default());
    let mut harness = builder().build_ui(|ui| seen.set(HoveredFiles::of(ui.ctx())));
    harness.step();
    assert!(seen.take().is_empty());
    let file = |path: Option<&str>| HoveredFile { path: path.map(PathBuf::from), ..Default::default() };
    harness.input_mut().hovered_files = vec![file(Some("/a/b.jpg")), file(None), file(Some("")), file(Some("/c.png"))];
    harness.step();
    let files = seen.take();
    assert_eq!(files.paths, [PathBuf::from("/a/b.jpg"), PathBuf::from("/c.png")]);
    assert_eq!(files.unnamed, 2);
}

// `DropVeil::show` paints the scrim over the whole content rect and the card in its middle with the lines.
#[test]
fn the_veil_covers_the_window_while_files_hover() {
    let lines = vec!["Import 3 photos".to_owned(), "1 preset".to_owned()];
    let mut harness = builder().build_ui(|ui| DropVeil::new("veil", &lines).show(ui.ctx()));
    harness.step();
    harness.step();
    let screen = harness.ctx.content_rect();
    let card = harness.ctx.memory(|m| m.area_rect(layer_id("veil", IdSalt::new("veil")))).expect("the card's area");
    assert!((card.center() - screen.center()).length() < 1.0, "{card:?} in {screen:?}");
    assert!(card.width() < screen.width() && card.height() < screen.height());
    let alert = harness.get_by_role(Role::Alert);
    assert_eq!(alert.accesskit_node().label().as_deref(), Some("Import 3 photos, 1 preset"));
}

// A veil line that wraps keeps its rows centred on one another.
#[test]
fn the_veil_centres_the_rows_of_a_wrapped_line() {
    let lines = vec!["Import these forty-two photos into the album named Holidays by the sea".to_owned()];
    let mut harness = builder().build_ui(|ui| DropVeil::new("veil", &lines).show(ui.ctx()));
    harness.step();
    harness.step();
    let mut centres = Vec::new();
    for clipped in &harness.output().shapes {
        if let egui::Shape::Text(text) = &clipped.shape {
            for placed in &text.galley.rows {
                centres.push(text.pos.x + placed.rect().center().x);
            }
        }
    }
    assert!(centres.len() > 1, "the line wraps: {centres:?}");
    assert!(centres.windows(2).all(|w| (w[0] - w[1]).abs() < 1.5), "{centres:?}");
}

// Neither the badge nor the veil asks for a repaint of its own: pointer and file events start the passes.
#[test]
fn drag_feedback_does_not_spin_the_frame_loop() {
    let lines = vec!["Import".to_owned()];
    let mut harness = builder().build_ui(|ui| {
        DragBadge::new("badge", "Paris.jpg").show(ui.ctx());
        DropVeil::new("veil", &lines).show(ui.ctx());
    });
    harness.hover_at(pos2(100.0, 100.0));
    harness.step();
    harness.step();
    harness.step();
    assert!(!harness.ctx.has_requested_repaint(), "{:?}", harness.ctx.repaint_causes());
}
