//! The selection overlay and the mask views: when the outline is traced again, and how the view commands change the
//! view state.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_imaging::selection::PreviewMode;
use craft_imaging::trace::{MaskRect, MaskSource};
use craft_imaging::ui::mask_view::{MaskViewChange, MaskViewKeys, MaskViewMode, MaskViewRecipe, MaskViewState};
use craft_imaging::ui::selection::SelectionOverlay;
use craft_ui::canvas::Canvas;
use craft_ui::contract::ShowContainer as _;
use craft_ui::runtime::commands::FirePhase;
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use egui::{Rect, Ui, pos2};

/// A rectangle mask whose revision the test sets.
struct Block {
    revision: u64,
}

impl MaskSource for Block {
    fn size(&self) -> [u32; 2] {
        [200, 150]
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn content_bounds(&self) -> MaskRect {
        MaskRect::new(40, 30, 120, 90)
    }

    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        for (i, value) in out.iter_mut().enumerate() {
            let x = x0 + i as i32 * step as i32;
            *value = f32::from(self.content_bounds().contains(x, y));
        }
    }
}

fn show_overlay(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let (revision, reduction) = (*data.value("revision", 1u64), *data.value("reduction", 1u32));
    let at = ui.available_rect_before_wrap();
    let view = CanvasView::new(Rect::from_min_max(pos2(0.0, 0.0), at.max), Point::new(100.0, 75.0), 1.0);
    let mask = Block { revision };
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| SelectionOverlay::new("Selection", &mask).reduction(reduction).preview(PreviewMode::None).show_at(ui, body.rect));
    if let Some(output) = shown.inner {
        *data.value("retraced", false) = output.retraced;
        *data.value("bounds", None::<MaskRect>) = output.bounds;
    }
    StoryOutput::default()
}

fn retraced(fixture: &Fixture) -> bool {
    *fixture.data().value("retraced", false)
}

#[trial]
fn the_overlay_retraces_only_when_the_revision_or_the_reduction_changes() {
    let mut fixture = Fixture::story(&Story::new("selection-overlay", show_overlay));
    fixture.settle();
    assert!(!retraced(&fixture), "a settled frame draws the cached trace");
    assert_eq!(*fixture.data().value("bounds", None::<MaskRect>), Some(MaskRect::new(40, 30, 120, 90)));
    *fixture.data().value("revision", 1u64) = 2;
    fixture.idle(1);
    assert!(retraced(&fixture), "the first frame after a new revision traces");
    fixture.idle(2);
    assert!(!retraced(&fixture), "and the frames after it draw the cache");
    *fixture.data().value("reduction", 1u32) = 2;
    fixture.idle(2);
    assert_eq!(
        *fixture.data().value("bounds", None::<MaskRect>),
        Some(MaskRect::new(80, 60, 240, 180)),
        "the bounds are in full-size document pixels"
    );
}

#[trial]
fn the_view_commands_pick_cycle_and_hold() {
    let mut state = MaskViewState::default();
    assert_eq!(state.run(MaskViewKeys::OnWhite, FirePhase::Press), Some(MaskViewChange::Mode(MaskViewMode::OnWhite)));
    assert_eq!(state.mode, MaskViewMode::OnWhite);
    assert_eq!(state.run(MaskViewKeys::OnWhite, FirePhase::Repeat), None, "a repeat does nothing");
    state.run(MaskViewKeys::OnLayers, FirePhase::Press);
    state.run(MaskViewKeys::NextView, FirePhase::Press);
    assert_eq!(state.mode, MaskViewMode::OnionSkin, "F wraps to the first view");
    state.run(MaskViewKeys::ShowEdge, FirePhase::Press);
    state.run(MaskViewKeys::ShowOriginal, FirePhase::Press);
    assert!(state.show_edge && state.show_original);
    state.run(MaskViewKeys::ShowEdge, FirePhase::Press);
    assert!(!state.show_edge, "J toggles");
    state.run(MaskViewKeys::HideView, FirePhase::Press);
    assert!(state.hide_view && state.recipe().hidden);
    state.run(MaskViewKeys::HideView, FirePhase::Release);
    assert!(!state.hide_view, "X held hides the view until it is released");
}

#[trial]
fn each_view_keeps_its_own_opacity() {
    let mut state = MaskViewState::default();
    state.set_opacity(MaskViewMode::Overlay, 30.0);
    state.set_opacity(MaskViewMode::OnBlack, 80.4);
    state.set_opacity(MaskViewMode::MarchingAnts, 10.0);
    assert_eq!(state.opacity(MaskViewMode::Overlay), Some(30.0));
    assert_eq!(state.opacity(MaskViewMode::OnBlack), Some(80.0), "rounded to a percent");
    assert_eq!(state.opacity(MaskViewMode::OnionSkin), Some(50.0), "the other views keep theirs");
    assert_eq!(state.opacity(MaskViewMode::MarchingAnts), None, "a view without an opacity ignores it");
    assert_eq!(MaskViewMode::from_id("onLayers"), Some(MaskViewMode::OnLayers));
    assert!(MaskViewMode::ALL.into_iter().all(|mode| MaskViewMode::from_id(mode.id()) == Some(mode)));
}

#[trial]
fn the_recipe_follows_the_view() {
    let recipe = |mode| {
        let mut state = MaskViewState::default();
        state.mode = mode;
        state.recipe()
    };
    let onion: MaskViewRecipe = recipe(MaskViewMode::OnionSkin);
    assert!(onion.hide_other_layers && onion.veils.is_empty() && onion.layer_floor == Some(0.5));
    assert!(recipe(MaskViewMode::MarchingAnts).ants);
    assert_eq!(recipe(MaskViewMode::BlackAndWhite).veils.len(), 2, "black everywhere, then white where selected");
    assert_eq!(recipe(MaskViewMode::OnLayers).layer_floor, Some(0.0));
}
