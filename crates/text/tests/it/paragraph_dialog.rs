//! The Paragraph dialog: what OK answers for typed values and choices, the messages beside values out of range, the Tabs…
//! button, the kind of a spacing, and the preview's lines.

use crate::set;
use craft_text::ui::DocUnit;
use craft_text::ui::character::{
    LineSpacing, ParagraphAlign, ParagraphAnswer, ParagraphDialog, ParagraphFormat, ParagraphPreview, SpacingKind, SpecialIndent,
};
use craft_ui::contract::Show as _;
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

fn start() -> ParagraphFormat {
    set(ParagraphFormat::default(), |f| {
        f.align = Some(ParagraphAlign::Left);
        f.before_text = Some(0.0);
        f.after_text = None;
        f.special = Some(SpecialIndent::None);
        f.by = Some(0.0);
        f.space_before = Some(0.0);
        f.space_after = Some(0.0);
        f.line_spacing = Some(LineSpacing::Multiple(1.5));
    })
}

/// The dialog on a host, with the answers it gave.
struct Rig {
    fixture: Fixture,
    answers: Rc<RefCell<Vec<ParagraphAnswer>>>,
}

impl Rig {
    fn new(format: ParagraphFormat) -> Rig {
        let mut host = DialogHost::new();
        host.open(ParagraphDialog::new(format).unit(DocUnit::new(UnitKind::Inches, 96.0)));
        let host = Rc::new(RefCell::new(host));
        let answers = Rc::new(RefCell::new(Vec::new()));
        let sink = answers.clone();
        let mut fixture = Fixture::new(move |ui| {
            host.borrow_mut().run(ui);
            if let Some(DialogResult::Ok(answer)) = host.borrow_mut().take::<ParagraphDialog>() {
                sink.borrow_mut().push(answer);
            }
        });
        fixture.settle();
        Rig { fixture, answers }
    }

    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.click(&target);
    }

    fn type_into(&mut self, field: &str, text: &str) {
        self.click(field);
        self.fixture.key(Key::A, Modifiers::COMMAND);
        self.fixture.type_text(text);
        self.fixture.key(Key::Tab, Modifiers::NONE);
    }

    fn choose(&mut self, list: &str, option: &str) {
        self.click(list);
        self.click(option);
    }

    fn shows(&self, text: &str) -> bool {
        self.fixture.texts().iter().any(|painted| painted.text.contains(text))
    }

    fn value_of(&self, field: &str) -> String {
        let target = self.fixture.by_label(field).unwrap_or_else(|| panic!("`{field}` is shown"));
        self.fixture.tree().value_of(target.node).unwrap_or_default().to_owned()
    }

    /// Presses OK and returns the answer it gave, if any.
    fn ok(&mut self) -> Option<ParagraphAnswer> {
        self.click("OK");
        self.answers.borrow().last().cloned()
    }
}

/// Before text 0.5 in, Hanging by 0.25 in and Exactly answer 36, Hanging, 18 and `Exactly(18.0)`.
#[trial]
fn ok_returns_the_format_in_points() {
    let mut rig = Rig::new(start());
    rig.type_into("Before text", "0.5");
    rig.choose("Special", "Hanging");
    rig.type_into("By", "0.25");
    rig.choose("Line spacing", "Exactly");
    let answer = rig.ok().expect("OK answers");
    assert!(!answer.open_tabs);
    let f = answer.format;
    assert_eq!(
        (f.before_text, f.special, f.by, f.line_spacing),
        (Some(36.0), Some(SpecialIndent::Hanging), Some(18.0), Some(LineSpacing::Exactly(18.0)))
    );
}

/// Choosing Exactly with an At of 1.5 answers 18; Double answers `Multiple(2.0)`, and Multiple keeps a multiple.
#[trial]
fn line_spacing_presets_fix_their_at_value() {
    let mut rig = Rig::new(start());
    rig.choose("Line spacing", "Exactly");
    assert_eq!(rig.ok().and_then(|answer| answer.format.line_spacing), Some(LineSpacing::Exactly(18.0)));

    let mut rig = Rig::new(start());
    rig.choose("Line spacing", "Double");
    assert_eq!(rig.ok().and_then(|answer| answer.format.line_spacing), Some(LineSpacing::Multiple(2.0)));

    let mut rig = Rig::new(start());
    rig.choose("Line spacing", "Double");
    rig.choose("Line spacing", "Multiple");
    assert_eq!(rig.ok().and_then(|answer| answer.format.line_spacing), Some(LineSpacing::Multiple(2.0)), "a multiple stays when Multiple is chosen");
}

