//! Tests of the stroke editor and its button.

use egui::accesskit::{Role, Toggled};
use egui::{Context, Event, Key, Sense, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{LineCap, LineJoin, StrokeButton, StrokeChoice, StrokeChoiceRow, StrokeEditor, StrokeField, StrokeMixed, StrokeSections, StrokeStyle};
use craft_ui::controls::MixedValue;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

/// A host of one stroke style that applies every value the editor reports.
struct Host {
    style: StrokeStyle,
    mixed: StrokeMixed,
    sections: StrokeSections,
    closed: bool,
    corners: bool,
    /// Shows the app row "Arrowhead" (choices "Open" and "Filled", "Open" picked).
    rows: bool,
    /// Shows the button in place of the editor body.
    button: bool,
    open: bool,
    target: u64,
    enabled: bool,
    events: Vec<EditEvent<StrokeStyle>>,
    fields: Vec<StrokeField>,
    choices: Vec<(String, String)>,
    /// Presses that reached the strip beside the button.
    canvas_clicks: usize,
    /// Whether the opener is disabled.
    disable: bool,
}

impl Host {
    fn new(style: StrokeStyle) -> Self {
        Self {
            style,
            mixed: StrokeMixed::default(),
            sections: StrokeSections::POPOVER,
            closed: true,
            corners: true,
            rows: false,
            button: false,
            open: false,
            target: 1,
            enabled: true,
            events: Vec::new(),
            fields: Vec::new(),
            choices: Vec::new(),
            canvas_clicks: 0,
            disable: false,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let numbers = LocaleNumbers::from_tag("en");
        let style = self.style.clone();
        let choices = [StrokeChoice::new("open", "Open"), StrokeChoice::new("filled", "Filled")];
        let rows = [StrokeChoiceRow::new("arrow", "Arrowhead", &choices).selected(MixedValue::Same("open")).paint(|_, _, _| {})];
        let (event, field, choice) = if self.button {
            ui.horizontal_top(|ui| {
                let r = ui
                    .add_enabled_ui(!self.disable, |ui| {
                        StrokeButton::new("stroke", &style, &numbers)
                            .enabled(self.enabled)
                            .closed(self.closed)
                            .corners(self.corners)
                            .target(self.target)
                            .show(ui, &mut self.open)
                    })
                    .inner;
                ui.add_space(400.0);
                let (rect, canvas) = ui.allocate_exact_size(vec2(100.0, 100.0), Sense::click());
                canvas.widget_info(|| egui::WidgetInfo::labeled(Role::Button, true, "Canvas"));
                ui.painter().rect_filled(rect, 0.0, egui::Color32::GRAY);
                self.canvas_clicks += usize::from(canvas.clicked());
                (r.event, r.field, None)
            })
            .inner
        } else {
            let editor =
                StrokeEditor::new("stroke", &style, &numbers).sections(self.sections).closed(self.closed).corners(self.corners).mixed(&self.mixed);
            let editor = if self.rows { editor.rows(&rows) } else { editor };
            let r = editor.show(ui);
            (r.event, r.field, r.choice)
        };
        if let Some(event) = event {
            if let Some(value) = event.value() {
                self.style = value.clone();
            }
            self.events.push(event);
        }
        self.fields.extend(field);
        self.choices.extend(choice);
    }

    fn last(&self) -> Option<&StrokeStyle> {
        self.events.iter().rev().find_map(EditEvent::value)
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size([700.0, 900.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run();
    harness
}

fn node<'a>(harness: &'a Harness<'_, Host>, role: Role, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == role && n.label().as_deref() == Some(name.as_str()))
}

/// Types `text` into the number field `name` and commits it.
fn type_into(harness: &mut Harness<'_, Host>, name: &str, text: &str) {
    node(harness, Role::SpinButton, name).click();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

fn dashed(dashes: &[f32]) -> StrokeStyle {
    StrokeStyle { dashes: dashes.to_vec(), ..StrokeStyle::default() }
}

/// Whether the node is lit (checked or selected).
fn lit(node: &Node<'_>) -> bool {
    let data = node.accesskit_node().data();
    data.toggled() == Some(Toggled::True) || data.is_selected() == Some(true)
}

fn selected(harness: &Harness<'_, Host>, name: &str) -> Option<bool> {
    Some(lit(&node(harness, Role::RadioButton, name)))
}

/// Runs a few passes: the hover fade of the controls keeps repainting, so `run` never settles.
fn settle(harness: &mut Harness<'_, Host>) {
    harness.run_steps(8);
}

// Clicking the Dashed and Dotted rows reports Sets of the preset style (Dotted with round caps);
// the row matching the value is selected, none for a custom pattern.
#[test]
fn stroke_editor_preset_rows_report_the_preset_style() {
    let mut harness = harness(Host::new(StrokeStyle::default()));
    assert_eq!(selected(&harness, "Solid"), Some(true));
    assert_eq!(selected(&harness, "Dashed"), Some(false));

    node(&harness, Role::RadioButton, "Dashed").click();
    harness.run();
    let want = StrokeStyle::default().with_line_style(super::LineStyle::Dashed);
    assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { value, .. }] if *value == want));
    assert_eq!(harness.state().fields, [StrokeField::Preset]);
    assert_eq!(selected(&harness, "Dashed"), Some(true));

    node(&harness, Role::RadioButton, "Dotted").click();
    harness.run();
    let last = harness.state().last().cloned();
    let Some(last) = last else { panic!("a Set") };
    assert_eq!((last.dashes.as_slice(), last.cap), (&StrokeStyle::DOTTED[..], LineCap::Round));
    assert_eq!(selected(&harness, "Dotted"), Some(true));

    let custom = harness_custom();
    for name in ["Solid", "Dashed", "Dotted"] {
        assert_eq!(selected(&custom, name), Some(false), "{name} is not the custom pattern");
    }
}

fn harness_custom() -> Harness<'static, Host> {
    harness(Host::new(dashed(&[3.0, 1.0])))
}

fn disabled(harness: &Harness<'_, Host>, role: Role, name: &str) -> bool {
    node(harness, role, name).accesskit_node().is_disabled()
}

// PhotoCraft's rules: Alignment is disabled for an open path, Caps for a solid closed path,
// Corners without corners.
#[test]
fn stroke_editor_disables_alignment_on_open_paths_and_caps_on_solid_closed_paths() {
    let closed = harness(Host::new(StrokeStyle::default()));
    assert!(!disabled(&closed, Role::ComboBox, "Alignment"));
    assert!(disabled(&closed, Role::ComboBox, "Caps"), "a solid closed path has no ends");
    assert!(!disabled(&closed, Role::ComboBox, "Corners"));

    let open = Host { closed: false, corners: false, ..Host::new(StrokeStyle::default()) };
    let open = harness(open);
    assert!(disabled(&open, Role::ComboBox, "Alignment"), "an open path is centred");
    assert!(!disabled(&open, Role::ComboBox, "Caps"));
    assert!(disabled(&open, Role::ComboBox, "Corners"));
    assert!(open.query_by_label("Open paths use centered strokes").is_some());

    let dashes = harness(Host::new(dashed(&[4.0, 2.0])));
    assert!(!disabled(&dashes, Role::ComboBox, "Caps"), "a dashed closed path has ends");
}

// The Miter limit field is enabled only for miter corners.
#[test]
fn stroke_editor_miter_limit_only_for_miter_corners() {
    let miter = harness(Host::new(StrokeStyle::default()));
    assert!(!disabled(&miter, Role::SpinButton, "Miter limit"));
    let round = harness(Host::new(StrokeStyle { join: LineJoin::Round, ..StrokeStyle::default() }));
    assert!(disabled(&round, Role::SpinButton, "Miter limit"));
    let none = harness(Host { corners: false, ..Host::new(StrokeStyle::default()) });
    assert!(disabled(&none, Role::SpinButton, "Miter limit"));
}

// Add pair appends [4, 2] (or 2 to an odd pattern) up to 32 entries; Remove pair keeps one pair;
// the dash offset field reports its value.
#[test]
fn stroke_editor_dash_pairs_add_remove_and_offset() {
    let mut harness = harness(Host::new(dashed(&[4.0, 2.0, 3.0])));
    node(&harness, Role::Button, "Add pair").click();
    settle(&mut harness);
    assert_eq!(harness.state().style.dashes, [4.0, 2.0, 3.0, 2.0], "an odd pattern is completed");
    node(&harness, Role::Button, "Add pair").click();
    settle(&mut harness);
    assert_eq!(harness.state().style.dashes, [4.0, 2.0, 3.0, 2.0, 4.0, 2.0]);
    assert_eq!(harness.state().fields, [StrokeField::AddPair, StrokeField::AddPair]);

    node(&harness, Role::Button, "Remove pair").click();
    settle(&mut harness);
    node(&harness, Role::Button, "Remove pair").click();
    settle(&mut harness);
    assert_eq!(harness.state().style.dashes, [4.0, 2.0]);
    assert!(disabled(&harness, Role::Button, "Remove pair"), "one pair stays");

    type_into(&mut harness, "Dash offset", "5");
    assert_eq!(harness.state().last().map(|s| s.dash_offset), Some(5.0));
    assert_eq!(harness.state().fields.last(), Some(&StrokeField::DashOffset));

    let full = harness_full();
    assert!(disabled(&full, Role::Button, "Add pair"), "32 entries are the most");
}

fn harness_full() -> Harness<'static, Host> {
    harness(Host::new(dashed(&[1.0; StrokeStyle::MAX_DASHES])))
}

