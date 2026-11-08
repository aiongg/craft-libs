use craft_theme::TextSize;
use craft_ui::contract::Role;
use craft_ui::testing::{Fixture, STORIES, trial};

#[trial]
fn diag_grid_sizes() {
    let entry = STORIES.iter().find(|e| e.crate_name == "craft-table" && (e.type_name)().contains("CellGrid") && e.story.name == "cell_grid").expect("story");
    let mut out = String::new();
    for large in [false, true] {
        let mut f = Fixture::story(&entry.story);
        if large {
            f.text_size(TextSize::Large);
        }
        f.settle();
        out += &format!("large={large} shown={:?}\n", f.shown());
        for t in f.texts() {
            if ["14", "13", "1", "2", "A", "B"].contains(&t.text.as_str()) || t.rect.max.y > f.shown().max.y - 30.0 || t.rect.max.x > f.shown().max.x - 30.0 {
                out += &format!("  text {:?} {:?} elided={}\n", t.text, t.rect, t.elided);
            }
        }
    }
    let mut f = Fixture::story(&entry.story);
    f.settle();
    let root = f.root_id().and_then(|id| f.node_of(id)).expect("root");
    out += &format!("root {:?}\n", root.rect);
    let rows = f.by_role(Role::ROW);
    out += &format!("rows before: {:?}\n", rows.iter().map(|r| r.rect).collect::<Vec<_>>());
    f.frame(vec![craft_ui::testing::pointer::moved(root.rect.center())]);
    f.frame(vec![craft_ui::testing::pointer::wheel(-60.0)]);
    let rows = f.by_role(Role::ROW);
    out += &format!("rows after wheel: {:?}\n", rows.iter().map(|r| r.rect).collect::<Vec<_>>());
    panic!("DIAG\n{out}");
}
