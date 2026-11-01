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

// Ancestors are in the text colour, the "…" in the muted one, and hovering an ancestor adds no underline.
#[test]
fn ancestors_keep_the_label_colour_and_hover_draws_no_underline() {
    use egui::{Context, Event, RawInput, Rect, Shape, pos2, vec2};

    use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let crumbs = path_crumbs("/a/b/c/d/e");
    let frame = |events: Vec<Event>| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 80.0))), events, ..Default::default() };
        let mut tokens = None;
        let output = ctx.run_ui(input, |ui| {
            tokens = Some(Tokens::of(ui));
            BreadcrumbBar::new("folder", &crumbs).show(ui);
        });
        let (mut texts, mut lines) = (Vec::new(), 0);
        for clipped in &output.shapes {
            match &clipped.shape {
                Shape::Text(t) => {
                    let section = &t.galley.job.sections[0];
                    texts.push((t.galley.text().to_owned(), section.format.color));
                }
                Shape::LineSegment { .. } => lines += 1,
                _ => {}
            }
        }
        output.drop_without_applying_deltas();
        (tokens.expect("the pass ran"), texts, lines)
    };
    frame(vec![]);
    let (tokens, texts, _) = frame(vec![Event::PointerMoved(pos2(120.0, 12.0))]);
    let (normal, muted) = (tokens.color.text.normal.to_egui(), tokens.color.text.muted.to_egui());
    let colour = |text: &str| texts.iter().find(|(t, _)| t == text).map(|(_, c)| *c);
    assert_eq!(colour("…"), Some(muted));
    assert_eq!(colour("d"), Some(normal), "an ancestor is in the label colour, hovered or not");
    let (_, _, lines) = frame(vec![Event::PointerMoved(pos2(120.0, 12.0))]);
    assert_eq!(lines, 0, "a hovered ancestor has no underline");
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
