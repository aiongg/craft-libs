//! `Env`: the root, scopes and the interface scale.

use craft_behavior::MotionMode;
use craft_host::{AppInfo, Host};
use craft_theme::ControlSize;
use craft_ui::runtime::env::{AgeStyle, Direction, Env, EnvValue, RootEnv, Scale, with_env};
use craft_ui::testing::frames::show_area;
use craft_ui::testing::memory::scoped;
use craft_ui::testing::{Frames, trial};
use std::sync::Arc;

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

/// 2.3: a scope's body panicking leaves no entry behind.
#[trial]
fn a_panicking_scope_body_leaves_the_outer_env() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_env(
                ui,
                |o| {
                    o.values.set(Units("mm"));
                },
                |_| panic!("scope body"),
            );
        }));
        assert!(unwound.is_err());
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

/// 2.1: `Units::times`, `duration` and `file_size` and `Env::relative_age` write the scope's language and digits.
#[trial]
fn units_and_ages_read_the_envs_language() {
    let catalog = "@id\tunits.times\t{n} fois\n@id\tunits.seconds\t{n} sec\n@id\tage.justNow\tÀ l'instant\nunit\t{n} MB\t{n} Mo\n";
    let (i18n, problems) = craft_i18n::I18n::from_owned([("fr".to_owned(), String::new(), catalog.to_owned())], &["fr"]);
    assert!(problems.is_empty(), "{problems:?}");
    i18n.set_format(craft_i18n::Format::from_tag("fr-FR"));
    let fr = i18n.from_code("fr");
    assert!(fr.is_some(), "the catalog offers French");
    let host = Arc::new(
        Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).i18n(Arc::new(i18n)).lang(fr.unwrap_or(craft_i18n::Lang::EN)).build(),
    );
    let mut frames = Frames::new();
    frames.host(host);
    let mut seen = None;
    frames.frame(vec![], |ui| {
        let env = Env::of(ui);
        let units = env.units();
        seen = Some((units.times(3.0), units.duration(1.5), units.file_size(12 << 20), env.relative_age(10, AgeStyle::Long)));
    });
    let (times, duration, size, age) = seen.unwrap_or_default();
    assert_eq!((times.as_str(), duration.as_str(), size.as_str(), age.as_str()), ("3 fois", "1,5 sec", "12,0 Mo", "À l'instant"));
}

/// `EnvView::mode` reads the light or dark mode the app runs in.
#[trial]
fn the_env_view_reports_the_mode_the_app_runs_in() {
    use craft_theme::{Contrast, Mode};
    use craft_ui::testing::Fixture;
    let seen = std::rc::Rc::new(std::cell::Cell::new(Mode::Light));
    let read = std::rc::Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| read.set(Env::of(ui).view().mode()));
    fixture.theme(Mode::Dark, Contrast::Standard).frame(vec![]);
    assert_eq!(seen.get(), Mode::Dark);
    fixture.theme(Mode::Light, Contrast::Standard).frame(vec![]);
    assert_eq!(seen.get(), Mode::Light);
}
