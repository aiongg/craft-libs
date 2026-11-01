//! `ChipField` layout and keys.

use super::*;

/// The lines as (first, end) pairs.
fn lines(widths: &[f32], row_width: f32) -> Vec<(usize, usize)> {
    chip_lines(widths, row_width, 6.0).into_iter().map(|line| (line.start, line.end)).collect()
}

// `chip_lines` starts a new line for a chip that does not fit and gives an over-wide chip a line of its own.
#[test]
fn chips_move_whole_to_the_next_line() {
    assert_eq!(lines(&[40.0, 40.0, 40.0], 100.0), [(0, 2), (2, 3)], "40 + 6 + 40 fits, the third would need 138");
    assert_eq!(lines(&[40.0, 40.0], 86.0), [(0, 2)], "exactly the row's width fits");
    assert_eq!(lines(&[40.0, 200.0, 10.0], 100.0), [(0, 1), (1, 2), (2, 3)], "an over-wide chip has a line of its own");
    assert_eq!(lines(&[300.0], 100.0), [(0, 1)]);
    assert!(lines(&[], 100.0).is_empty());
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod field {
    use egui::accesskit::Role;
    use egui::{Key, Ui};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::*;
    use crate::theme::{Base, install};

    #[derive(Default)]
    struct Host {
        names: Vec<String>,
        max_lines: Option<usize>,
        removed: Vec<usize>,
        clicked: Vec<usize>,
    }

    fn harness(names: &[&str], max_lines: Option<usize>) -> Harness<'static, Host> {
        HarnessBuilder::default().with_size([260.0, 160.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let chips: Vec<Chip> = host.names.iter().map(|n| Chip::new(n.as_str())).collect();
                let mut field = ChipField::new("keywords", "Keywords", &chips);
                if let Some(lines) = host.max_lines {
                    field = field.max_lines(lines);
                }
                let out = field.show(ui);
                host.clicked.extend(out.clicked);
                if let Some(i) = out.removed {
                    host.removed.push(i);
                    host.names.remove(i);
                }
            },
            Host { names: names.iter().map(|n| (*n).to_string()).collect(), max_lines, ..Host::default() },
        )
    }

    // In a harness: a click on the × reports `removed`, on the name `clicked`; Delete on the focused chip reports `removed` and the focus moves on.
    #[test]
    fn the_cross_removes_and_the_name_clicks() {
        let mut harness = harness(&["alpha", "beta", "gamma"], None);
        harness.run();
        harness.get_by_label("beta").click();
        harness.run();
        assert_eq!((harness.state().clicked.clone(), harness.state().removed.clone()), (vec![1], vec![]));
        harness.get_by_label("Remove “alpha”").click();
        harness.run();
        assert_eq!(harness.state().removed, [0]);
        assert_eq!(harness.state().names, ["beta", "gamma"]);
        harness.get_by_label("beta").focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        harness.run();
        assert_eq!(harness.state().names, ["gamma"]);
        assert!(harness.get_by_label("gamma").accesskit_node().is_focused(), "the focus moved to the next chip");
    }

    // In a harness: with `max_lines(1)` the hidden chips are counted on "+N more", whose menu removes one.
    #[test]
    fn chips_past_the_last_line_go_behind_more() {
        let names = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel"];
        let mut harness = harness(&names, Some(1));
        harness.run();
        let more = harness.get_by(|n| n.label().is_some_and(|l| l.starts_with('+') && l.ends_with("more")));
        let hidden: usize = more.accesskit_node().label().unwrap().trim_start_matches('+').trim_end_matches(" more").parse().unwrap();
        assert!(hidden > 0 && hidden < names.len());
        assert!(harness.query_by_label(names[names.len() - 1]).is_none(), "a hidden chip is not drawn");
        more.click();
        harness.run();
        let rows = harness.query_all_by_role(Role::MenuItem).count();
        assert_eq!(rows, hidden);
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().removed, [names.len() - hidden], "the first hidden chip");
    }
}
