//! What text layout and control sizing read of the font stack the text engine sets text in: the stack's facts, kept
//! in a window env cell that the window root measures again only when the engine's stack generation moves.

use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use craft_ui_rt::{BuildCx, FontFace, FontSpec, Overflow, TextEngine, TextSpec};

use crate::contract::rt::EnvCells;
use crate::fonts::system::ui_face::SYSTEM_UI_REGULAR;

/// The size, in points, at which the leading face's line height is measured. Large, so the pixel rounding of a row
/// height is negligible in the ratio.
const LINE_PROBE_SIZE: f32 = 100.0;

/// What text layout and control sizing read of the font stack the engine sets text in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[doc(hidden)]
pub struct StackFacts {
    /// Whether the OS's interface face leads the interface family ([`craft_theme::InterfaceFont::System`], once the
    /// face resolved).
    pub system_ui: bool,
    /// Whether the stack has a semibold family ([`crate::fonts::SEMIBOLD`]).
    pub semibold: bool,
    /// Whether the stack has a tab-title family ([`crate::fonts::TAB_TITLE`]).
    pub tab_title: bool,
    /// The line height per point of text of the face that leads the interface family; 0 before a stack is set.
    pub line_ratio: f32,
}

/// The facts of the stack `engine` sets text in, the line ratio from a probe layout; the defaults before a stack.
pub(crate) fn facts_of(engine: &mut dyn TextEngine) -> StackFacts {
    let Some(stack) = engine.stack() else { return StackFacts::default() };
    let system_ui = stack.family(&FontFace::Slot(FontSlot::Ui)).is_some_and(|faces| faces.iter().any(|face| &*face.name == SYSTEM_UI_REGULAR));
    let semibold = stack.family(&FontFace::Slot(FontSlot::UiSemibold)).is_some();
    let tab_title = stack.family(&FontFace::Slot(FontSlot::UiCondensed)).is_some();
    let probe = TextSpec {
        text: Arc::from("x"),
        font: FontSpec { slot: FontSlot::Ui, size: LINE_PROBE_SIZE },
        line_height: None,
        color: Rgba([0, 0, 0, 255]),
        max_width: None,
        overflow: Overflow::Clip,
    };
    let line_ratio = engine.layout(&probe, 1.0).size.y / LINE_PROBE_SIZE;
    StackFacts { system_ui, semibold, tab_title, line_ratio }
}

/// Measures the stack's facts into the window's `cells` when the engine's stack generation moved since the window root
/// last measured them. The generation is a tracked read, so a stack change runs the window root again; the cell is
/// written only when the facts differ.
pub(crate) fn refresh_facts(cx: &mut BuildCx<'_>, cells: EnvCells) {
    let generation = cx.env().text_stack.get(cx);
    let seen = cx.slot("craft-font-facts", || None::<u64>);
    if *cx.peek_state(seen) == Some(generation) {
        return;
    }
    let Some(engine) = cx.text_engine() else { return };
    let facts = facts_of(engine);
    seen.set(cx, Some(generation));
    cells.fonts.set_if_changed(cx, facts);
}

/// The facts the node `cx` builds sees: its window's cell, read with a recorded read so a change runs the node again;
/// the defaults outside a window root.
pub(crate) fn stack_facts(cx: &mut BuildCx<'_>) -> StackFacts {
    match EnvCells::of(cx) {
        Some(cells) => cells.fonts.get(cx),
        None => StackFacts::default(),
    }
}

/// [`stack_facts`] without recording the read, for code whose run read its env ([`crate::runtime::env::Env::of`]
/// records a read of every env cell, this one included).
pub(crate) fn peek_stack_facts(cx: &BuildCx<'_>) -> StackFacts {
    EnvCells::of(cx).map_or_else(StackFacts::default, |cells| *cx.peek_state(cells.fonts))
}

