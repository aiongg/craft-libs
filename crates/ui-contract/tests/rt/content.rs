//! A kept component whose content comes from a closure keys it by the closure's declared version: it reruns when the
//! version moves and not otherwise, and a closure without a version reruns it at every build of its scope.

use std::cell::Cell;

use craft_behavior::IconId;
use craft_ui_contract::contract::rt::skeleton::IconButton;
use craft_ui_contract::contract::rt::{Component, ComponentKey, Cx, MeasureCx, Response, Ui, show_leaf, window_root};
use craft_ui_contract::contract::{MemoKey, MemoProps, Versioned};
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Rect, Vec2};
use craft_ui_test_kit::rt::Rig;

/// A leaf whose content is a closure the key cannot read: it holds the closure's version instead.
struct Painted<'a> {
    content: Versioned<()>,
    runs: &'a Cell<u32>,
}

impl ComponentKey for Painted<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.content.key(key);
    }
}

impl Component for Painted<'_> {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        self.runs.set(self.runs.get() + 1);
        Vec2::new(40.0, 10.0)
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Probe {
    content: Versioned<()>,
    runs: Cell<u32>,
    other: Rect,
}

fn step(rig: &mut Rig<RecordingRenderer>, probe: &mut Probe) -> bool {
    rig.step_with(&mut |_, cx| {
        window_root(cx, |ui: &mut Ui<'_, '_>| {
            probe.other = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
            show_leaf(Painted { content: probe.content, runs: &probe.runs }, ui);
        })
    })
    .ran
}

/// Builds the scope once more: a click on the other button runs the root's closure, which shows the leaf again.
fn rebuild(rig: &mut Rig<RecordingRenderer>, probe: &mut Probe) -> u32 {
    let before = probe.runs.get();
    let at = probe.other.center();
    let main = rig.main;
    for input in [Input::mouse_move(at), Input::mouse_down(at), Input::mouse_up(at)] {
        rig.inject(main, input);
        step(rig, probe);
    }
    probe.runs.get() - before
}

fn settled(content: Versioned<()>) -> (Rig<RecordingRenderer>, Probe) {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let mut probe = Probe { content, runs: Cell::new(0), other: Rect::NOTHING };
    for _ in 0..6 {
        if !step(&mut rig, &mut probe) {
            break;
        }
    }
    assert!(probe.runs.get() > 0, "the first build runs the leaf");
    (rig, probe)
}

#[test]
fn a_versioned_content_closure_reruns_its_component_only_when_the_version_moves() {
    let (mut rig, mut probe) = settled(Versioned::versioned(1, ()));
    assert_eq!(rebuild(&mut rig, &mut probe), 0, "the same version keeps the leaf");
    probe.content = Versioned::versioned(2, ());
    assert!(rebuild(&mut rig, &mut probe) > 0, "a new version runs the leaf");
    assert_eq!(rebuild(&mut rig, &mut probe), 0, "and keeps it again");
}

#[test]
fn a_content_closure_without_a_version_reruns_its_component_at_every_build() {
    let (mut rig, mut probe) = settled(Versioned::new(()));
    assert!(rebuild(&mut rig, &mut probe) > 0);
    assert!(rebuild(&mut rig, &mut probe) > 0);
}
