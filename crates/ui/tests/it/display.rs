//! Display components: the keycap chord, host ink, text roles, the divider and the value maths of `Badge::count` and `Progress`.

use craft_behavior::{Metrics, PartKey, Sense};
use craft_host::target::Os;
use craft_theme::Stateful;
use craft_ui::buttons::Button;
use craft_ui::contract::{CommandSet, Common, Component, Cx, MeasureCx, PaintUi, PartsCx, Props as _, Role, Show as _};
use craft_ui::display::{Badge, Divider, Kbd, Progress, Text};
use craft_ui::style::{ComponentStyle, Flags, StyleCx, TextLook, TypeRole};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Rect, Response, Vec2, vec2};
use std::cell::Cell;
use std::rc::Rc;

/// Commands of the Kbd trial: one with a default chord and one without.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "kbd_trial")]
#[non_exhaustive]
pub enum KbdCommand {
    #[command(label = "Chorded", default = "Cmd+Alt+Shift+K")]
    Chorded,
    #[command(label = "Bare")]
    Bare,
}

/// A Kbd bound to a command measures the chord step 1 resolved for it, so the room it takes matches what it paints.
#[trial]
fn a_kbd_bound_to_a_command_measures_the_resolved_chord() {
    let width = |command: KbdCommand| {
        let mut frames = Frames::new();
        let mut width = 0.0;
        frames.idle(3, |ui| width = Kbd::command(command).id_salt("kbd").show(ui).rect.width());
        width
    };
    assert!(width(KbdCommand::Chorded) > width(KbdCommand::Bare), "a command with a chord takes the chord's room");
}

/// A count shows its digits up to the cap and `99+` above it.
#[trial]
fn a_count_badge_caps_at_ninety_nine() {
    for (count, shown) in [(7, "7"), (99, "99"), (100, "99+"), (1234, "99+")] {
        let mut fixture = Fixture::new(move |ui| {
            Badge::count(count).show(ui);
        });
        fixture.settle();
        assert!(fixture.by_label(shown).is_some(), "a count of {count} reads {shown}");
    }
}

/// A fraction outside 0 to 1 draws and reports as the nearest end, and one that is not a number as empty.
#[trial]
fn a_progress_fraction_is_clamped() {
    assert_eq!(Progress::clamped(1.7), 1.0);
    assert_eq!(Progress::clamped(-3.0), 0.0);
    assert_eq!(Progress::clamped(f32::NAN), 0.0);
    assert_eq!(Progress::clamped(0.25), 0.25);
}

/// The painted texts of a keycap chord, each inside a box of its own: one keycap per key.
fn keycaps_of(fixture: &Fixture) -> Vec<(String, usize)> {
    fixture
        .texts()
        .iter()
        .map(|text| {
            let boxes: Vec<&Rect> = fixture.shape_bounds().iter().filter(|b| b.contains_rect(text.rect) && b.area() > text.rect.area()).collect();
            (text.text.clone(), boxes.len())
        })
        .collect()
}

/// The keycap form draws one box per key, names the chord for a screen reader in the spoken form, and shows
/// nothing for a command with no chord.
#[trial]
fn a_keycap_chord_paints_one_box_per_key_and_is_named_spoken() {
    let mut fixture = Fixture::new(|ui| {
        Kbd::command(KbdCommand::Chorded).keycap(true).id_salt("kbd").show(ui);
    });
    fixture.os(Os::Windows).settle();
    let texts: Vec<String> = fixture.texts().iter().map(|text| text.text.clone()).collect();
    assert_eq!(texts, ["Ctrl", "Alt", "Shift", "K"], "one run per key");
    assert!(keycaps_of(&fixture).iter().all(|(_, boxes)| *boxes == 1), "each key sits in its own box: {:?}", keycaps_of(&fixture));

    fixture.os(Os::Mac).settle();
    assert!(fixture.by_label("Option+Shift+Cmd+K").is_some(), "the node name is the spoken chord, not the symbols");

    let mut bare = Fixture::new(|ui| {
        Kbd::command(KbdCommand::Bare).keycap(true).id_salt("bare").show(ui);
    });
    bare.settle();
    assert!(bare.texts().is_empty() && bare.shape_bounds().is_empty(), "a command with no chord paints nothing");
}

