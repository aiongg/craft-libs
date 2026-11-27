//! What sensing hands a component: the part results its hooks read, a text part's outcome, and the parts it declares.
//! ui-rt's hit test over the node's kept shapes and its press machine produce the facts (`contract/rt/run.rs` folds
//! them into `PartResult`s); nothing here registers shapes or runs a press lifecycle.

use craft_behavior::{Cursor, HitPriority, HitShape, PartKey, PressState, Sense};
use craft_theme::Flags;
use craft_ui_rt::{Pos2, Rect};
use std::collections::HashMap;

/// A part's results this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PartResult {
    /// The part's bounds.
    pub rect: Rect,
    /// Its press state.
    pub press: PressState,
    /// The pointer is over it and nothing above takes the hover.
    pub hovered: bool,
    /// The pointer's position while `hovered`, in the owner's layer space: read through `Cx::hover_pos`, which marks the
    /// part as following the pointer.
    pub(crate) hover_pos: Option<Pos2>,
    /// The value flags the component set on the part.
    pub flags: Flags,
    /// What the part senses, as declared.
    pub sense: Sense,
    /// The component declared the part inactive: it paints with the Disabled state and `PaintUi::draw_part` draws data
    /// colours with the disabled fill, and it keeps sensing.
    pub inactive: bool,
    /// The component holds a press on the part that the part does not sense itself: it paints pressed.
    pub held: bool,
}

impl PartResult {
    /// What a read returns for a part with no press history: face mode, or a part first registered this pass.
    pub const IDLE: PartResult = PartResult {
        rect: Rect::NOTHING,
        press: PressState::IDLE,
        hovered: false,
        hover_pos: None,
        flags: Flags::empty(),
        sense: Sense::empty(),
        inactive: false,
        held: false,
    };

    /// The part paints pressed: its own press, or one its component holds on it.
    pub fn pressed(&self) -> bool {
        self.press.phase.is_press() || self.held
    }
}

/// A text part's result this pass.
#[derive(Clone)]
#[non_exhaustive]
pub struct TextOutcome<V> {
    /// The part's rect.
    pub rect: Rect,
    /// The draft being typed, if any.
    pub draft: Option<String>,
    /// The value the draft parsed to.
    pub value: Option<V>,
    /// The laid-out text, so an editor draws gutters and line numbers beside it.
    pub layout: Option<super::TextLayout>,
    /// Whether the caret stroke shows this pass (the caret blink).
    pub caret_visible: bool,
    /// The draft is not a value of the codec.
    pub invalid: bool,
    /// What the parse learned in the pass the draft committed through Enter, a separator or blur; `None` otherwise.
    pub committed: Option<craft_behavior::ParseNote>,
    /// An IME composition is under way.
    pub composing: bool,
    /// The completion candidates the list shows; empty while it is closed.
    pub completions: Vec<super::Completion>,
    /// A primary click on the text ended this pass.
    pub clicked: bool,
}

impl<V> TextOutcome<V> {
    /// An outcome over `rect` with no draft or value.
    pub const fn new(rect: Rect) -> Self {
        TextOutcome {
            rect,
            draft: None,
            value: None,
            layout: None,
            caret_visible: true,
            invalid: false,
            committed: None,
            composing: false,
            completions: Vec::new(),
            clicked: false,
        }
    }
}

/// A part as the sensing core registers it.
// F2-MERGE: F3 (`PartBuilder` adds tooltip and `pointer_only`)
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartDecl {
    pub key: PartKey,
    pub index: Option<u64>,
    pub shape: HitShape,
    pub sense: Sense,
    pub priority: HitPriority,
    pub cursor: Option<Cursor>,
    /// Drawn by `paint_over`: stacks above the body members.
    pub over: bool,
    pub flags: Flags,
    /// The part paints with the Disabled state and its data colours take the theme's disabled fill. It keeps sensing.
    pub inactive: bool,
    /// The component holds a press on the part (`PartBuilder::held`).
    pub held: bool,
    /// The hit area is the shape as declared, never grown to the pointer's target size (`PartBuilder::exact_hit`).
    pub exact: bool,
    /// The named stacking order among parts of one priority (default 0).
    pub stacking: u32,
    pub drag_to_root: bool,
}

impl PartDecl {
    /// A part of `key` over `shape` sensing `sense`.
    pub(crate) fn new(key: PartKey, shape: impl Into<HitShape>, sense: Sense) -> Self {
        PartDecl {
            key,
            index: None,
            shape: shape.into(),
            sense,
            priority: HitPriority::BODY,
            cursor: None,
            over: false,
            flags: Flags::default(),
            inactive: false,
            held: false,
            exact: false,
            stacking: 0,
            drag_to_root: false,
        }
    }
}

/// The results of a component's parts this pass, by part key and index.
pub(crate) type PartResults = HashMap<(PartKey, Option<u64>), PartResult>;
