//! The report dialog in a harness.

use egui::{Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

struct App {
    dialog: ReportDialog,
    result: Option<DialogResult<()>>,
    copied: Vec<String>,
}

fn notes(count: usize) -> Vec<ReportNote> {
    (0..count).map(|i| ReportNote::new(if i % 2 == 0 { NoteTone::Warning } else { NoteTone::Info }, format!("Font {i} was substituted"))).collect()
}

fn harness(count: usize, height: f32) -> Harness<'static, App> {
    let dialog = ReportDialog::new("report", "Import Report", "Imported 12 pages.", notes(count));
    Harness::<App>::builder().with_size(vec2(640.0, height)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = ui.ctx().clone();
            let mut stack = DialogStack::default();
            stack.push(app.dialog.id(), None);
            if let Some(result) = app.dialog.show(&ctx, &stack) {
                app.result = Some(result);
            }
            app.copied.extend(ctx.output(|o| {
                o.commands.iter().filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None }).collect::<Vec<_>>()
            }));
        },
        App { dialog, result: None, copied: Vec::new() },
    )
}

// The summary, the count and every note show; the list scrolls past the body height.
#[test]
fn summary_count_and_notes_show_and_scroll() {
    let mut harness = harness(3, 480.0);
    harness.run();
    assert!(harness.query_by_label("Imported 12 pages.").is_some());
    assert!(harness.query_by_label("3 notes").is_some());
    for i in 0..3 {
        assert!(harness.query_by_label(&format!("Font {i} was substituted")).is_some(), "note {i}");
    }

    let mut many = self::harness(60, 300.0);
    many.run();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(640.0, 300.0));
    let shown = many.get_by_label("OK").rect();
    assert!(screen.contains_rect(shown), "the footer stays on screen: {shown:?} in {screen:?}");
}

// Copy puts the notes on the clipboard as `as_text`; Enter and Esc both close.
#[test]
fn copy_writes_the_notes_and_either_key_closes() {
    let mut harness = harness(2, 480.0);
    harness.run();
    harness.get_by_label("Copy").click();
    harness.run();
    let expected = harness.state().dialog.as_text();
    assert_eq!(expected, "Warning: Font 0 was substituted\nInfo: Font 1 was substituted");
    assert_eq!(harness.state().copied, [expected]);
    assert!(harness.state().result.is_none());

    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(harness.state().result, Some(DialogResult::Ok(())));

    let mut escaped = self::harness(1, 480.0);
    escaped.run();
    escaped.key_press(Key::Escape);
    escaped.run();
    assert_eq!(escaped.state().result, Some(DialogResult::Ok(())));
}
