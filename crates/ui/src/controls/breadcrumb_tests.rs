//! `BreadcrumbBar` and path segments.

use super::*;

fn crumbs(path: &str) -> Vec<(String, String)> {
    path_crumbs(path).into_iter().map(|c| (c.label, c.key)).collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| ((*a).to_string(), (*b).to_string())).collect()
}

// `path_crumbs`: "/a/b" keys "/a" and "/a/b"; "C:\\x" keys "C:\\" and "C:\\x"; a UNC path keeps its leading "\\\\".
#[test]
fn path_segments_keep_the_paths_spelling() {
    assert_eq!(crumbs("/a/b"), pairs(&[("a", "/a"), ("b", "/a/b")]));
    assert_eq!(crumbs("a//b/"), pairs(&[("a", "a"), ("b", "a//b")]), "empty parts are skipped, the spelling kept");
    assert_eq!(crumbs("C:\\x"), pairs(&[("C:", "C:\\"), ("x", "C:\\x")]));
    assert_eq!(crumbs("C:/x"), pairs(&[("C:", "C:/"), ("x", "C:/x")]));
    assert_eq!(
        crumbs("\\\\server\\share\\dir"),
        pairs(&[("server", "\\\\server"), ("share", "\\\\server\\share"), ("dir", "\\\\server\\share\\dir")])
    );
    assert!(crumbs("").is_empty() && crumbs("/").is_empty());
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod bar {
    use egui::accesskit::Role;
    use egui::{Key, Ui};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::*;
    use crate::theme::{Base, install};

    struct Host {
        crumbs: Vec<Crumb<String>>,
        opened: Vec<String>,
    }

    fn harness(path: &str, width: f32) -> Harness<'static, Host> {
        HarnessBuilder::default().with_size([width, 80.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                host.opened.extend(BreadcrumbBar::new("folder", &host.crumbs).show(ui).opened);
            },
            Host { crumbs: path_crumbs(path), opened: Vec::new() },
        )
    }

    // In a harness: clicking an ancestor reports `opened`; the last segment is not a button.
    #[test]
    #[ignore = "needs S5S-F5 text::elided_galley"]
    fn a_click_on_an_ancestor_opens_it_and_the_last_never_reports() {
        let mut harness = harness("/photos/2026/trip", 480.0);
        harness.run();
        assert!(harness.query_by_label("trip").is_some_and(|n| n.accesskit_node().role() == Role::Label), "the last segment is text");
        harness.get_by_label("2026").click();
        harness.run();
        harness.get_by_label("trip").click();
        harness.run();
        harness.get_by_label("photos").focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().opened, ["/photos/2026", "/photos"]);
    }

    // In a harness: a narrow bar shows the last segments and a "…" button whose menu opens a hidden one.
    #[test]
    #[ignore = "needs S5S-F5 text::elided_galley"]
    fn segments_that_do_not_fit_go_behind_an_ellipsis() {
        let mut harness = harness("/a/b/c/d/e", 200.0);
        harness.run();
        assert!(harness.query_by_label("a").is_none() && harness.query_by_label("b").is_none() && harness.query_by_label("c").is_none());
        assert!(harness.query_by_label("d").is_some() && harness.query_by_label("e").is_some(), "two fit in 200 points");
        harness.get_by_label("Show the whole path").click();
        harness.run();
        let rows: Vec<String> = harness.query_all_by_role(Role::MenuItem).filter_map(|n| n.accesskit_node().label()).collect();
        assert_eq!(rows, ["a", "b", "c"], "the hidden segments, the nearest last");
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::ArrowDown);
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().opened, ["/a/b"]);
    }
}