/// An arrow key reads as its word off macOS and as its glyph on macOS.
#[trial]
fn a_chord_shows_arrow_keys_as_words_off_macos_and_glyphs_on_it() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
    #[command(prefix = "kbd_arrow")]
    #[non_exhaustive]
    enum Arrow {
        #[command(label = "Next", default = "Cmd+Down")]
        Next,
    }
    let shown = |os: Os| {
        let mut fixture = Fixture::new(|ui| {
            Kbd::command(Arrow::Next).id_salt("arrow").show(ui);
        });
        fixture.os(os).settle();
        fixture.texts().iter().map(|text| text.text.clone()).collect::<Vec<_>>()
    };
    assert_eq!(shown(Os::Windows), ["Ctrl+Down"]);
    assert_eq!(shown(Os::Mac), ["⌘↓"]);
}

/// The ink a host paints its part with, set on its selected state.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct HostStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(text)]
    ink: TextLook,
}

const FACE: PartKey = PartKey::new("face");

/// A selected row that shows a chord as the face of a part.
#[derive(Component)]
struct SelectedRow {
    #[common]
    common: Common,
}

impl Component for SelectedRow {
    const ROLE: Role = Role::None;
    type Style = HostStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: SelectedRowStyleKey) -> HostStyle {
        let colors = &cx.tokens.color;
        let ink = Stateful::new(cx.color(colors.text.normal)).when(Flags::SELECTED, cx.color(colors.accent.solid));
        HostStyle { metrics: Metrics::new(), ink: TextLook::new(TypeRole::Body, ink) }
    }

    fn flags(&self) -> Flags {
        Flags::SELECTED
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        vec2(80.0, 30.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, rect).sense(Sense::HOVER);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        ui.face(FACE, Kbd::new("Ctrl+K"));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn texts_of(fixture: &Fixture, type_name: &str) -> Vec<TextLook> {
    fixture.styles_of(type_name).iter().flat_map(|seen| seen.looks.texts.clone()).collect()
}

/// A chord shown on a selected row, and a spinner on a ghost button, paint with the ink of the host that shows
/// them, not a fallback of their own.
#[trial]
fn faces_paint_with_the_ink_of_their_host() {
    let mut row = Fixture::new(|ui| {
        SelectedRow { common: Common::new() }.id_salt("row").show(ui);
    });
    row.settle();
    let host = texts_of(&row, std::any::type_name::<SelectedRow>());
    let chord = texts_of(&row, std::any::type_name::<Kbd>());
    assert!(
        chord.iter().any(|look| host.iter().any(|ink| look.color == ink.color)),
        "the chord reads in the selected row's ink: {chord:?} against {host:?}"
    );

    let mut button = Fixture::new(|ui| {
        Button::new("Save").variant(craft_theme::Variant::GHOST).loading(true).id_salt("save").show(ui);
    });
    button.settle();
    let host = texts_of(&button, std::any::type_name::<Button>());
    let spinner: Vec<_> = button
        .styles_of(std::any::type_name::<craft_ui::display::Spinner>())
        .iter()
        .flat_map(|seen| seen.looks.icons.iter().map(|look| look.color.clone()))
        .collect();
    assert!(spinner.iter().any(|color| host.iter().any(|ink| *color == ink.color)), "a loading ghost button's spinner uses the button's ink");
}

/// A text in the section role resolves its style with that role, which is what a theme override or the contrast
/// case reads.
#[trial]
fn a_texts_role_is_part_of_its_style() {
    let mut fixture = Fixture::new(|ui| {
        Text::new("Blend mode").role(TypeRole::Label).id_salt("label").show(ui);
        Text::new("Layers").id_salt("body").show(ui);
    });
    fixture.settle();
    let roles: Vec<TypeRole> = texts_of(&fixture, std::any::type_name::<Text>()).iter().map(|look| look.role).collect();
    assert!(roles.contains(&TypeRole::Label) && roles.contains(&TypeRole::Body), "one style per role: {roles:?}");
}

/// A vertical divider takes the height of a row taller than a control.
#[trial]
fn a_vertical_divider_spans_its_row() {
    let height = Rc::new(Cell::new(0.0));
    let seen = height.clone();
    let mut frames = Frames::new();
    frames.idle(3, move |ui| {
        let row = Rect::from_min_size(ui.cursor().min, vec2(200.0, 120.0));
        craft_ui::testing::layout::in_row_region(ui, row, |ui| {
            seen.set(Divider::vertical().id_salt("divider").show(ui).rect.height());
        });
    });
    const ROW: f32 = 119.0;
    assert!(height.get() >= ROW, "the divider fills the 120 point row, not one control: {}", height.get());
}
