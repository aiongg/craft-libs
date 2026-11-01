//! The render queue list on screen (egui_kittest).

use std::time::Duration;

use craft_ui::behavior::Selection;
use craft_ui::testing::symbols;
use craft_ui::theme::{Base, install};
use egui::Ui;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::render_queue::{RenderItemId, RenderQueueEvent, RenderQueueItem, RenderQueueList, RenderStatus};

struct App {
    items: Vec<RenderQueueItem>,
    selection: Selection<RenderItemId>,
    running: bool,
    events: Vec<RenderQueueEvent>,
}

fn item(id: u64, status: RenderStatus) -> RenderQueueItem {
    RenderQueueItem {
        id: RenderItemId(id),
        source: "Sequence 01".to_owned(),
        file: format!("out{id}.mp4"),
        preset: "Match Source".to_owned(),
        status,
    }
}

fn show(ui: &mut Ui, app: &mut App) {
    let output = RenderQueueList::new("queue", &app.items, &app.selection, app.running).show(ui);
    app.events.extend(output.events);
}

fn harness(items: Vec<RenderQueueItem>, running: bool) -> Harness<'static, App> {
    let app = App { items, selection: Selection::new(), running, events: Vec::new() };
    let mut harness = Harness::<App>::builder()
        .with_size([480.0, 360.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn enabled(harness: &Harness<'_, App>, label: &str, at: usize) -> bool {
    let nodes: Vec<_> = harness.query_all_by_role_and_label(Role::Button, label).collect();
    !nodes[at].accesskit_node().is_disabled()
}

fn count(harness: &Harness<'_, App>, label: &str) -> usize {
    harness.query_all_by_role_and_label(Role::Button, label).count()
}

// Cancel for Ready and Encoding, Retry otherwise, Remove disabled while encoding.
#[test]
fn row_buttons_follow_the_status() {
    let encoding = RenderStatus::Encoding { progress: 0.4, remaining: Some(Duration::from_secs(75)) };
    let items =
        vec![item(1, RenderStatus::Ready), item(2, encoding), item(3, RenderStatus::Failed("disk full".to_owned())), item(4, RenderStatus::Done)];
    let mut harness = harness(items, true);
    assert_eq!(count(&harness, "Cancel"), 2);
    assert_eq!(count(&harness, "Retry"), 2);
    assert_eq!(count(&harness, "Remove"), 4);
    assert!(enabled(&harness, "Remove", 0));
    assert!(!enabled(&harness, "Remove", 1), "the encoding row cannot be removed");
    assert!(enabled(&harness, "Remove", 2));
    harness.query_all_by_role_and_label(Role::Button, "Retry").next().unwrap().click();
    harness.run();
    assert_eq!(harness.state().events, vec![RenderQueueEvent::Retry(RenderItemId(3))]);
}

// Start Queue with Ready items when stopped, Stop while running, Clear Finished with finished items.
#[test]
fn footer_buttons_follow_the_queue() {
    let stopped = harness(vec![item(1, RenderStatus::Ready)], false);
    assert!(enabled(&stopped, "Start Queue", 0));
    assert!(!enabled(&stopped, "Stop", 0));
    assert!(!enabled(&stopped, "Clear Finished", 0));

    let mut running = harness(vec![item(1, RenderStatus::Ready), item(2, RenderStatus::Done)], true);
    assert!(!enabled(&running, "Start Queue", 0));
    assert!(enabled(&running, "Stop", 0));
    assert!(enabled(&running, "Clear Finished", 0));
    running.get_by_role_and_label(Role::Button, "Clear Finished").click();
    running.run();
    assert_eq!(running.state().events, vec![RenderQueueEvent::ClearFinished]);
}
