//! `SectionLayout`: reading a saved arrangement leniently, reconciling it with the registered sections, and the
//! rules for hiding, pinning and moving sections.

use craft_ui::sections::{SectionEvent, SectionKey, SectionLayout};
use craft_ui::testing::trial;
use serde::{Deserialize, Serialize};

const IDS: [&str; 5] = ["albums", "local", "byDate", "folders", "keywords"];

fn key(id: &str) -> SectionKey {
    SectionKey::new(id)
}

fn registered() -> Vec<SectionKey> {
    IDS.iter().map(|id| key(id)).collect()
}

fn usual() -> SectionLayout {
    SectionLayout::new(registered())
}

fn order(layout: &SectionLayout) -> Vec<&str> {
    layout.states().iter().map(|state| state.key.as_str()).collect()
}

fn shown(layout: &SectionLayout) -> Vec<&str> {
    layout.shown().map(SectionKey::as_str).collect()
}

#[derive(Serialize, Deserialize)]
struct Settings {
    sidebar: SectionLayout,
}

fn read(text: &str) -> SectionLayout {
    let mut layout = toml::from_str::<Settings>(text).expect("a saved arrangement always reads").sidebar;
    layout.reconcile(&registered());
    layout
}

#[trial]
fn an_arrangement_survives_saving() {
    let mut layout = usual();
    layout.move_before(&key("keywords"), Some(&key("albums")));
    layout.set_hidden(&key("local"), true);
    layout.set_folded(&key("byDate"), true);
    layout.set_pinned(&key("folders"), true);

    let saved = toml::Value::try_from(Settings { sidebar: layout.clone() }).expect("an arrangement serialises");
    let back: Settings = saved.try_into().expect("a saved arrangement reads back");
    assert_eq!(back.sidebar, layout);
}

/// A saved arrangement is read leniently: a damaged one is made whole (unknown sections dropped, missing ones added,
/// the first entry for a section counting) and an unreadable one is the usual arrangement.
#[trial]
fn a_damaged_arrangement_is_made_whole() {
    let layout = read(
        "sidebar = [{ id = \"keywords\", collapsed = true }, { id = \"map\" }, \"folders\", { id = \"keywords\", hidden = true }, \
         { id = 7 }, { id = \"albums\", hidden = \"yes\", folded = 1 }]",
    );
    assert_eq!(
        order(&layout),
        ["keywords", "folders", "albums", "local", "byDate"],
        "unknown ids are dropped, missing ones follow in registration order"
    );
    assert!(layout.is_folded(&key("keywords")) && !layout.is_hidden(&key("keywords")), "the first entry for a section counts; `collapsed` is read");

    for saved in ["sidebar = []", "sidebar = \"albums\"", "sidebar = 3", "sidebar = { albums = true }"] {
        assert_eq!(read(saved), usual(), "{saved}");
    }
}

#[trial]
fn reconciling_drops_unknown_sections_and_reports_a_change_only_when_there_is_one() {
    let mut layout = usual();
    assert!(!layout.reconcile(&registered()), "a whole layout is left as it is");
    let fewer = registered()[..3].to_vec();
    assert!(layout.reconcile(&fewer));
    assert_eq!(order(&layout), ["albums", "local", "byDate"]);
}

/// Moving by a place steps over hidden sections, and a move that changes nothing shown is a no-op.
#[trial]
fn moving_by_a_place_steps_over_hidden_sections() {
    let mut layout = usual();
    layout.set_hidden(&key("local"), true);
    assert!(layout.can_move(&key("albums"), false) && !layout.can_move(&key("albums"), true));
    assert!(!layout.can_move(&key("local"), true), "a hidden section does not move");

    let place = |layout: &SectionLayout| shown(layout).iter().position(|id| *id == "albums");
    let first = place(&layout);
    assert!(layout.move_by(&key("albums"), false, false));
    assert_eq!(place(&layout), first.map(|at| at + 1), "one shown place, over the hidden one");
    assert!(layout.move_by(&key("albums"), false, true));
    assert_eq!(shown(&layout).last(), Some(&"albums"));
    assert!(!layout.move_by(&key("albums"), false, true));
    assert!(layout.move_by(&key("albums"), true, true));
    assert_eq!(shown(&layout).first(), Some(&"albums"));

    let mut layout = usual();
    layout.set_hidden(&key("local"), true);
    let before = layout.clone();
    // Albums before ByDate only steps over the hidden Local: the shown order stays.
    assert!(!layout.move_before(&key("albums"), Some(&key("byDate"))));
    assert!(!layout.move_before(&key("albums"), Some(&key("albums"))));
    assert!(!layout.move_before(&key("keywords"), None));
    assert_eq!(layout, before, "nothing changed");
}

#[trial]
fn pinned_sections_stay_above_the_others() {
    let mut layout = usual();
    layout.set_pinned(&key("folders"), true);
    layout.set_pinned(&key("local"), true);
    assert_eq!(order(&layout), ["folders", "local", "albums", "byDate", "keywords"], "pinned sections keep their relative order");

    assert!(!layout.move_before(&key("albums"), Some(&key("local"))), "an unpinned section does not cross into the pinned ones");
    assert!(layout.move_to(&key("local"), 0));
    assert_eq!(order(&layout), ["local", "folders", "albums", "byDate", "keywords"]);
    assert!(layout.move_to(&key("local"), 4));
    assert_eq!(
        order(&layout),
        ["folders", "local", "albums", "byDate", "keywords"],
        "the last place a pinned section can take is the last pinned one"
    );

    layout.set_pinned(&key("folders"), false);
    assert_eq!(order(&layout), ["local", "folders", "albums", "byDate", "keywords"], "a released section sits after the pinned ones");
}

/// An event that sets what is already set reports no change.
#[trial]
fn applying_an_event_that_sets_what_is_set_reports_no_change() {
    let mut layout = usual();
    assert!(layout.apply(&SectionEvent::Hidden(key("local"))) && layout.is_hidden(&key("local")));
    assert!(!layout.apply(&SectionEvent::Hidden(key("local"))), "already hidden");
}
