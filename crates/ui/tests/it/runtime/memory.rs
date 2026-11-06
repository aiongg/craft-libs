//! Memory lifetimes, eviction by owner and the runtime sub-id derivation.

use crate::support::Frames;
use craft_ui::runtime::egui::memory::put;
use craft_ui::runtime::memory::{Facts, MemoryStage, dispose_evicted, forget, forget_saved, global, sub};
use craft_ui::testing::trial;
use egui::Id;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

static CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
static DROPPED: AtomicU32 = AtomicU32::new(0);
static CALLED_CONTEXT: AtomicU32 = AtomicU32::new(0);

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
    let mut stage = MemoryStage::open(ui.ctx(), id);
    let _: &mut Probe = stage.shown(id);
    stage.write_back();
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
        let mut stage = MemoryStage::open(ui.ctx(), id);
        *stage.session::<u32>(id) = 7;
        stage.write_back();
    });
    frames.frame(vec![], |ui| {
        let mut sizing = MemoryStage::open_peek(ui.ctx(), id);
        assert_eq!(*sizing.session::<u32>(id), 7, "a sizing pass reads the kept value");
        *sizing.session::<u32>(id) = 99;
        sizing.write_back();
    });
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert_eq!(*stage.session::<u32>(id), 7, "the sizing pass wrote nothing and moved nothing");
        stage.write_back();
    });
}

/// The lifetimes of 7.2: shown and one-shot entries go with their owner; session and saved ones only through
/// `forget` and `forget_saved`; a press entry goes when its press ends.
#[trial]
fn lifetimes_follow_their_owner_until_forgotten() {
    let mut frames = Frames::new();
    let id = global("owner", ());
    let mut appeared = Vec::new();
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            let mut stage = MemoryStage::open(ui.ctx(), id);
            appeared.push(stage.appeared_at(id));
            *stage.shown::<u32>(id) += 1;
            *stage.session::<u32>(id) = 5;
            *stage.saved::<u32>(id) = 6;
            *stage.press::<u32>(id) = Some(8);
            stage.write_back();
        });
    }
    assert_eq!(appeared, vec![true, false], "appeared is true on the first pass only");
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert_eq!(*stage.shown::<u32>(id), 2);
        assert_eq!(*stage.press::<u32>(id), Some(8), "a press entry lives while its press does");
        *stage.press::<u32>(id) = None;
        stage.write_back();
    });
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert_eq!(*stage.press::<u32>(id), None, "an ended press leaves no entry");
        stage.write_back();
    });
    frames.idle(60, |_| {});
    dispose_evicted(&frames.ctx);
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert!(stage.appeared_at(id), "a re-shown component appears again");
        assert_eq!(*stage.shown::<u32>(id), 0, "the shown entry went with its owner");
        assert_eq!(*stage.session::<u32>(id), 5, "the session entry stayed");
        assert_eq!(*stage.saved::<u32>(id), 6, "the saved entry stayed");
        stage.write_back();
    });
    forget(&frames.ctx, id);
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert_eq!(*stage.session::<u32>(id), 0, "forget removes the session entry");
        assert_eq!(*stage.saved::<u32>(id), 6, "and not the saved one");
        stage.write_back();
    });
    forget_saved(&frames.ctx, id);
    frames.frame(vec![], |ui| {
        let mut stage = MemoryStage::open(ui.ctx(), id);
        assert_eq!(*stage.saved::<u32>(id), 0, "forget_saved removes the saved entry");
        stage.write_back();
    });
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
            read.push(Facts::<u32>::of_last_pass(ui.ctx()));
            Facts::register(ui.ctx(), n);
            Facts::register(ui.ctx(), n + 10);
        });
    }
    assert_eq!(read, vec![vec![], vec![0, 10], vec![1, 11]]);
    // A frame that read no facts skips a pass of the records: what pass 2 registered is not "last pass" later.
    frames.idle(1, |_| {});
    frames.frame(vec![], |ui| assert!(Facts::<u32>::of_last_pass(ui.ctx()).is_empty()));
}
