//! `Env`: the root, scopes and the interface scale.

use craft_behavior::MotionMode;
use craft_theme::ControlSize;
use craft_theme::{ContrastChoice, Mode, ModeChoice, MotionChoice};
use craft_ui::runtime::env::{Direction, Env, EnvValue, RootEnv, Scale, with_env};
use craft_ui::testing::frames::show_area;
use craft_ui::testing::memory::scoped;
use craft_ui::testing::windows::RawInput;
use craft_ui::testing::{Frames, trial};

#[derive(Clone, Debug, PartialEq, EnvValue)]
#[env(root = Units("pt"))]
struct Units(&'static str);

/// Skeleton: `frames_state::interface_scale_from_the_root_env_changes_pixels_per_point_in_the_next_pass`.
#[trial]
fn interface_scale_from_the_root_env_changes_pixels_per_point_in_the_next_pass() {
    let mut frames = Frames::new();
    let seen = std::cell::Cell::new((0.0f32, 0.0f32));
    let pass = |frames: &mut Frames| {
        frames.frame(vec![], |ui| seen.set((ui.ctx().pixels_per_point(), Env::of(ui).scale.0)));
        seen.get()
    };
    frames.root(RootEnv::new().scale(Scale::new(1.0)));
    assert_eq!(pass(&mut frames), (1.0, 1.0), "scale 1 is the native density");
    frames.root(RootEnv::new().scale(Scale::new(2.0)));
    pass(&mut frames);
    assert_eq!(pass(&mut frames), (2.0, 2.0), "the scale reaches pixels per point and Env::scale by the pass after set_input");
    frames.root(RootEnv::new().scale(Scale::new(1.0)));
    pass(&mut frames);
    assert_eq!(pass(&mut frames), (1.0, 1.0), "and back");
}

/// 2.3 merge table: enabled ANDs and disables the child `Ui`, read-only ORs, motion takes the lowest mode, the
/// innermost value wins, and `Env::of` follows the `Ui` tree.
#[trial]
fn scopes_merge_by_the_table_and_follow_the_ui_tree() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        let root = Env::of(ui);
        assert!(root.enabled && !root.read_only && ui.is_enabled());
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
                        assert!(!ui.is_enabled(), "a disabling scope disables its child Ui, and an inner scope cannot enable it");
                        assert!(inner.read_only, "nothing below can make it writable");
                        assert_eq!(inner.motion.mode, MotionMode::Reduced, "the lowest mode wins");
                        assert_eq!(inner.size, ControlSize::Sm, "the outer scope's size is inherited");
                        assert_eq!(inner.get::<Units>(), Units("in"), "the innermost app value wins");
                        scoped(ui, |child| assert_eq!(Env::of(child).get::<Units>(), Units("in"), "a child Ui sees its parent's entry"));
                    },
                );
                assert_eq!(Env::of(ui).get::<Units>(), Units("mm"), "leaving a scope restores the outer entry");
                assert!(!ui.is_enabled(), "the scope's own Ui stays disabled after an inner scope");
            },
        );
        assert_eq!(Env::of(ui).get::<Units>(), Units("pt"));
        assert!(ui.is_enabled(), "the parent Ui is untouched by the scope");
    });
}

/// 2.1: the `Ui` of an egui `Area` has no entry above it and reads its viewport's root env, not defaults.
#[trial]
fn an_area_ui_reads_the_root_env_of_its_viewport() {
    let mut frames = Frames::new();
    let mut root = RootEnv::new().direction(Direction::Rtl);
    root.values.set(Units("mm"));
    frames.root(root);
    let mut seen = None;
    frames.frame(vec![], |ui| {
        show_area(ui.ctx(), "popup", |area_ui| {
            let env = Env::of(area_ui);
            seen = Some((env.direction, env.get::<Units>()));
        });
    });
    assert_eq!(seen, Some((Direction::Rtl, Units("mm"))));
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

/// `System` resolves to the OS's mode in the pass and follows a change of it; a chosen mode ignores the OS. Where the
/// OS reports no contrast or motion preference, `System` is standard contrast and full motion.
#[trial]
fn system_resolves_from_the_os_and_follows_it() {
    let mut frames = Frames::new();
    let mut system = RootEnv::new();
    (system.mode, system.contrast, system.motion) = (ModeChoice::System, ContrastChoice::System, MotionChoice::System);
    frames.root(system.clone());
    let mut time = 0.0;
    // The OS theme reaches egui through the raw input, as a window integration gives it.
    let mut mode_after = |frames: &mut Frames, os: Option<egui::Theme>| {
        time += 0.1;
        let seen = std::cell::Cell::new(Mode::Light);
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0))),
            time: Some(time),
            system_theme: os,
            ..RawInput::default()
        };
        frames.frame(vec![], |_| {});
        frames.ctx.run_ui(input, |ui| seen.set(Env::of(ui).view().mode())).textures_delta.clear();
        seen.get()
    };
    assert_eq!(mode_after(&mut frames, Some(egui::Theme::Dark)), Mode::Dark, "the OS asks for dark");
    assert_eq!(mode_after(&mut frames, Some(egui::Theme::Light)), Mode::Light, "and follows the change to light");
    assert_eq!(mode_after(&mut frames, None), Mode::Light, "an OS that reports nothing leaves its last answer, light here");
    frames.root(RootEnv::new().mode(Mode::Light));
    assert_eq!(mode_after(&mut frames, Some(egui::Theme::Dark)), Mode::Light, "a chosen mode ignores the OS");
    let resolved = system.resolved(&frames.ctx);
    assert_eq!((resolved.contrast, resolved.motion), (craft_theme::Contrast::Standard, MotionMode::Full));
}
