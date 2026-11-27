//! Trials of the same-frame rerun of `Tracked` readers (owner rule: a value edit reruns the pass in the same frame only
//! when something earlier in that frame read the edited value, as the framework records reads): an app that applies a
//! panel's events after the panel ran shows the edit in that frame, and a field's own bound write reruns nothing.
//! Relations only, no times.

use crate::skeleton::IconButton;
use crate::text_field::{TextField, edit_harness_from};
use craft_behavior::IconId;
use craft_ui_contract::contract::{MemoKey, MemoProps, MemoView, Outbox, Tracked, Ui};
use craft_ui_contract::testing::Harness;
use craft_ui_rt::{Key, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A run counter and the last value a view showed; feeds no key.
#[derive(Clone, Default)]
struct Seen(Rc<Cell<u32>>);

impl MemoProps for Seen {
    fn key(&self, _: &mut MemoKey) {}
}

/// The app's view: it shows the panel, then applies the panel's events to the model, as the Layers glue does.
struct Body<'a> {
    count: &'a mut Tracked<u32>,
    panel_runs: Seen,
    shown: Seen,
}

impl MemoProps for Body<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.count.key(key);
    }
}

/// The panel: it reads the model in its own run and reports a click on its button.
struct Panel<'a> {
    count: &'a Tracked<u32>,
    runs: Seen,
    shown: Seen,
    added: Outbox<'a, ()>,
}

impl MemoProps for Panel<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.count.key(key);
    }
}

fn panel(props: &mut Panel<'_>, ui: &mut Ui) {
    props.runs.0.set(props.runs.0.get() + 1);
    props.shown.0.set(**props.count);
    if IconButton::new(IconId::new("plus"), "Add").show(ui).clicked() {
        props.added.send(());
    }
}

fn body(props: &mut Body<'_>, ui: &mut Ui) {
    let mut added = Vec::new();
    let shown = Panel { count: &*props.count, runs: props.panel_runs.clone(), shown: props.shown.clone(), added: Outbox::new(&mut added) };
    let _ = MemoView::new("panel", shown, panel).show(ui);
    if !added.is_empty() {
        props.count.edit(|count| *count += 1);
    }
}

#[test]
fn an_edit_after_the_panel_ran_reruns_the_panel_once_in_the_same_frame() {
    let (panel_runs, shown) = (Seen::default(), Seen::default());
    let (runs, last) = (panel_runs.clone(), shown.clone());
    let mut h = Harness::builder().with_size(Vec2::new(300.0, 200.0)).build_ui_state(
        move |ui: &mut Ui<'_, '_>, count: &mut Tracked<u32>| {
            let _ = MemoView::new("body", Body { count, panel_runs: runs.clone(), shown: last.clone() }, body).show(ui);
        },
        Tracked::new(0),
    );
    h.run();
    let add = h.get_by_label("Add");
    h.hover(&add).run();
    h.press(&add).run();
    let before = panel_runs.0.get();
    h.release(&add);
    let release = h.last_report();
    assert_eq!(**h.state(), 1, "the click reached the model");
    assert_eq!(shown.0.get(), 1, "the panel shows the edit in the frame of the click: {release:?}");
    assert_eq!(panel_runs.0.get(), before + 2, "the panel ran for the click and once more for the edit, in that frame: {release:?}");
    let after = panel_runs.0.get();
    h.run();
    assert_eq!(panel_runs.0.get(), after, "the next frames keep the panel");
}

/// A field's view: it hands the field its `Tracked` text through `bind_tracked!` and reads nothing itself.
struct FieldView<'a> {
    text: &'a mut Tracked<String>,
    runs: Seen,
}

impl MemoProps for FieldView<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.text.key(key);
    }
}

fn field_view(props: &mut FieldView<'_>, ui: &mut Ui) {
    props.runs.0.set(props.runs.0.get() + 1);
    let _ = TextField::new(craft_ui_contract::bind_tracked!(props.text)).show(ui);
}

#[test]
fn a_field_s_own_bound_write_reruns_nothing() {
    let text = Rc::new(RefCell::new(Tracked::new(String::new())));
    let view_runs = Seen::default();
    let (t, runs) = (Rc::clone(&text), view_runs.clone());
    let mut h = edit_harness_from(Harness::builder(), move |ui: &mut Ui<'_, '_>| {
        let mut text = t.borrow_mut();
        let _ = MemoView::new("field", FieldView { text: &mut text, runs: runs.clone() }, field_view).show(ui);
    });
    h.run();
    h.key_press(Key::Tab);
    h.run();
    for (typed, c) in ["a", "ab"].into_iter().zip(["a", "b"]) {
        let before = view_runs.0.get();
        h.type_text(c);
        let report = h.last_report();
        assert_eq!(text.borrow().as_str(), typed, "the character reaches the model");
        assert_eq!(report.frame.runs, 1, "the field alone runs, once: {report:?}");
        assert!(view_runs.0.get() <= before + 1, "the view runs at most to reach the field, never again for the field's write: {report:?}");
        h.run();
    }
}
