use egui::accesskit::Role;
use egui::{Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::testing::{HarnessExt as _, a11y};
use crate::theme::{Base, install};

/// The chosen option, and where option 1's content sits.
type State = (usize, Option<Rect>);

fn samples() -> Harness<'static, State> {
    Harness::builder().with_size([320.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, (choice, content): &mut State| {
            let mut set = RadioSet::new("sizes", "Sizes", choice).item(0).item(1).item(2).begin(ui);
            let _ = set.radio(ui, 0, "None");
            let _ = set.radio_with(
                ui,
                1,
                RadioOption::new("Small").description("a 20 pt sample").content(|ui| {
                    let (_, rect) = ui.allocate_space(vec2(20.0, 20.0));
                    *content = Some(rect);
                }),
            );
            let _ = set.radio(ui, 2, "Large");
            let _ = set.end(ui);
        },
        (0, None),
    )
}

#[test]
fn an_option_with_content_is_one_tab_stop_with_a_description_and_the_content_selects_it() {
    let mut harness = samples();
    harness.run();
    let radios: Vec<_> = harness.query_all_by_role(Role::RadioButton).collect();
    assert_eq!(radios.len(), 3, "no extra Button nodes or radio buttons");
    assert!(harness.query_all_by_role(Role::Button).next().is_none());
    assert_eq!(harness.get_by_label("Small").accesskit_node().data().description(), Some("a 20 pt sample"));
    assert_eq!(a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect::<Vec<_>>(), ["None", "Small", "Large"]);

    let content = harness.state().1.expect("the content was drawn");
    let hit = harness.get_by_label("Small").rect();
    assert!(hit.contains_rect(content), "the content {content:?} sits inside the option's hit area {hit:?}");
    harness.click_at(content.center());
    assert_eq!(harness.state().0, 1, "a click on the content selected the option");
}
