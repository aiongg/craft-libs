//! Count trial of session memory on kept nodes: a component's session slot (`Remember::keep`) is written back to the
//! runtime's store at the end of each run, so the same component shown at another place (a panel moved to another
//! window or parent: a new node, the same id) starts from the value the old node left.

use std::cell::Cell;

use craft_ui_contract::contract::*;
use craft_ui_contract::runtime::memory::Remember;
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_contract::style::StyleCx;
use craft_ui_rt::Vec2;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::Stack;

thread_local! {
    /// The session value the counter found at its latest run.
    static SEEN: Cell<u32> = const { Cell::new(0) };
}

/// Counts its runs in its session slot.
#[derive(Component)]
pub struct Counter {
    #[common]
    common: Common,
}

impl Component for Counter {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: CounterStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(10.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let seen = *cx.keep::<u32>();
        SEEN.with(|s| s.set(seen));
        *cx.keep::<u32>() = seen + 1;
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_session_slot_outlives_its_node_and_reaches_the_node_that_shows_the_same_id_elsewhere() {
    let id = Id::new("moving-counter");
    let mut rig = Rig::inline(Vec2::new(200.0, 100.0), 1.0, RecordingRenderer::default());
    let mut place = 0_u32;
    let step = |rig: &mut Rig<RecordingRenderer>, place: u32| {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                Stack::new().id_salt(("place", place)).show(ui, |ui| {
                    Counter { common: Common::new() }.id(id).show(ui);
                });
            })
        })
    };
    step(&mut rig, place);
    assert_eq!(SEEN.with(Cell::get), 0, "the first run finds an empty slot");
    place = 1;
    let main = rig.main;
    rig.driver.runtime.mark_root(main);
    step(&mut rig, place);
    assert_eq!(SEEN.with(Cell::get), 1, "the node at the new place starts from what the old node wrote back");
}
