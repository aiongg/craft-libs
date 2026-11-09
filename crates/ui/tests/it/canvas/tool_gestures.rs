//! A tool's one-shot `Set` events belong to no gesture: each is its own undo step, whatever gesture ran before.

use craft_behavior::{DocKey, EditEvent, GestureId, Mixed};
use craft_ui::edit::{GestureHost, StepName, UndoMarks};
use craft_ui::testing::trial;

#[derive(Default)]
struct Doc {
    value: i32,
    steps: Vec<(i32, String)>,
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.steps.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some((value, _)) = self.steps.get(mark) {
            self.value = *value;
        }
        self.steps.truncate(mark);
    }

    fn squash_since(&mut self, mark: usize, name: &StepName) {
        if self.steps.len() > mark {
            self.steps.truncate(mark + 1);
            self.steps[mark].1 = name.label.to_string();
        }
    }
}

fn apply(doc: &mut Doc, value: i32) {
    doc.steps.push((doc.value, String::new()));
    doc.value = value;
}

#[trial]
fn sets_and_gestures_are_each_one_undo_step() {
    {
        let mut doc = Doc::default();
        let mut host = GestureHost::new();
        let set = |before, after| EditEvent::set(Mixed::Same(before), after, "Nudge".into(), Some(DocKey::new("trial.nudge")));
        host.handle(&mut doc, set(0, 1), apply);
        host.handle(&mut doc, set(1, 2), apply);
        assert_eq!(doc.value, 2);
        assert_eq!(doc.steps.len(), 2, "each Set is one step: {:?}", doc.steps);
    }
    {
        let mut doc = Doc::default();
        let mut host = GestureHost::new();
        let id = GestureId::new(1);
        host.handle(&mut doc, EditEvent::begin(id, Mixed::Same(0), "Drag".into(), None), apply);
        host.handle(&mut doc, EditEvent::update(id, 5), apply);
        host.handle(&mut doc, EditEvent::commit(id, Mixed::Same(0), 5), apply);
        host.handle(&mut doc, EditEvent::set(Mixed::Same(5), 6, "Nudge".into(), None), apply);
        let names: Vec<&str> = doc.steps.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(names, ["Drag", "Nudge"], "the gesture and the Set are two steps");
    }
}
