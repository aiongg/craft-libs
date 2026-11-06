//! `Env`: the root, scopes and the interface scale.

use crate::support::Frames;
use craft_behavior::MotionMode;
use craft_theme::ControlSize;
use craft_ui::runtime::egui::memory::scoped;
use craft_ui::runtime::env::{Direction, Env, EnvValue, RootEnv, Scale, with_env};
use craft_ui::testing::trial;

#[derive(Clone, Debug, PartialEq)]
struct Units(&'static str);

impl EnvValue for Units {
    fn root() -> Self {
        Units("pt")
    }
}

/// Skeleton: `frames_state::interface_scale_from_the_root_env_changes_pixels_per_point_in_the_next_pass`.
#[trial]
fn interface_scale_from_the_root_env_changes_pixels_per_point_in_the_next_pass() {
    let mut frames = Frames::new();
    let seen = std::cell::Cell::new((0.0f32, 0.0f32));
    let pass = |frames: &mut Frames| {
        frames.frame(vec![], |ui| seen.set((ui.ctx().pixels_per_point(), Env::of(ui).scale.0)));
        seen.get()
    };
    frames.root(RootEnv::new().scale(Scale(1.0)));
    assert_eq!(pass(&mut frames), (1.0, 1.0), "scale 1 is the native density");
    frames.root(RootEnv::new().scale(Scale(2.0)));
    pass(&mut frames);
    assert_eq!(pass(&mut frames), (2.0, 2.0), "the scale reaches pixels per point and Env::scale by the pass after set_input");
    frames.root(RootEnv::new().scale(Scale(1.0)));
    pass(&mut frames);
    assert_eq!(pass(&mut frames), (1.0, 1.0), "and back");
}

/// 2.3 merge table: enabled ANDs, read-only ORs, motion takes the lowest mode, the innermost value wins, and
/// `Env::of` follows the `Ui` tree.
#[trial]
fn scopes_merge_by_the_table_and_follow_the_ui_tree() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        let root = Env::of(ui);
        assert!(root.enabled && !root.read_only);
        assert_eq!(root.size, ControlSize::Md);
        assert_eq!(root.get::<Units>(), Units("pt"));
        with_env(
            ui,
            |o| {
                o.size = Some(ControlSize::Sm);
                o.enabled = Some(false);
                o.read_only = Some(true);
                o.motion = Some(MotionMode::Reduced);
                o.values.set(Units("mm"));
            },
            |ui| {
                with_env(
                    ui,
                    |o| {
                        o.enabled = Some(true);
                        o.read_only = Some(false);
                        o.motion = Some(MotionMode::Full);
                        o.values.set(Units("in"));
                    },
                    |ui| {
                        let inner = Env::of(ui);
                        assert!(!inner.enabled, "nothing below can enable again");
                        assert!(inner.read_only, "nothing below can make it writable");
                        assert_eq!(inner.motion.mode, MotionMode::Reduced, "the lowest mode wins");
                        assert_eq!(inner.size, ControlSize::Sm, "the outer scope's size is inherited");
                        assert_eq!(inner.get::<Units>(), Units("in"), "the innermost app value wins");
                        scoped(ui, |child| assert_eq!(Env::of(child).get::<Units>(), Units("in"), "a child Ui sees its parent's entry"));
                    },
                );
                assert_eq!(Env::of(ui).get::<Units>(), Units("mm"), "leaving a scope restores the outer entry");
            },
        );
        assert_eq!(Env::of(ui).get::<Units>(), Units("pt"));
    });
}

/// Skeleton: `frames_state::ltr_scope_inside_rtl_lays_out_left_to_right`, at the scope's `Ui`.
#[trial]
fn a_scope_that_changes_direction_builds_its_own_style_and_layout() {
    let mut frames = Frames::new();
    for (root, scope) in [(Direction::Rtl, Direction::Ltr), (Direction::Ltr, Direction::Rtl)] {
        frames.root(RootEnv::new().direction(root));
        let mut seen = None;
        frames.frame(vec![], |ui| {
            assert_eq!(Env::of(ui).direction, root);
            assert_eq!(ui.style().right_to_left, root.is_rtl(), "the root's theme styles carry the root direction");
            with_env(ui, |o| o.direction = Some(scope), |ui| seen = Some((Env::of(ui).direction, ui.style().right_to_left)));
        });
        assert_eq!(seen, Some((scope, scope.is_rtl())), "the scope's Ui reads {scope:?} under a {root:?} root");
    }
}
