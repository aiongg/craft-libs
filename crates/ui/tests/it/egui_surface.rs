//! Tests of the egui APIs, behaviours and fork patches the runtime relies on; run on every egui or fork bump.

use craft_i18n::Direction;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::{Frames, surface};
use craft_ui::testing::trial;
use std::path::Path;

/// PL2: the input hook of a child viewport runs before the viewport is pushed; `on_begin_pass` runs after.
#[trial]
fn plugin_input_hook_runs_before_the_viewport_is_pushed() {
    let facts = surface::child_viewport_hooks();
    assert!(facts.child_input_hooks > 0, "the child's input hook ran: {facts:?}");
    assert!(facts.input_hooks_see_parent, "during the child's input hook the current viewport is the parent: {facts:?}");
    assert!(facts.child_begin_hooks > 0, "the begin hook runs with the child as the current viewport: {facts:?}");
}

/// IN3: with one press per pass, `ViewportState::hits` is the hit test at the last pointer event's position.
#[trial]
fn hits_follow_the_last_pointer_event_of_the_pass() {
    let facts = surface::press_hits();
    assert!(facts.press_last_is_press_target, "a press as the last pointer event: hits.click is the widget under the press: {facts:?}");
    assert!(facts.move_last_is_move_target, "a move after the press: hits.click is the widget under the pointer's end position: {facts:?}");
}

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// IN4: a swallowed press gets empty `press_hits` while `hits` still names the widget, so egui and the runtime
/// would disagree; nothing in the library may swallow presses.
#[trial]
fn no_library_code_swallows_presses() {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty(), "the library sources were found");
    let banned = ["swallow_presses_outside", "swallow_outside_press", "allow_presses_in"];
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for word in banned {
            assert!(!text.contains(word), "{} names {word}", file.display());
        }
    }
}

/// The fork's `Context::set_layer_order`: listed layers go above the unlisted ones of their order, in list
/// order; the others keep their order; unknown ids are ignored.
#[trial]
fn set_layer_order_moves_listed_layers_above_unlisted_ones() {
    let facts = surface::layer_order();
    assert!(facts.listed_on_top, "{facts:?}");
    assert!(facts.unlisted_keep_order, "{facts:?}");
    assert!(facts.unknown_ignored, "{facts:?}");
}

/// The fork's public `Context::register_accesskit_parent`: a member with no egui widget attaches its node to the
/// registered parent's node.
#[trial]
fn registered_accesskit_parent_owns_the_member_node() {
    assert!(surface::registered_parent_owns_member_node());
}

/// PA1 and PA2: a discard pass gets no input events, and the frame number stays while the pass number moves.
#[trial]
fn discard_pass_gets_no_events_and_keeps_the_frame_number() {
    let passes = surface::discard_passes();
    assert_eq!(passes.len(), 2, "one discard pass per frame: {passes:?}");
    assert!(passes[0].events > 0, "the first pass gets the events: {passes:?}");
    assert_eq!(passes[1].events, 0, "the discard pass gets none: {passes:?}");
    assert_eq!(passes[0].frame, passes[1].frame, "{passes:?}");
    assert_eq!(passes[0].pass + 1, passes[1].pass, "{passes:?}");
}

/// The option table's focus row: egui clears focus on a bare Escape unless the filter given to
/// `request_focus_with_filter` lists Escape.
#[trial]
fn a_focus_filter_that_lists_escape_keeps_focus_on_a_bare_escape() {
    assert!(surface::focus_survives_escape(true), "the filter keeps focus");
    assert!(!surface::focus_survives_escape(false), "control: without the filter egui clears focus");
}

/// The pinned egui has no `RawInput.system_locale`; a backend that derived the direction from the locale would
/// write it into the style before the first pass. The root's direction wins in every pass, the first included.
#[trial]
fn system_locale_never_sets_the_direction() {
    for (preset_rtl, root) in [(true, Direction::Ltr), (false, Direction::Rtl)] {
        let mut frames = Frames::new();
        surface::preset_style_direction(&frames.ctx, preset_rtl);
        frames.root(RootEnv::default().direction(root));
        let mut seen = Vec::new();
        frames.frame(Vec::new(), |ui| seen.push(ui.style().right_to_left));
        frames.frame(Vec::new(), |ui| seen.push(ui.style().right_to_left));
        assert!(!seen.is_empty() && seen.iter().all(|rtl| *rtl == root.is_rtl()), "{root:?}: {seen:?}");
    }
}
