use craft_ui::testing::trial;

use super::fixture::Rig;

fn names(rig: &Rig) -> String {
    let tree = rig.fixture.tree();
    let mut out = String::new();
    for key in tree.keys() {
        if let Some(name) = tree.name_of(key) {
            out.push_str(&format!("{:?} {:?} {:?}\n", tree.role_of(key), name, tree.bounds_of(key).map(|r| (r.min.x as i32, r.min.y as i32, r.width() as i32, r.height() as i32))));
        }
    }
    out
}

#[trial]
fn zz_dump() {
    let mut rig = Rig::mac();
    let first = names(&rig);
    rig.click("Show:");
    let second = names(&rig);
    panic!("FIRST\n{first}\nSECOND\n{second}");
}
