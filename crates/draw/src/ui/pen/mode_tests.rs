//! Tests of the path mode table and the Path Options popover.

use egui::accesskit::Role;
use egui::{Context, Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{BezierTool, PATH_OPTIONS_COLOR_KEY, PathMode, PathOptionsPopover, PathOptionsPopoverResponse, PathOptionsSettings, PathTool, PenMode};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

fn mode(bezier: BezierTool, draft: Option<PenMode>, draft_senses: bool) -> PathMode {
    PathMode { bezier, draft, draft_senses }
}

fn mods(shift: bool, alt: bool, command: bool) -> Modifiers {
    Modifiers { shift, alt, command, ctrl: command, ..Modifiers::NONE }
}

// `PathMode::resolve` over every row of its table: each tool, drawing or not, Auto Add/Delete on
// and off, with no key, ⇧, ⌥ and ⌘.
#[test]
fn path_mode_resolves_the_pen_tools_and_modifiers() {
    use BezierTool::{AddAnchor, AutoAddDelete, Convert, Curvature, DeleteAnchor, Direct, Show};
    use PathTool as T;
    let (none, shift, alt, cmd) = (Modifiers::NONE, mods(true, false, false), mods(false, true, false), mods(false, false, true));
    let pen = Some(PenMode::Pen);
    let curve = Some(PenMode::Curvature);
    let r = PathMode::resolve;

    // ⌘ makes any pen or anchor tool Direct Selection; the draft shows only while drawing.
    for tool in [T::Pen, T::AddAnchor, T::DeleteAnchor, T::ConvertPoint] {
        assert_eq!(r(tool, cmd, false, true), mode(Direct, None, false), "{tool:?}");
        assert_eq!(r(tool, cmd, true, true), mode(Direct, pen, false), "{tool:?}");
    }
    assert_eq!(r(T::Curvature, cmd, false, false), mode(Direct, None, false));
    assert_eq!(r(T::Curvature, cmd, true, false), mode(Direct, curve, false));

    // The Pen.
    assert_eq!(r(T::Pen, none, true, true), mode(Show, pen, true));
    assert_eq!(r(T::Pen, none, false, true), mode(AutoAddDelete, pen, true));
    assert_eq!(r(T::Pen, none, false, false), mode(Show, pen, true));
    assert_eq!(r(T::Pen, alt, false, true), mode(Convert, pen, true));
    for held in [shift, alt] {
        assert_eq!(r(T::Pen, held, true, true), mode(Show, pen, true));
    }
    assert_eq!(r(T::Pen, shift, false, true), mode(Show, pen, true));

    // The Curvature Pen.
    for held in [none, shift, alt] {
        assert_eq!(r(T::Curvature, held, false, true), mode(Curvature, curve, true));
        assert_eq!(r(T::Curvature, held, true, true), mode(Show, curve, true));
    }

    // The anchor tools: ⌥'s swap is the overlay's; the draft shows without sensing.
    for held in [none, alt] {
        assert_eq!(r(T::AddAnchor, held, false, true), mode(AddAnchor, None, false));
        assert_eq!(r(T::AddAnchor, held, true, true), mode(AddAnchor, pen, false));
        assert_eq!(r(T::DeleteAnchor, held, false, true), mode(DeleteAnchor, None, false));
        assert_eq!(r(T::ConvertPoint, held, false, true), mode(Convert, None, false));
        assert_eq!(r(T::ConvertPoint, held, true, true), mode(Convert, pen, false));
    }
    for held in [none, shift, alt, cmd] {
        assert_eq!(r(T::DirectSelection, held, false, true), mode(Direct, None, false));
        assert_eq!(r(T::DirectSelection, held, true, true), mode(Direct, pen, false));
    }
}

// `PathOptionsSettings::default()` has the rubber band off and no colour or thickness; JSON
// round-trips and `{}` deserialises to the default.
#[test]
fn path_options_settings_default_to_photoshops() {
    let d = PathOptionsSettings::default();
    assert!(!d.rubber_band && d.color.is_none() && d.thickness.is_none());
    let set = PathOptionsSettings { rubber_band: true, color: Some([1, 2, 3]), thickness: Some(2.5) };
    let Ok(json) = serde_json::to_string(&set) else { panic!("serialises") };
    assert!(json.contains("rubberBand"));
    assert_eq!(serde_json::from_str::<PathOptionsSettings>(&json).ok(), Some(set));
    assert_eq!(serde_json::from_str::<PathOptionsSettings>("{}").ok(), Some(d));
}

struct Host {
    settings: PathOptionsSettings,
    open: bool,
    last: Option<PathOptionsPopoverResponse>,
    events: Vec<EditEvent<PathOptionsSettings>>,
    asked: Vec<(String, Option<String>)>,
}

fn harness() -> Harness<'static, Host> {
    let host = Host { settings: PathOptionsSettings::default(), open: false, last: None, events: Vec::new(), asked: Vec::new() };
    let mut harness = Harness::builder()
        .with_size([400.0, 400.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let r = PathOptionsPopover::new("path-options", &host.settings).show(ui, &mut host.open);
                if let Some(value) = r.event.as_ref().and_then(EditEvent::value) {
                    host.settings = *value;
                }
                host.events.extend(r.event.clone());
                host.asked.extend(r.pick_color.iter().map(|c| (c.key.clone(), c.title.clone())));
                host.last = Some(r);
            },
            host,
        );
    harness.run();
    harness
}

fn gear<'a>(harness: &'a Harness<'_, Host>) -> Node<'a> {
    harness.get_by_role_and_label(Role::Button, "Path Options")
}

// The gear opens the popover; the checkbox reports a Set with the rubber band on; the swatch
// reports `pick_color` keyed `pathOptions.color`; Esc closes and returns focus to the gear.
#[test]
fn path_options_popover_reports_changes_and_asks_for_the_colour() {
    let mut harness = harness();
    gear(&harness).click();
    harness.run_steps(8);
    assert!(harness.state().open);
    assert!(harness.query_by_role_and_label(Role::SpinButton, "Thickness").is_some());

    harness.get_by_role_and_label(Role::CheckBox, "Rubber Band").click();
    harness.run_steps(4);
    assert!(matches!(harness.state().events.last(), Some(EditEvent::Set { value, .. }) if value.rubber_band));
    assert!(harness.state().settings.rubber_band);

    harness.get_by_role_and_label(Role::ColorWell, "Color").click();
    harness.run_steps(4);
    assert_eq!(harness.state().asked, [(PATH_OPTIONS_COLOR_KEY.to_owned(), Some("Color".to_owned()))]);

    harness.key_press(Key::Escape);
    harness.run_steps(8);
    assert!(!harness.state().open);
    assert!(gear(&harness).accesskit_node().is_focused(), "focus returns to the gear");
}
