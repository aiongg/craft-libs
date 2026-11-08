use craft_ui::testing::{Fixture, STORIES, trial};
use egui::{Key, Modifiers};

fn dump(f: &Fixture, node: craft_ui::testing::access::NodeKey, depth: usize, out: &mut String) {
    let t = f.tree();
    out.push_str(&format!("{}{:?} {:?} {:?}\n", "  ".repeat(depth), t.role_of(node), t.name_of(node), t.bounds_of(node)));
    for c in t.children_of(node) {
        if depth < 6 {
            dump(f, c, depth + 1, out);
        }
    }
}

fn story(name: &str) -> &'static craft_ui::testing::StoryRegistration {
    STORIES.iter().find(|e| e.crate_name == "craft-browse" && e.story.name == name).expect("story")
}

#[trial]
fn diag_survey() {
    let mut f = Fixture::story(&story("compare_survey").story);
    f.settle();
    let root = f.root_id().and_then(|id| f.node_of(id)).expect("root");
    let mut out = format!("shown {:?} root {:?}\n", f.shown(), root.rect);
    dump(&f, root.node, 0, &mut out);
    f.frame(vec![craft_ui::testing::pointer::moved(root.rect.center())]);
    for _ in 0..40 {
        f.frame(vec![craft_ui::testing::pointer::wheel(-120.0)]);
    }
    f.idle(2);
    out.push_str("--- after\n");
    dump(&f, root.node, 0, &mut out);
    panic!("DIAG\n{out}");
}

#[trial]
fn diag_rename() {
    let mut f = Fixture::story(&story("free_bin").story);
    f.settle();
    let root = f.root_id().and_then(|id| f.node_of(id)).expect("root");
    let mut out = String::new();
    dump(&f, root.node, 0, &mut out);
    let rows = f.by_role(craft_ui::contract::Role::LIST_ITEM);
    out.push_str(&format!("items {:?}\n", rows.iter().map(|r| r.rect).collect::<Vec<_>>()));
    f.click_at(rows[0].rect.center());
    f.idle(2);
    f.take_outputs();
    f.key(Key::F2, Modifiers::NONE);
    f.idle(2);
    out.push_str(&format!("F2 events {:?}\nfocus role {:?}\n", f.take_outputs().into_iter().flat_map(|o| o.collection).collect::<Vec<_>>(), f.tree().role_of(f.tree().focused_key())));
    dump(&f, root.node, 0, &mut out);
    f.key(Key::Enter, Modifiers::NONE);
    f.idle(2);
    out.push_str(&format!("Enter events {:?}\nfocus role {:?}\n", f.take_outputs().into_iter().flat_map(|o| o.collection).collect::<Vec<_>>(), f.tree().role_of(f.tree().focused_key())));
    panic!("DIAG\n{out}");
}
