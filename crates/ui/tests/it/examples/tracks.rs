//! `TrackStack`: a clip is a part of its row's lane, reachable by pointer and by AccessKit, and reports one
//! selection per activation.

use craft_ui::contract::{CollectionEvent, Props as _, ShowCollection as _};
use craft_ui::testing::examples::tracks::{Clip, Track, TrackEvent, TrackRow, TrackStack, Tracks};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn rig() -> (Fixture, Rc<RefCell<Vec<u64>>>) {
    let selected = Rc::new(RefCell::new(Vec::new()));
    let shown = selected.clone();
    let tracks = vec![
        Track::new(0, "Video").clips(vec![Clip::new(1, 1.0, 3.0), Clip::new(2, 5.0, 2.0)]),
        Track::new(1, "Audio").clips(vec![Clip::new(3, 0.0, 6.0)]),
    ];
    let mut fixture = Fixture::new(move |ui| {
        let out = TrackStack::new("Tracks")
            .id_salt("stack")
            .show_items(ui, &Tracks(&tracks), &mut |key, icx| TrackRow::new(&tracks[*key], icx.cells().to_vec(), 10.0));
        for event in out.events {
            if let CollectionEvent::Item(_, TrackEvent::Selected(clip)) = event {
                shown.borrow_mut().push(clip);
            }
        }
    });
    fixture.settle();
    (fixture, selected)
}

#[trial]
fn a_click_on_a_clip_selects_that_clip_once() {
    let (mut fixture, selected) = rig();
    let clip = fixture.by_label("Audio, clip 3").expect("the clip has a node");
    fixture.click(&clip);
    assert_eq!(*selected.borrow(), [3]);
}

#[trial]
fn an_accesskit_click_on_a_clip_selects_it() {
    let (mut fixture, selected) = rig();
    let clip = fixture.by_label("Video, clip 2").expect("the clip has a node");
    fixture.access(craft_ui::testing::focus::AccessActionKind::Click, &clip);
    assert_eq!(*selected.borrow(), [2]);
}
