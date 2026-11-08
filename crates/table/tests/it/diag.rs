use craft_theme::TextSize;
use craft_ui::contract::Role;
use craft_ui::testing::{Fixture, STORIES, trial};

#[trial]
fn diag_grid_sizes() {
    let entry = STORIES.iter().find(|e| e.crate_name == "craft-table" && e.story.name == "cell_grid").expect("story");
    let mut out = String::new();
    for large in [false, true] {
        let mut f = Fixture::story(&entry.story);
        if large {
            f.text_size(TextSize::Large);
        }
        f.settle();
        let mut texts: Vec<String> = f.texts().iter().map(|t| format!("{}@{:.0},{:.0}{}", t.text, t.rect.min.x, t.rect.min.y, if t.elided { "e" } else { "" })).collect();
        texts.truncate(60);
        out += &format!("large={large} n={} shown={:?} {}\n", f.texts().len(), f.shown(), texts.join(" "));
    }
    let mut f = Fixture::story(&entry.story);
    f.settle();
    let root = f.root_id().and_then(|id| f.node_of(id)).expect("root");
    let span = |f: &Fixture| {
        let rows = f.by_role(Role::ROW);
        format!("n={} y {:?}..{:?}", rows.len(), rows.iter().map(|r| r.rect.min.y).fold(f32::MAX, f32::min), rows.iter().map(|r| r.rect.max.y).fold(f32::MIN, f32::max))
    };
    out += &format!("root {:?} rows before {}\n", root.rect, span(&f));
    f.frame(vec![craft_ui::testing::pointer::moved(root.rect.center())]);
    f.frame(vec![craft_ui::testing::pointer::wheel(-60.0)]);
    out += &format!("rows after wheel {}\n", span(&f));
    panic!("DIAG\n{out}");
}
