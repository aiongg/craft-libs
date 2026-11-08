use craft_ui::testing::access::{NodeKey, focused_key};
use craft_ui::testing::{Fixture, STORIES, trial};
use egui::{Key, Modifiers};

fn story(name: &str) -> &'static craft_ui::testing::StoryRegistration {
    STORIES.iter().find(|e| e.crate_name == "craft-browse" && e.story.name == name).expect("story")
}

fn all(f: &Fixture, node: NodeKey, found: &mut Vec<NodeKey>) {
    found.push(node);
    for c in f.tree().children_of(node) {
        if !found.contains(&c) {
            all(f, c, found);
        }
    }
}

fn lowest(f: &Fixture, root: NodeKey) -> String {
    let mut found = Vec::new();
    all(f, root, &mut found);
    let mut rows: Vec<(f32, String)> = found
        .into_iter()
        .filter_map(|k| f.tree().bounds_of(k).map(|b| (b.max.y, format!("{:?} {:?} {:?}", f.tree().role_of(k), f.tree().name_of(k), b))))
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    rows.truncate(4);
    rows.into_iter().map(|r| r.1).collect::<Vec<_>>().join("\n  ")
}

#[trial]
fn diag_survey() {
    let mut f = Fixture::story(&story("compare_survey").story);
    f.settle();
    let root = f.root_id().and_then(|id| f.node_of(id)).expect("root");
    let mut out = format!("shown {:?} root {:?}\nlowest before:\n  {}\n", f.shown(), root.rect, lowest(&f, root.node));
    f.frame(vec![craft_ui::testing::pointer::moved(root.rect.center())]);
    for _ in 0..40 {
        f.frame(vec![craft_ui::testing::pointer::wheel(-120.0)]);
    }
    f.idle(2);
    out += &format!("lowest after:\n  {}\n", lowest(&f, root.node));
    panic!("DIAG\n{out}");
}

#[trial]
fn diag_rename() {
    let mut f = Fixture::story(&story("free_bin").story);
    f.settle();
    let rows = f.by_role(craft_ui::contract::Role::LIST_ITEM);
    let mut out = format!("items {:?}\n", rows.iter().map(|r| r.rect).collect::<Vec<_>>());
    f.click_at(rows[0].rect.center());
    f.idle(2);
    f.take_outputs();
    f.key(Key::F2, Modifiers::NONE);
    f.idle(2);
    let ev = |f: &mut Fixture| f.take_outputs().into_iter().flat_map(|o| o.collection).collect::<Vec<_>>();
    out += &format!("F2 {:?} focus {:?}\n", ev(&mut f), f.tree().role_of(focused_key(f.tree())));
    f.key(Key::Enter, Modifiers::NONE);
    f.idle(2);
    out += &format!("Enter {:?} focus {:?}\n", ev(&mut f), f.tree().role_of(focused_key(f.tree())));
    panic!("DIAG\n{out}");
}
