//! Memory lifetimes, eviction by owner and the runtime sub-id derivation.

use craft_ui::runtime::memory::{Remember, SaveKey};
use craft_ui::testing::memory::{Stage, dispose_evicted, facts_of_last_pass, forget, forget_saved, global, put, register_fact, sub};
use craft_ui::testing::{Frames, trial};
use egui::Id;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

static CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
static DROPPED: AtomicU32 = AtomicU32::new(0);
static CALLED_CONTEXT: AtomicU32 = AtomicU32::new(0);
const PLACE: SaveKey = SaveKey::new("craft_tests.place");

/// A slot whose `Drop` calls a `Context` API: it deadlocks when it runs under an egui lock. Copies made for
/// reads (`Clone`) are marked and do not count.
#[derive(Default)]
struct Probe {
    copy: bool,
}

impl Clone for Probe {
    fn clone(&self) -> Self {
        Probe { copy: true }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        if self.copy {
            return;
        }
        DROPPED.fetch_add(1, Ordering::SeqCst);
        if let Some(ctx) = CTX.lock().expect("probe context lock").as_ref() {
            put(ctx, global("probe", ()), 1u8);
            let _ = ctx.input(|i| i.time);
            CALLED_CONTEXT.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn shown_stage(ui: &egui::Ui, id: Id) {
    let mut stage = Stage::open(ui.ctx(), id);
    let _: &mut Probe = stage.memory();
    stage.finish();
}

/// Skeleton: `frames_state::evicted_slot_drops_once_outside_any_egui_lock`.
#[trial]
fn evicted_slot_drops_once_outside_any_egui_lock() {
    let mut frames = Frames::new();
    *CTX.lock().expect("probe context lock") = Some(frames.ctx.clone());
    let holder = global("holder", ());
    for _ in 0..3 {
        frames.frame(vec![], |ui| shown_stage(ui, holder));
    }
    assert_eq!(DROPPED.load(Ordering::SeqCst), 0, "a shown slot is not evicted");
    // The holder is no longer shown; 3 passes and 0.5 s later its slot is queued, and the first pipeline
    // run of the next pass drops it.
    frames.idle(60, |_| {});
    dispose_evicted(&frames.ctx);
    *CTX.lock().expect("probe context lock") = None;
    assert_eq!(DROPPED.load(Ordering::SeqCst), 1, "the evicted slot's Drop ran exactly once");
    assert_eq!(CALLED_CONTEXT.load(Ordering::SeqCst), 1, "and could call a Context API");
}

/// Skeleton: `frames_state::kept_value_survives_the_first_appearance_in_a_new_layer`, at the memory stage.
#[trial]
fn sizing_peek_moves_nothing_and_keeps_the_kept_value() {
    let mut frames = Frames::new();
    let id = global("floaty", ());
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        *stage.keep::<u32>() = 7;
        stage.finish();
    });
    frames.frame(vec![], |ui| {
        let mut sizing = Stage::open_peek(ui.ctx(), id);
        assert_eq!(*sizing.keep::<u32>(), 7, "a sizing pass reads the kept value");
        *sizing.keep::<u32>() = 99;
        sizing.finish();
    });
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.keep::<u32>(), 7, "the sizing pass wrote nothing and moved nothing");
        stage.finish();
    });
}

/// The lifetimes of 7.2, through `Remember`: shown entries go with their owner; session and saved ones only
/// through `forget` and `forget_saved`; a press entry goes when its press ends.
#[trial]
fn lifetimes_follow_their_owner_until_forgotten() {
    let mut frames = Frames::new();
    let id = global("owner", ());
    let mut appeared = Vec::new();
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            let mut stage = Stage::open(ui.ctx(), id);
            appeared.push(stage.appeared());
            *stage.memory::<u32>() += 1;
            *stage.memory_at::<u32>("part") += 10;
            *stage.keep::<u32>() = 5;
            *stage.saved::<u32>(&PLACE) = 6;
            *stage.press_slot::<u32>() = Some(8);
            stage.finish();
        });
    }
    assert_eq!(appeared, vec![true, false], "appeared is true on the first pass only");
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.memory::<u32>(), 2);
        assert_eq!(*stage.memory_at::<u32>("part"), 20, "memory_at keeps its own slot");
        assert_eq!(*stage.press_slot::<u32>(), Some(8), "a press entry lives while its press does");
        *stage.press_slot::<u32>() = None;
        stage.finish();
    });
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.press_slot::<u32>(), None, "an ended press leaves no entry");
        stage.finish();
    });
    frames.idle(60, |_| {});
    dispose_evicted(&frames.ctx);
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert!(stage.appeared(), "a re-shown component appears again");
        assert_eq!(*stage.memory::<u32>(), 0, "the shown entry went with its owner");
        assert_eq!(*stage.memory_at::<u32>("part"), 0, "and the memory_at entry");
        assert_eq!(*stage.keep::<u32>(), 5, "the session entry stayed");
        assert_eq!(*stage.saved::<u32>(&PLACE), 6, "the saved entry stayed");
        stage.finish();
    });
    forget(&frames.ctx, id);
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.keep::<u32>(), 0, "forget removes the session entry");
        assert_eq!(*stage.saved::<u32>(&PLACE), 6, "and not the saved one");
        stage.finish();
    });
    forget_saved(&frames.ctx, &PLACE);
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.saved::<u32>(&PLACE), 0, "forget_saved removes the saved entry written through Remember::saved");
        stage.finish();
    });
}

