use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use super::{JAPANESE, KOREAN, Screens, compose, ime_events};
use crate::fonts::CraftFace;

const ARABIC: CraftFace = CraftFace {
    family: "Noto Sans Arabic",
    style: "Regular",
    scripts: &["Arab"],
    bytes: include_bytes!("../../tests/fonts/NotoSansArabic-screens.ttf"),
    index: 0,
};
// The subset has the test strings' kana, Han and Hangul, so it stands in for every CJK face.
const CJK: CraftFace = CraftFace {
    family: "Noto Sans CJK SC",
    style: "Regular",
    scripts: &["Jpan", "Kore", "Hans", "Hant"],
    bytes: include_bytes!("../../tests/fonts/NotoSansCJKsc-screens.otf"),
    index: 0,
};

fn screens() -> Screens {
    Screens::new("unit").size([300.0, 80.0]).fonts(|fonts| fonts.craft_faces(&[ARABIC, CJK]))
}

#[test]
fn ime_events_end_with_the_commit() {
    let events = ime_events(&KOREAN);
    assert_eq!(events.len(), 4);
    assert_eq!(events.last(), Some(&egui::Event::Ime(egui::ImeEvent::Commit("한".to_owned()))));
}

#[test]
fn compose_types_into_the_focused_text_field() {
    for (language, composition) in [("ko", KOREAN), ("ja", JAPANESE)] {
        let mut harness = screens().harness_state(
            language,
            |ui, _, text: &mut String| {
                let label = ui.label("Name");
                ui.text_edit_singleline(text).labelled_by(label.id);
            },
            String::new(),
        );
        harness.get_by_role(Role::TextInput).focus();
        harness.run();
        compose(&mut harness, &composition);
        assert_eq!(harness.state(), composition.commit, "{language}");
    }
}

#[test]
fn right_to_left_screens_start_at_the_right() {
    for (language, at_right) in [("ar", true), ("ja", false)] {
        let harness = screens().harness(language, |ui, _| _ = ui.label("ملف"));
        let label = harness.get_by_label("ملف").rect();
        assert_eq!(label.max.x > 250.0, at_right, "{language}: {label:?}");
        assert_eq!(label.min.x < 50.0, !at_right, "{language}: {label:?}");
    }
}

#[test]
fn a11y_helpers_on_the_stock_egui_story() {
    use egui_kittest::Harness;

    use super::a11y;
    use crate::theme::{Base, install};

    let story = crate::story::registry().into_iter().find(|s| s.id == "foundations/stock-egui/widgets").expect("story");
    let mut args = story.defaults();
    // kittest's own check would stop at the unlabelled text field before ours runs.
    let mut harness = Harness::builder()
        .with_size([400.0, 400.0])
        .with_accessibility_check(false)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(move |ui| (story.show)(ui, &mut args));
    harness.run();

    let missing = a11y::unlabelled(&harness);
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing[0].starts_with("TextInput"), "{missing:?}");

    let order: Vec<(Role, String)> = a11y::tab_order(&mut harness).into_iter().map(|stop| (stop.role, stop.label)).collect();
    let labels: Vec<&str> = order.iter().map(|(_, label)| label.as_str()).collect();
    assert_eq!(
        labels,
        ["Button", "Selected", "Small", "Checkbox", "One", "Two", "Three", "Slider", "Slider", "", "Combo box", "Collapsing header", "Hyperlink"]
    );
    assert_eq!(order[7].0, Role::Slider);
    assert_eq!(order[9].0, Role::TextInput);

    // Stock egui shows keyboard focus with the active widget border.
    let widgets = harness.ctx.global_style().visuals.widgets.clone();
    assert_ne!(widgets.active.bg_stroke.color, widgets.inactive.bg_stroke.color);
    assert!(harness.get_by_label("Button").is_focused());
    a11y::assert_focus_ring(&harness, widgets.active.bg_stroke.color);
    harness.get_by_label("Small").focus();
    harness.run();
    let (_, rings) = a11y::focus_rings(&harness).expect("focus");
    assert!(rings.iter().all(|ring| ring.stroke.color != widgets.inactive.bg_stroke.color), "{rings:?}");
}