// A pattern of zeros shows "Use finite lengths and a positive pattern total" and reports no event.
#[test]
fn stroke_editor_shows_the_error_for_an_invalid_pattern_and_reports_nothing() {
    let zeros = harness(Host::new(dashed(&[0.0, 0.0])));
    assert!(zeros.query_by_label("Use finite lengths and a positive pattern total").is_some());
    assert!(zeros.state().events.is_empty());

    let mut harness = harness(Host::new(dashed(&[0.0, 2.0])));
    assert!(harness.query_by_label("Use finite lengths and a positive pattern total").is_none());
    type_into(&mut harness, "Gap", "0");
    assert!(harness.state().events.iter().all(|e| e.value().is_none()), "the pattern [0, 0] is not reported: {:?}", harness.state().events);
    assert_eq!(harness.state().style.dashes, [0.0, 2.0]);
    assert!(
        harness.query_by_label("Use finite lengths and a positive pattern total").is_some(),
        "the typed edit stays pending and shows the message"
    );

    // A valid edit on the pending style reports and clears the message.
    type_into(&mut harness, "Gap", "3");
    assert_eq!(harness.state().style.dashes, [0.0, 3.0]);
    assert!(harness.query_by_label("Use finite lengths and a positive pattern total").is_none());
}

// With `mixed.cap` no cap button or select value is lit; picking one reports a Set for all.
#[test]
fn stroke_editor_mixed_values_light_no_button() {
    let host = Host {
        mixed: StrokeMixed { cap: true, ..StrokeMixed::default() },
        sections: StrokeSections::PANEL,
        closed: false,
        ..Host::new(StrokeStyle { cap: LineCap::Round, ..StrokeStyle::default() })
    };
    let mut harness = harness(host);
    let caps: Vec<_> = ["Butt / Flat", "Round", "Square"].iter().filter_map(|n| harness.query_all_by_label(n).next()).collect();
    assert!(!caps.is_empty());
    assert!(caps.iter().all(|n| !lit(n)), "no cap is lit while mixed");

    let square = harness.get_by_label("Square");
    square.click();
    harness.run();
    let Some(last) = harness.state().last() else { panic!("a Set") };
    assert_eq!(last.cap, LineCap::Square);
    assert!(matches!(harness.state().events.last(), Some(EditEvent::Set { .. })));
    assert_eq!(harness.state().fields, [StrokeField::Cap]);
}

