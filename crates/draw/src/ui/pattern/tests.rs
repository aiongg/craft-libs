//! Tests of the pattern fill editor and dialog. PhotoCraft's
//! `dialogs/pattern_fill_ui/tests.rs` cases move here.

use egui::accesskit::Role;
use egui::{Context, Event, Key, Painter, Rect, Ui, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, Node};

use super::{PatternColors, PatternFillDialog, PatternFillEditor, PatternFillOptions, PatternSwatch};
use craft_theme::Srgba;
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

fn swatches() -> Vec<PatternSwatch> {
    ["25 %", "50 %", "Bubbles"].iter().map(|name| PatternSwatch { id: name.to_lowercase(), name: (*name).to_owned() }).collect()
}

fn paint(painter: &Painter, rect: Rect, _: &PatternSwatch) {
    painter.rect_filled(rect, 0.0, egui::Color32::GRAY);
}

/// A host of one editor that applies every value it reports.
struct EditorHost {
    options: PatternFillOptions,
    swatches: Vec<PatternSwatch>,
    events: Vec<EditEvent<PatternFillOptions>>,
}

impl EditorHost {
    fn show(&mut self, ui: &mut Ui) {
        let r = PatternFillEditor::new("pattern", &self.options, &self.swatches).thumbnails(paint).show(ui);
        if let Some(event) = r.event {
            if let Some(value) = event.value() {
                self.options = value.clone();
            }
            self.events.push(event);
        }
    }
}

fn editor(options: PatternFillOptions) -> Harness<'static, EditorHost> {
    let host = EditorHost { options, swatches: swatches(), events: Vec::new() };
    let mut harness = Harness::builder()
        .with_size([480.0, 640.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut EditorHost| host.show(ui), host);
    harness.run();
    harness
}

fn field<'a, S>(harness: &'a Harness<'_, S>, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name.as_str()))
}

/// Types `text` into the number field `name` and commits it with Tab.
fn type_into<S>(harness: &mut Harness<'_, S>, name: &str, text: &str) {
    field(harness, name).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

// Typing 2000 in Scale gives 1000, 0 gives 1; Snap to Origin reports `Set` with phase (0, 0).
#[test]
fn pattern_editor_scale_range_and_snap_to_origin() {
    let mut options = PatternFillOptions::bitmap("bubbles");
    options.phase = [12.0, 7.0];
    let mut harness = editor(options);

    type_into(&mut harness, "Scale", "2000");
    assert_eq!(harness.state().options.scale, 1000.0);
    type_into(&mut harness, "Scale", "0");
    assert_eq!(harness.state().options.scale, 1.0);

    harness.state_mut().events.clear();
    harness.get_by_label("Snap to Origin").click();
    harness.run();
    assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { value, .. }] if value.phase == [0.0, 0.0]));
}

// With `colors` the editor shows the swatch grid and Foreground and Background chips, no Scale or
// Angle; a cell click reports `Set` with its id.
#[test]
fn pattern_editor_preset_mode_shows_foreground_and_background() {
    let colors = PatternColors { foreground: Srgba::BLACK, background: Srgba::WHITE };
    let mut harness = editor(PatternFillOptions::preset("25 %".to_lowercase(), colors));
    assert!(harness.query_by_label("Foreground").is_some());
    assert!(harness.query_by_label("Background").is_some());
    assert!(harness.query_by_label("Scale").is_none());
    assert!(harness.query_by_label("Angle").is_none());

    harness.get_by_label("Bubbles").click();
    harness.run();
    assert!(
        matches!(harness.state().events.as_slice(), [EditEvent::Set { value, .. }] if value.pattern == "bubbles" && value.colors == Some(colors))
    );
}

/// A host of one dialog.
struct DialogHost {
    dialog: PatternFillDialog,
    result: Option<DialogResult<PatternFillOptions>>,
    saw_changed: bool,
}

fn dialog(options: PatternFillOptions) -> Harness<'static, DialogHost> {
    let dialog = PatternFillDialog::new(options, swatches()).thumbnails(paint);
    let mut harness = Harness::<DialogHost>::builder()
        .with_size(vec2(640.0, 640.0))
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut DialogHost| {
                let ctx = ui.ctx().clone();
                let mut stack = DialogStack::default();
                stack.push(host.dialog.id(), None);
                if let Some(result) = host.dialog.show(&ctx, &stack) {
                    host.result = Some(result);
                }
                host.saw_changed |= host.dialog.changed();
            },
            DialogHost { dialog, result: None, saw_changed: false },
        );
    harness.run();
    harness
}

// A Scale edit in the dialog moves `current()` and sets `changed()` that pass; Cancel answers
// `DialogResult::Cancel`.
#[test]
fn pattern_dialog_reports_live_values_and_cancel_returns_nothing() {
    let mut harness = dialog(PatternFillOptions::bitmap("bubbles"));
    assert!(!harness.state().saw_changed);
    type_into(&mut harness, "Scale", "250");
    assert_eq!(harness.state().dialog.current().scale, 250.0);
    assert!(harness.state().saw_changed);
    assert!(!harness.state().dialog.unchanged());
    assert!(harness.state().result.is_none(), "the dialog stays open until it is answered");

    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().result, Some(DialogResult::Cancel));
}

// OK without a change answers the opening values and `unchanged()` is true.
#[test]
fn pattern_dialog_unchanged_ok_is_a_no_op() {
    let opening = PatternFillOptions::bitmap("bubbles");
    let mut harness = dialog(opening.clone());
    harness.get_by_label("OK").click();
    harness.run();
    assert_eq!(harness.state().result, Some(DialogResult::Ok(opening)));
    assert!(harness.state().dialog.unchanged());
    assert!(!harness.state().saw_changed);
}
