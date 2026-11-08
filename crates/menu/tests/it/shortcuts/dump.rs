//! Temporary diagnostics.

use craft_keymap::Platform;
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture::{self, Rig};

fn names(rig: &Rig) -> String {
    let tree = rig.fixture.tree();
    let mut all: Vec<String> = tree
        .keys()
        .into_iter()
        .filter_map(|key| {
            let name = tree.name_of(key)?;
            let parent = tree.parent_of_key(key).and_then(|p| tree.role_of(p));
            let lb = tree.labelled_by_key(key).len();
            let r = tree.bounds_of(key).map(|r| (r.min.x as i32, r.min.y as i32, r.width() as i32, r.height() as i32));
            Some(format!("{:?}<{:?} lb{lb} {r:?}:{name}", tree.role_of(key), parent))
        })
        .collect();
    all.sort();
    format!("focus={:?}\n{}", rig.fixture.focused(), all.join("\n"))
}

#[trial]
fn zz_dump_a() {
    let mut rig = Rig::mac();
    let first = names(&rig);
    rig.click("Show:");
    let second = names(&rig);
    panic!("A1\n{first}\nA2 after click Show:\n{second}\nEND");
}

#[trial]
fn zz_dump_b() {
    let rig = Rig::mac_with_sets(craft_keymap::Profiles::default());
    panic!("B sets\n{}\nEND", names(&rig));
}

#[trial]
fn zz_dump_c() {
    let mut rig = Rig::mac();
    rig.click("Save");
    let a = names(&rig);
    rig.press(Modifiers::NONE, Key::ArrowDown);
    let b = names(&rig);
    panic!("C after click Save\n{a}\nC2 after Down\n{b}\nEND");
}

#[trial]
fn zz_dump_d() {
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), fixture::options(), None);
    rig.click("Bring to Front");
    let a = names(&rig);
    let rec = rig.recording();
    rig.press(Modifiers::NONE, Key::F8);
    let b = names(&rig);
    panic!("D after click chip rec={rec}\n{a}\nD2 after F8 rec={} bind={:?}\n{b}\nEND", rig.recording(), rig.bindings("object.front"));
}