#[cfg(all(feature = "testing", any(test, feature = "suite")))]
pub(crate) mod facts_tests {
    use super::facts_of;
    use crate::testing::trial;
    use craft_theme::{Contrast, FontSlot, Mode, TypeRole};
    use craft_ui_rt::record::FixedText;
    use craft_ui_rt::{FaceTweak, FontFace, FontStack, StackFace, TextEngine};
    use egui::FontFamily;

    use crate::fonts::{SEMIBOLD, TAB_TITLE};

    fn face(name: &str) -> StackFace {
        StackFace { name: name.into(), bytes: std::sync::Arc::from(&b""[..]), index: 0, coords: Vec::new(), tweak: FaceTweak::default() }
    }

    fn stack(slots: &[FontSlot]) -> FontStack {
        let mut stack = FontStack::default();
        for slot in slots {
            stack.set_family(FontFace::Slot(*slot), vec![face(&format!("{slot:?}"))]);
        }
        stack
    }

    /// A title draws in the semibold family only where the stack has one, else in the interface family, so text never
    /// lands in a family with no faces.
    #[trial]
    fn a_title_draws_in_a_family_the_stack_has() {
        let tokens = crate::testing::style::tokens(Mode::Light, Contrast::Standard);
        let mut engine = FixedText::default();
        engine.set_stack(&stack(&[FontSlot::Ui, FontSlot::UiSemibold]));
        let facts = facts_of(&mut engine);
        assert!(facts.semibold, "the stack has the semibold family");
        assert_eq!(tokens.font(facts, TypeRole::Title, false).family, FontFamily::Name(SEMIBOLD.into()), "and a title draws in it");
        engine.set_stack(&stack(&[FontSlot::Ui]));
        let facts = facts_of(&mut engine);
        assert!(!facts.semibold, "a stack without it says so");
        assert_eq!(tokens.font(facts, TypeRole::Title, false).family, FontFamily::Proportional);
    }

    /// A tab title draws in the tab-title family where the stack has it, else as a semibold label; it keeps the label's
    /// size, and a strong label never takes the tab-title family.
    #[trial]
    fn a_tab_title_draws_in_the_tab_title_family_only_where_the_stack_has_it() {
        let tokens = crate::testing::style::tokens(Mode::Light, Contrast::Standard);
        let mut engine = FixedText::default();
        let font = |facts| tokens.font_weighted(facts, TypeRole::TabTitle, false, true);
        engine.set_stack(&stack(&[FontSlot::Ui, FontSlot::UiSemibold]));
        let facts = facts_of(&mut engine);
        assert!(!facts.tab_title);
        assert_eq!(font(facts).family, FontFamily::Name(SEMIBOLD.into()), "a tab title draws in the semibold family");
        assert_eq!(font(facts).size, tokens.font(facts, TypeRole::Label, false).size, "at the label's size");
        engine.set_stack(&stack(&[FontSlot::Ui, FontSlot::UiSemibold, FontSlot::UiCondensed]));
        let facts = facts_of(&mut engine);
        assert!(facts.tab_title);
        assert_eq!(font(facts).family, FontFamily::Name(TAB_TITLE.into()), "with the family a tab title draws in it");
        assert_eq!(tokens.font_weighted(facts, TypeRole::Label, false, true).family, FontFamily::Name(SEMIBOLD.into()), "a strong label does not");
        engine.set_stack(&stack(&[FontSlot::Ui]));
        assert_eq!(font(facts_of(&mut engine)).family, FontFamily::Proportional, "without either family a tab title draws in the interface one");
    }

    /// The line ratio is the leading face's line height per point, and 0 before any stack is set.
    #[trial]
    fn the_line_ratio_comes_from_the_engine_once_a_stack_is_set() {
        let mut engine = FixedText::default();
        assert_eq!(facts_of(&mut engine).line_ratio, 0.0);
        engine.set_stack(&stack(&[FontSlot::Ui]));
        assert_eq!(facts_of(&mut engine).line_ratio, 1.25, "the engine's line height at the probe size, per point");
    }
}
