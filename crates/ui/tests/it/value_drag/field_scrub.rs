//! A text field that scrubs under a drag on the field itself: the text takes the pointer only once the field is in edit.

use craft_behavior::{Codec, Metrics, NotAValue, PartKey, Sense, ValueDrag};
use craft_ui::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Input, MeasureCx, PartsCx, Role, Scrub, Show as _};
use craft_ui::primitives::{TextKey, TextOptions};
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::text::{caret, widget_id};
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use egui::{Rect, Ui, Vec2, vec2};

const START: f64 = 100.0;
const TEXT: PartKey = PartKey::new("text");
const SIZE: Vec2 = Vec2::new(120.0, 24.0);
/// How far the drag moves; the value changes by one per point.
const TRAVEL: f32 = 10.0;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
struct ReadoutStyle {
    #[style(metrics)]
    metrics: Metrics,
}

#[derive(Clone, Copy)]
struct WholeNumber;

impl Codec for WholeNumber {
    type Value = f64;

    fn format(&self, v: &f64) -> String {
        format!("{v}")
    }

    fn parse(&self, text: &str, _start: &f64) -> Result<f64, NotAValue> {
        text.trim().parse().map_err(|_| NotAValue)
    }
}

/// A readout with no side label: a drag on the field scrubs its number.
#[derive(Component)]
struct Readout<'a> {
    #[common]
    common: Common,
    value: Bind<'a, f64>,
    options: TextOptions,
}

impl Component for Readout<'_> {
    const ROLE: Role = Role::SPIN_BUTTON;
    const INPUT: Input = Input::Text;
    const KEYS: KeyUse = KeyUse::of(Input::Text);
    type Style = ReadoutStyle;
    type Output = EditResponse<f64>;

    fn style(_: &StyleCx<'_>, _: ReadoutStyleKey) -> ReadoutStyle {
        ReadoutStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::DRAG | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        SIZE
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let rect = cx.content_rect;
        if let Some(value) = cx.value() {
            cx.text(TextKey::new(TEXT), rect, &WholeNumber, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, f64>> {
        Some(self.value.reborrow())
    }

    fn scrub(&self) -> Option<Scrub<f64>> {
        Some(Scrub::new(ValueDrag::relative(1.0, 0.0..=1000.0), |value| *value, |value| value).on_field())
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f64> {
        cx.edit_response()
    }
}

fn show(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut options = TextOptions::new();
    options.submits = true;
    Readout { common: Common::new(), value: Bind::bound(data.value("value", START)), options }.show(ui).into()
}

fn fixture() -> Fixture {
    let mut fixture = Fixture::story(&Story::new("readout", show));
    fixture.settle();
    fixture
}

fn readout(fixture: &Fixture) -> craft_ui::testing::Target {
    fixture.by_role(Role::SPIN_BUTTON).first().cloned().expect("the readout is a spin button")
}

/// A press and drag on the field scrubs the value through the component's value drag.
#[trial]
fn a_press_and_drag_on_a_scrubbing_field_scrubs_the_value() {
    let mut fixture = fixture();
    let target = readout(&fixture);
    fixture.drag(&target, vec2(TRAVEL, 0.0));
    assert_eq!(*fixture.data().value("value", START), START + f64::from(TRAVEL));
}

/// A click without travel enters edit with the caret at the click, not at the end or over the whole text.
#[trial]
fn a_click_on_a_scrubbing_field_enters_edit_with_the_caret_at_the_click() {
    let mut fixture = fixture();
    let target = readout(&fixture);
    fixture.click_at(target.rect.left_center() + vec2(1.0, 0.0));
    fixture.idle(2);
    let id = fixture.root_id().expect("the readout has an id");
    let widget = widget_id(id, TEXT);
    assert_eq!(fixture.focused(), Some(widget), "the click puts the field in edit");
    assert_eq!(caret(fixture.ctx(), widget), Some(0));
    assert_eq!(*fixture.data().value("value", START), START, "and changes no value");
}