/// 7.2 `take`: the shown slot is read once.
#[trial]
fn take_moves_the_shown_slot_out() {
    let mut frames = Frames::new();
    let id = global("taken", ());
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(stage.take::<u32>(), None, "nothing written yet");
        *stage.memory::<u32>() = 9;
        stage.finish();
    });
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(stage.take::<u32>(), Some(9));
        stage.finish();
    });
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(stage.take::<u32>(), None, "read once");
        stage.finish();
    });
}

/// Entries written through a stage whose base differs from its owner go with the owner: they stay while the
/// owner registers and are evicted with it.
#[trial]
fn entries_of_a_stage_follow_its_owner_not_its_base() {
    let mut frames = Frames::new();
    let (owner, base) = (global("owner", ()), global("popup body", ()));
    frames.frame(vec![], |ui| {
        let mut own = Stage::open(ui.ctx(), owner);
        *own.memory::<u32>() = 1;
        own.finish();
        let mut body = Stage::open_owned(ui.ctx(), base, owner);
        *body.memory::<u32>() = 7;
        body.finish();
    });
    frames.idle(60, |ui| {
        let own = Stage::open(ui.ctx(), owner);
        own.finish();
    });
    frames.frame(vec![], |ui| {
        let mut body = Stage::open_owned(ui.ctx(), base, owner);
        assert_eq!(*body.memory::<u32>(), 7, "the body's entry outlives 60 frames in which only its owner registered");
        body.finish();
    });
    frames.idle(60, |_| {});
    dispose_evicted(&frames.ctx);
    frames.frame(vec![], |ui| {
        let mut body = Stage::open_owned(ui.ctx(), base, owner);
        assert_eq!(*body.memory::<u32>(), 0, "and it went with its owner");
        body.finish();
    });
}

/// A press entry whose press never ended goes with its dead owner, together with its shown entries.
#[trial]
fn a_press_entry_of_a_dead_owner_is_evicted() {
    let mut frames = Frames::new();
    let id = global("pressed", ());
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        *stage.press_slot::<u32>() = Some(3);
        stage.finish();
    });
    frames.idle(60, |_| {});
    dispose_evicted(&frames.ctx);
    frames.frame(vec![], |ui| {
        let mut stage = Stage::open(ui.ctx(), id);
        assert_eq!(*stage.press_slot::<u32>(), None);
        stage.finish();
    });
}

/// `appeared` is true through both passes of the frame a component first shows in, so a discard pass does not
/// lose the first-appearance work.
#[trial]
fn appeared_holds_through_the_discard_pass_of_the_first_frame() {
    let mut frames = Frames::new();
    let id = global("appearing", ());
    let mut seen = Vec::new();
    let mut first = true;
    frames.frame(vec![], |ui| {
        let stage = Stage::open(ui.ctx(), id);
        seen.push(stage.appeared());
        stage.finish();
        if std::mem::take(&mut first) {
            ui.ctx().request_discard("test");
        }
    });
    frames.frame(vec![], |ui| {
        let stage = Stage::open(ui.ctx(), id);
        seen.push(stage.appeared());
        stage.finish();
    });
    assert_eq!(seen, vec![true, true, false]);
}

/// A saved key names its crate: a prefix, a dot and a name.
#[trial]
fn a_save_key_needs_a_crate_prefix() {
    for bad in [".place", "place", "craft_tests.", "Craft.place", "craft-tests.place"] {
        assert!(std::panic::catch_unwind(|| SaveKey::new(bad)).is_err(), "{bad:?} is no save key");
        assert!(std::panic::catch_unwind(|| SaveKey::named(bad)).is_err(), "{bad:?} is no save key");
    }
    assert_eq!(SaveKey::new("craft_dialogs.export.place").as_str(), "craft_dialogs.export.place");
}

/// A key built at run time, one for each open document, names the slots of the same key written out.
#[trial]
fn a_save_key_built_at_run_time_names_the_same_slots() {
    let document = 7;
    assert_eq!(SaveKey::named(format!("craft_tests.window.{document}")), SaveKey::new("craft_tests.window.7"));
}

/// Skeleton: `collide::children_and_items_keyed_like_runtime_regions_keep_their_own_ids`.
#[trial]
fn runtime_sub_ids_never_equal_an_author_key() {
    let component = global("component", ());
    for region in ["body", "children", "header", "footer", "layer", "edit"] {
        assert_ne!(sub(component, region), component.with(region), "{region}: a runtime region differs from an author key");
    }
    assert_ne!(sub(component, ("popup", "menu")), component.with(("popup", "menu")));
    let list = global("list", ());
    assert_ne!(sub(list, ("key", "children")), sub(list, "children"), "an item keyed like a region keeps its own id");
    assert_ne!(sub(list, ("mem", "a")), sub(list, ("mem", "b")));
}

/// `Facts`: what one pass registers, the next pass reads, and a pass the viewport skipped leaves nothing stale.
#[trial]
fn facts_registered_in_one_pass_are_read_in_the_next() {
    let mut frames = Frames::new();
    let mut read = Vec::new();
    for n in 0..3u32 {
        frames.frame(vec![], |ui| {
            read.push(facts_of_last_pass::<u32>(ui.ctx()));
            register_fact(ui.ctx(), n);
            register_fact(ui.ctx(), n + 10);
        });
    }
    assert_eq!(read, vec![vec![], vec![0, 10], vec![1, 11]]);
    // A frame that read no facts skips a pass of the records: what pass 2 registered is not "last pass" later.
    frames.idle(1, |_| {});
    frames.frame(vec![], |ui| assert!(facts_of_last_pass::<u32>(ui.ctx()).is_empty()));
}