// An app row's pick reports `choice` with the row's and the choice's ids and no style event.
#[test]
fn stroke_editor_choice_row_reports_the_pick() {
    let mut harness = harness(Host { rows: true, ..Host::new(StrokeStyle::default()) });
    node(&harness, Role::ComboBox, "Arrowhead").click();
    harness.run();
    harness.get_by_label("Filled").click();
    harness.run();
    assert_eq!(harness.state().choices, [("arrow".to_owned(), "filled".to_owned())]);
    assert!(harness.state().events.is_empty());
}

fn open_button(host: Host) -> Harness<'static, Host> {
    let mut harness = harness(Host { button: true, ..host });
    node(&harness, Role::Button, "Stroke Options").click();
    settle(&mut harness);
    harness
}

// The popover closes when the target or the value changes from outside; Esc closes it and the
// opener has focus again.
#[test]
fn stroke_button_closes_on_target_change_and_escape_returns_focus() {
    let mut harness = open_button(Host::new(StrokeStyle::default()));
    assert!(harness.state().open);
    assert!(harness.query_by_role_and_label(Role::RadioButton, "Dashed").is_some(), "the editor is in the popover");

    // Its own edit keeps it open.
    node(&harness, Role::RadioButton, "Dashed").click();
    settle(&mut harness);
    assert!(harness.state().open && !harness.state().events.is_empty());

    harness.state_mut().target = 2;
    settle(&mut harness);
    assert!(!harness.state().open, "another layer");

    node(&harness, Role::Button, "Stroke Options").click();
    settle(&mut harness);
    assert!(harness.state().open);
    harness.state_mut().style = StrokeStyle::default();
    settle(&mut harness);
    assert!(!harness.state().open, "undo changed the value");

    node(&harness, Role::Button, "Stroke Options").click();
    settle(&mut harness);
    assert!(harness.state().open);
    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert!(!harness.state().open);
    assert!(node(&harness, Role::Button, "Stroke Options").accesskit_node().is_focused(), "focus returns to the opener");
}

