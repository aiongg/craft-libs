//! `SectionLayout`, moved from LightCraft's `sidebar.rs` tests.

use serde::{Deserialize, Serialize};

use super::{SectionEvent, SectionKey, SectionLayout};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    Albums,
    Local,
    ByDate,
    Folders,
    Keywords,
}

use Side::{Albums, ByDate, Folders, Keywords, Local};

impl SectionKey for Side {
    const ALL: &'static [Self] = &[Albums, Local, ByDate, Folders, Keywords];

    fn id(self) -> &'static str {
        match self {
            Albums => "albums",
            Local => "local",
            ByDate => "byDate",
            Folders => "folders",
            Keywords => "keywords",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.id() == id)
    }
}

type Layout = SectionLayout<Side>;

fn order(layout: &Layout) -> Vec<Side> {
    layout.states().iter().map(|s| s.section).collect()
}

fn read(saved: serde_json::Value) -> Layout {
    serde_json::from_value(saved).unwrap()
}

#[derive(Serialize, Deserialize)]
struct Settings {
    sidebar: Layout,
}

#[test]
fn a_new_sidebar_shows_every_section_open_in_the_usual_order() {
    let layout = Layout::default();
    assert_eq!(order(&layout), [Albums, Local, ByDate, Folders, Keywords]);
    assert!(layout.states().iter().all(|s| !s.hidden && !s.folded));
    assert!(layout.is_default());
}

#[test]
fn an_arrangement_survives_saving() {
    let mut layout = Layout::default();
    layout.move_before(Keywords, Some(Albums));
    layout.set_hidden(Local, true);
    layout.set_folded(ByDate, true);

    let saved = serde_json::to_value(&layout).unwrap();
    assert_eq!(saved[0], serde_json::json!({"id": "keywords", "hidden": false, "folded": false}));
    assert_eq!(read(saved), layout);

    let text = toml::to_string(&Settings { sidebar: layout.clone() }).unwrap();
    let back: Settings = toml::from_str(&text).unwrap();
    assert_eq!(back.sidebar, layout);
}

#[test]
fn a_damaged_arrangement_is_made_whole() {
    use serde_json::json;
    let layout = read(json!([
        {"id": "keywords", "collapsed": true},
        {"id": "map"},
        "folders",
        {"id": "keywords", "hidden": true},
        {"id": 7},
        null,
        {"id": "albums", "hidden": "yes", "folded": 1}
    ]));
    assert_eq!(order(&layout), [Keywords, Folders, Albums, Local, ByDate]);
    assert!(layout.is_folded(Keywords) && !layout.is_hidden(Keywords), "the first entry for a section counts; `collapsed` is read");
    assert!(!layout.is_hidden(Albums) && !layout.is_folded(Albums), "a flag that isn't true or false is off");

    let from_toml: Settings = toml::from_str("sidebar = [{ id = \"local\", hidden = true }, \"nope\", \"albums\"]").unwrap();
    assert_eq!(order(&from_toml.sidebar), [Local, Albums, ByDate, Folders, Keywords]);
    assert!(from_toml.sidebar.is_hidden(Local));
}

#[test]
fn an_unreadable_arrangement_is_the_usual_one() {
    use serde_json::json;
    for saved in [json!(null), json!("albums"), json!(3), json!({"albums": true}), json!([])] {
        assert!(read(saved.clone()).is_default(), "{saved}");
    }
}

#[test]
fn a_move_that_changes_nothing_shown_is_a_no_op() {
    let mut layout = Layout::default();
    layout.set_hidden(Local, true);
    let before = layout.clone();
    // Albums before ByDate would only step over the hidden Local: the shown order stays.
    assert!(!layout.move_before(Albums, Some(ByDate)));
    assert_eq!(layout, before);
    assert!(!layout.move_before(Albums, Some(Albums)));
    assert!(!layout.move_before(Keywords, None));
    assert_eq!(layout, before);
    assert!(layout.move_before(Albums, Some(Folders)));
    assert_eq!(order(&layout), [Local, ByDate, Albums, Folders, Keywords]);
}

#[test]
fn move_by_steps_over_hidden_sections() {
    let mut layout = Layout::default();
    layout.set_hidden(Local, true);
    assert!(layout.can_move(Albums, false) && !layout.can_move(Albums, true));
    assert!(layout.can_move(Keywords, true) && !layout.can_move(Keywords, false));
    assert!(!layout.can_move(Local, true), "a hidden section does not move");

    assert!(layout.move_by(Albums, false, false));
    assert_eq!(order(&layout), [Local, ByDate, Albums, Folders, Keywords], "one shown place, over the hidden one");
    assert!(layout.move_by(Albums, true, false));
    assert_eq!(layout.shown().collect::<Vec<_>>(), [Albums, ByDate, Folders, Keywords]);
    assert!(!layout.move_by(Albums, true, false));

    assert!(layout.move_by(Albums, false, true));
    assert_eq!(layout.shown().last(), Some(Albums));
    assert!(!layout.move_by(Albums, false, true));
    assert!(layout.move_by(Albums, true, true));
    assert_eq!(layout.shown().next(), Some(Albums));
}

#[test]
fn applying_events_changes_the_layout() {
    let mut layout = Layout::default();
    assert!(layout.apply(&SectionEvent::Moved { section: Keywords, before: Some(Albums) }));
    assert_eq!(layout.states()[0].section, Keywords);
    assert!(layout.apply(&SectionEvent::Hidden(Local)) && layout.is_hidden(Local));
    assert!(!layout.apply(&SectionEvent::Hidden(Local)), "already hidden");
    assert!(layout.apply(&SectionEvent::Shown(Local)) && !layout.is_hidden(Local));
    assert!(layout.apply(&SectionEvent::Folded { section: ByDate, folded: true }) && layout.is_folded(ByDate));
    assert!(!layout.apply(&SectionEvent::Button { section: Albums, id: "albumNew" }));
    assert!(layout.apply(&SectionEvent::Reset));
    assert!(layout.is_default());
    assert!(!layout.apply(&SectionEvent::Reset));
}
