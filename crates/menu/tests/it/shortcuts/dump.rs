use craft_ui::testing::trial;

use super::fixture::Rig;

fn names(rig: &Rig) -> String {
    let tree = rig.fixture.tree();
    let mut all: Vec<String> = tree.keys().into_iter().filter_map(|key| tree.name_of(key).map(|name| format!("{:?}:{name}", tree.role_of(key)))).collect();
    all.sort();
    all.join(" | ")
}

#[trial]
fn zz_dump() {
    let mut rig = Rig::mac();
    let first = names(&rig);
    rig.click("Show:");
    let second = names(&rig);
    panic!("FIRST {first} SECOND {second} END");
}