// An outside width change keeps the popover open (the editor never edits it); an outside cap
// change closes it.
#[test]
fn stroke_button_ignores_the_width_when_comparing_the_value() {
    let mut harness = open_button(Host::new(StrokeStyle::default()));
    harness.state_mut().style.width = 9.0;
    settle(&mut harness);
    assert!(harness.state().open, "a width change from outside");

    harness.state_mut().style.cap = LineCap::Square;
    settle(&mut harness);
    assert!(!harness.state().open, "a cap change from outside");
}

// The popover closes when the opener is disabled.
#[test]
fn stroke_button_closes_when_disabled() {
    let mut harness = open_button(Host::new(StrokeStyle::default()));
    assert!(harness.state().open);
    harness.state_mut().disable = true;
    settle(&mut harness);
    assert!(!harness.state().open);
}

fn press_on_canvas(harness: &mut Harness<'_, Host>) {
    let at = node(harness, Role::Button, "Canvas").rect().center();
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    harness.hover_at(at);
    harness.event(button(true));
    settle(harness);
    harness.event(button(false));
    settle(harness);
}

// A press outside the open popover on a canvas closes it and the canvas sees neither the press
// nor its release.
#[test]
fn stroke_button_closing_press_reaches_nothing() {
    let mut harness = open_button(Host::new(StrokeStyle::default()));
    assert!(harness.state().open);
    press_on_canvas(&mut harness);
    assert!(!harness.state().open);
    assert_eq!(harness.state().canvas_clicks, 0);

    press_on_canvas(&mut harness);
    assert_eq!(harness.state().canvas_clicks, 1, "with the popover shut the canvas takes its click");
}

// The opener is a button named "Stroke Options" with the style's name as its value; presets are
// radio buttons with their selected state; number fields carry their names and values.
#[test]
fn stroke_editor_accesskit_names_and_values() {
    let harness = open_button(Host::new(dashed(&[4.0, 2.0])));
    let opener = node(&harness, Role::Button, "Stroke Options");
    assert_eq!(opener.value().as_deref(), Some("Dashed"));
    assert_eq!(selected(&harness, "Dashed"), Some(true));
    assert_eq!(selected(&harness, "Solid"), Some(false));
    let miter = node(&harness, Role::SpinButton, "Miter limit");
    assert_eq!(miter.accesskit_node().numeric_value(), Some(100.0));
    let opacity = node(&harness, Role::SpinButton, "Opacity");
    assert_eq!(opacity.accesskit_node().numeric_value(), Some(100.0));
    assert!(harness.query_by_role_and_label(Role::SpinButton, "Dash").is_some());
    assert!(harness.query_by_role_and_label(Role::CheckBox, "Dashed line").is_some());
    assert!(harness.query_by_role_and_label(Role::Image, "Dashed").is_some(), "the sample is an image named by the style");
}

// The mixed fields are the ones that differ from the first style; no styles give none.
#[test]
fn stroke_mixed_of_marks_the_fields_that_differ() {
    let (a, b) = (StrokeStyle::default(), StrokeStyle { cap: LineCap::Round, opacity: 0.5, ..StrokeStyle::default() });
    let Some((first, mixed)) = StrokeMixed::of([&a, &b, &a]) else { panic!("a first style") };
    assert_eq!(first, a);
    assert_eq!(mixed, StrokeMixed { cap: true, opacity: true, ..StrokeMixed::default() });
    assert!(StrokeMixed::of([]).is_none());
}