/// A value out of range shows the field's message and OK waits for a fix.
#[trial]
fn a_value_out_of_range_refuses_ok_with_its_message() {
    let format = set(start(), |f| f.space_before = Some(2000.0));
    let mut rig = Rig::new(format);
    assert!(rig.shows("Spacing before must be from"));
    assert!(rig.ok().is_none(), "OK is refused");

    rig.type_into("Spacing before", "12");
    assert!(!rig.shows("Spacing before must be from"));
    let answer = rig.ok().expect("OK answers once the value is in range");
    assert_eq!(answer.format.space_before, Some(12.0));
}

/// Tabs… answers with `open_tabs` and the format as edited.
#[trial]
fn tabs_button_answers_with_open_tabs() {
    let mut rig = Rig::new(start());
    rig.type_into("Spacing after", "6");
    rig.click("Tabs…");
    let answer = rig.answers.borrow().last().cloned().expect("Tabs… answers");
    assert!(answer.open_tabs);
    assert_eq!(answer.format, set(start(), |f| f.space_after = Some(6.0)));
}

/// The preview's lines follow the indents and spacing: a hanging indent pulls the first line out, the paragraph's
/// spacing moves the lines under it.
#[trial]
fn the_preview_lays_out_the_values() {
    let format = set(ParagraphFormat::default(), |f| {
        f.before_text = Some(36.0);
        f.special = Some(SpecialIndent::Hanging);
        f.by = Some(18.0);
        f.space_before = Some(10.0);
        f.line_spacing = Some(LineSpacing::Exactly(20.0));
    });
    let mut fixture = Fixture::new(move |ui| {
        ParagraphPreview::new("Preview", &format, "Sample Text").show(ui);
    });
    fixture.settle();
    let lines = fixture.texts().to_vec();
    assert_eq!(lines.len(), 9, "three paragraphs of three lines");
    let subject = &lines[3..6];
    assert!((subject[1].rect.left() - subject[0].rect.left() - 18.0).abs() < 0.01, "the hanging indent pulls the first line out");
    assert!((subject[1].rect.top() - subject[0].rect.top() - 20.0).abs() < 0.01, "the lines are 20 apart");
    let before = lines[2].rect.top();
    assert!(subject[0].rect.top() - before >= 10.0, "the space before moves the paragraph down");
}

/// A spacing kept in lines shows its number without points, answers the kind it opened with while the value is
/// untouched, and answers points once the value changes.
#[trial]
fn a_spacing_in_lines_keeps_its_kind_until_its_value_changes() {
    let lines = set(start(), |f| {
        f.space_before = Some(1.5);
        f.space_before_kind = SpacingKind::Lines;
    });
    let mut rig = Rig::new(lines.clone());
    let shown = rig.value_of("Spacing before");
    assert!(shown.starts_with("1.5") && !shown.contains("pt"), "{shown:?}");
    let answer = rig.ok().expect("OK answers");
    assert_eq!((answer.format.space_before, answer.format.space_before_kind), (Some(1.5), SpacingKind::Lines));

    let mut rig = Rig::new(lines);
    rig.type_into("Spacing before", "12");
    let answer = rig.ok().expect("OK answers");
    assert_eq!((answer.format.space_before, answer.format.space_before_kind), (Some(12.0), SpacingKind::Points));
    assert_eq!(answer.format.space_after_kind, SpacingKind::Points);
}

/// A spacing of 11 lines is out of range (DeckCraft keeps at most 10).
#[trial]
fn lines_of_spacing_have_their_own_range() {
    let rig = Rig::new(set(start(), |f| {
        f.space_after = Some(11.0);
        f.space_after_kind = SpacingKind::Lines;
    }));
    assert!(rig.shows("Spacing after must be from 0 to 10"));
}
