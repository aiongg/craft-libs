//! The row line: the lines in a row share one vertical centre, and its blocks and columns its top.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Entry, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};

/// How far apart two centres may lie: pixel rounding.
const ROUNDING: f32 = 1.0;

/// A caption, a select and a button in a row put their text on one centre line, as in a bar's captioned cell. Guards
/// against row children placed on the row's top edge, where a caption shorter than the controls sits above their text.
#[trial]
fn a_caption_a_select_and_a_button_in_a_row_share_a_centre_line() {
    let mut fixture = Fixture::new(|ui| {
        Row::new().show(ui, |ui, _| {
            Text::new("Theme").show(ui);
            let entries = [Entry::new(0u8, "Studio"), Entry::new(1, "Paper")];
            Select::new("Theme", Bind::bound(&mut 0u8), &entries).show(ui);
            Button::new("Apply").show(ui);
        });
    });
    fixture.settle();
    fixture.idle(3);
    let centre = |text: &str| {
        let found = fixture.texts().iter().find(|painted| painted.text == text).unwrap_or_else(|| panic!("no {text} painted"));
        found.rect.center().y
    };
    let (caption, select, button) = (centre("Theme"), centre("Studio"), centre("Apply"));
    assert!((caption - select).abs() <= ROUNDING && (select - button).abs() <= ROUNDING, "caption {caption}, select {select}, button {button}");
}

/// A short column beside a taller one starts at the row's top, while a button and a nested row stay on the centre
/// of the line. Guards against a nav column centred beside a taller page, and against lines moved to the top.
#[trial]
fn a_short_column_in_a_row_sits_at_the_top_and_lines_at_the_centre() {
    let mut fixture = Fixture::new(|ui| {
        Row::new().show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                Text::new("Short").show(ui);
            });
            Stack::new().show(ui, |ui, _| {
                for line in ["One", "Two", "Three", "Four"] {
                    Text::new(line).show(ui);
                }
            });
            Button::new("Go").show(ui);
            Row::new().show(ui, |ui, _| {
                Text::new("Inline").show(ui);
            });
        });
    });
    fixture.settle();
    fixture.idle(3);
    let rect = |text: &str| fixture.texts().iter().find(|painted| painted.text == text).unwrap_or_else(|| panic!("no {text} painted")).rect;
    let (short, top, bottom) = (rect("Short"), rect("One"), rect("Four"));
    assert!((short.top() - top.top()).abs() <= ROUNDING, "short column at {}, tall column at {}", short.top(), top.top());
    let line = (top.top() + bottom.bottom()) / 2.0;
    for text in ["Go", "Inline"] {
        let centre = rect(text).center().y;
        assert!((centre - line).abs() <= ROUNDING, "{text} centred at {centre}, the line at {line}");
    }
}
