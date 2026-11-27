//! The pass core: `CxCore`, `Cx` over it, their stage access and the inherent methods of `Cx`.

use super::parts::{PartKeys, Toggle};
use super::pointer::Pointer;
use super::scroll::ScrollMove;
use crate::contract::rt::motion::MotionSpec;
use crate::contract::rt::node::NodeInput;
use crate::contract::{Component, Resolved, Validity, ValueOf};
use crate::contract::{Context, Response};
use crate::primitives::Id;
use crate::primitives::sensing::PartResults;
use crate::primitives::{PaintRecord, TextLayout};
use crate::runtime::commands::{CommandRef, FireSource, ResolvedCommand};
use crate::runtime::env::{Env, EnvView};
use crate::runtime::focus::{self};
use crate::runtime::focus::{FocusTarget, KeepKeys};
use crate::runtime::keys::Intents;
use crate::runtime::memory::Slot;
use crate::runtime::memory::{MemoryStage, StageAccess, sealed};
use crate::style::{Ink, StateSet};
use craft_behavior::{Cursor, CursorPriority, EditEvent, EditorInput, Flow, GestureId, PartKey, PartRef, PressCancel, PressState};
use craft_keymap::Chord;
use craft_ui_rt::{CloseReason, Rect};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::ops::{Deref, Range};
use std::sync::Arc;

/// How a component was activated this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct Activation {
    /// By `Intent::Activate` (Enter, Space or an AccessKit click) rather than a primary click.
    pub by_key: bool,
    /// The modifiers held at the activation: with the click for a pointer activation, in the pass for a key or AccessKit one.
    pub mods: craft_keymap::Mods,
}

/// The text part's result of a pass as the core holds it: `Cx::text` reads the typed outcome, the response reads
/// the rest without the value type.
pub(crate) trait TextResult {
    /// The modifiers of the Enter that submitted the part this pass; none when it did not submit.
    fn submit_mods(&self) -> craft_keymap::Mods;
    /// Whether Tab (`Some(true)`) or Shift+Tab (`Some(false)`) moved focus off the part this pass.
    fn left_by_tab(&self) -> Option<bool>;
    /// Whether a `TabKey::Reports` part kept focus on Tab (`Some(true)`) or Shift+Tab (`Some(false)`) this pass.
    fn tab(&self) -> Option<bool>;
    /// The part's caret as a byte offset into its text after the pass; `None` while its widget has no caret.
    fn caret(&self) -> Option<usize>;
    /// Whether the part lost key focus this pass, edited or not.
    fn text_lost_focus(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub(crate) struct RecordedText<V> {
    pub(crate) outcome: crate::primitives::TextOutcome<V>,
    pub(crate) submit_mods: craft_keymap::Mods,
    pub(crate) left_by_tab: Option<bool>,
    pub(crate) tab: Option<bool>,
    pub(crate) caret: Option<usize>,
    pub(crate) text_lost_focus: bool,
}

impl<V: 'static> TextResult for RecordedText<V> {
    fn submit_mods(&self) -> craft_keymap::Mods {
        self.submit_mods
    }

    fn left_by_tab(&self) -> Option<bool> {
        self.left_by_tab
    }

    fn tab(&self) -> Option<bool> {
        self.tab
    }

    fn caret(&self) -> Option<usize> {
        self.caret
    }

    fn text_lost_focus(&self) -> bool {
        self.text_lost_focus
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The non-generic part of every `Cx`; the step functions take this. It is a read-only output: component code
/// changes nothing here except through a chapter trait.
#[non_exhaustive]
pub struct CxCore<'a> {
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// `Common` after step 1.
    pub resolved: &'a Resolved<'a>,
    /// The component's id.
    pub id: Id,
    /// The allocated rect.
    pub rect: Rect,
    /// The rect minus padding, mirrored for right-to-left.
    pub content_rect: Rect,
    /// The rect intersected with the clip rect.
    pub visible: Rect,
    /// Start and end geometry of the rect.
    pub flow: Flow,
    /// The node's response: the run's input facts and the node's interaction state, each fact read on its own.
    pub response: Response,
    /// The bound command, resolved at step 1.
    pub command: Option<Arc<ResolvedCommand>>,
    /// Written by step 12; read with `Styled::state`.
    pub(crate) state: StateSet,
    /// True once step 12 wrote `state` in this pass.
    pub(crate) stated: bool,
    /// True once the run reached step 7 with something to show. A measuring run, a sizing pass of the parent or host
    /// `Ui`, and a host that shows nothing (a layer in its parent's sizing pass) stop before it, and an inert run (a
    /// size the guard refuses) declares no parts: their core is blank and every part reads idle.
    pub(crate) parted: bool,
    /// Read with `Styled::ink`, valid from step 12.
    pub(crate) ink: Ink,
    /// An owned copy of the intents routed to this id.
    #[doc(hidden)]
    pub intents: Intents,
    #[doc(hidden)]
    pub memory: &'a mut MemoryStage,
    /// The node's run on craft-ui-rt: its input queue and what the run hands back to the node.
    pub(crate) run: RunIo<'a>,
    pub(crate) paint: RefCell<PaintRecord>,
    pub(crate) results: PartResults,
    /// A face reads idle results for parts it has not registered.
    pub(crate) face_mode: bool,
    /// The active part while the component holds focus.
    pub(crate) active: Option<PartRef>,
    /// What focus holds of this component, read once at step 8.
    #[doc(hidden)]
    pub focus: crate::runtime::focus::Facts,
    /// The keys a session member borrowed from its arena root and the window this pass (`KeyUse::session`).
    pub(crate) lent: crate::runtime::keys::Lent,
    pub(crate) activation: Option<Activation>,
    /// The activation `Cx::activate` injected this pass, which the pipeline runs as it runs a click.
    pub(crate) injected: Option<FireSource>,
    /// The activation leaves the layer chain open (`Cx::keep_open`).
    pub(crate) keep_open: bool,
    /// An AccessKit click named a part this pass: the response reports it as a click, as a pointer click on the part does.
    pub(crate) part_clicked: bool,
    /// For an arena root: the member activated this pass.
    pub(crate) activated_member: Option<Id>,
    pub(crate) press: PressState,
    pub(crate) cursor: Option<(Cursor, CursorPriority)>,
    /// The declared cursors of the parts hovered this pass, recorded at step 8 and written at step 16.
    pub(crate) part_cursors: Vec<Cursor>,
    pub(crate) claims: Vec<Chord>,
    pub(crate) reveal: Option<Rect>,
    /// The tooltip of the collection member the pointer rests on, which the item step leaves for step 16.
    pub(crate) member_tip: Option<crate::contract::tooltip::MemberTip>,
    /// The stops of the container's header parts at the end edge, which the body step registers after the header's
    /// children.
    pub(crate) header_stops: Vec<crate::runtime::focus::Stop>,
    pub(crate) press_cancel: Option<PressCancel>,
    /// The visible range each scroll part moved to this pass, from step 8's results.
    pub(crate) scroll_moves: Vec<ScrollMove>,
    /// The parts that show the value, whose double click resets it like the root's.
    pub(crate) value_parts: PartKeys,
    /// The value parts that declared `own_reset`.
    pub(crate) own_reset_parts: PartKeys,
    /// The parts that declared `state_layer`, which step 13 fills with the state layer.
    pub(crate) state_layer_parts: PartKeys,
    /// The parts that declared `toggles`, with whether their popup is open as of step 9.
    #[doc(hidden)]
    pub toggles: Vec<Toggle>,
    /// The part a reset targeted that declared `own_reset`; step 9 leaves its value to `update`.
    pub(crate) reset_part: Option<PartRef>,
    /// The member of the container's roving group Tab arrives at, set by the group's pick.
    pub(crate) roving_arrival: Option<Id>,
    /// The member of the container's roving group that moves start from, set by `Cx::roving_lead`.
    pub(crate) roving_lead: Option<Id>,
    /// The component joined the roving group of the body it is shown in this pass (`Cx::join_roving`).
    pub(crate) roving_member: bool,
    /// The collection's selection (a `Selection<K>`) and lead as the item step holds them, for `Cx::collection_selection`.
    pub(crate) collection_selection: Option<Box<dyn std::any::Any>>,
    /// The events and visible range the item step left for `collection_response`.
    pub(crate) collection_events: Option<Box<dyn std::any::Any>>,
    pub(crate) visible_rows: Range<usize>,
    /// The header row's facts the item step leaves for the collection's `update` and `overlay` (`Cx::header_menu`).
    pub(crate) header: crate::contract::items::HeaderFacts,
    /// The header menu's picks, which the item step applies at the end of the pass (`Cx::column_request`).
    pub(crate) column_requests: Vec<crate::contract::ColumnRequest>,
    /// What the collection's props allow, for `collection_response`.
    pub(crate) gestures: crate::contract::collection::Gestures,
    /// The text part's result of this pass, from step 8b.
    pub(crate) texts: Option<(PartKey, Box<dyn TextResult>)>,
    /// The selection parts step 7 declared, which `PaintUi::selectable_text` shows.
    pub(crate) selections: Vec<craft_behavior::PartRef>,
    /// Not enabled, with `DisabledLook::Faded`: the component and its faces paint their enabled look, which step 13
    /// fades.
    pub(crate) faded: bool,
    /// The component has an AccessKit node this pass (step 8), or as a collection member a sub-node under its
    /// collection's: its access data, its members' and its faces' have a reader.
    pub(crate) node: bool,
    /// Where the faded component's shapes start and the ground its veil takes (`steps::fade`).
    pub(crate) fade: Option<crate::contract::steps::Fade>,
    /// The focus ring shape each part declared (`focus_shape`), in which step 13 draws the active part's ring.
    pub(crate) focus_shapes: Vec<((PartKey, Option<u64>), focus::FocusShape)>,
    /// The focus ring shape the component declared for its own ring (`Cx::focus_shape`); its rect when `None`.
    pub(crate) focus_shape: Option<focus::FocusShape>,
    /// The editor part as step 8b set it up.
    pub(crate) editor: Option<crate::contract::editor::EditorInfo>,
    /// The editor part's inputs of this pass, each with its order among the pass's events, until `editor_input` or
    /// `editor_steps` takes them.
    pub(crate) editor_inputs: Vec<(crate::contract::editor::Order, craft_behavior::EditorInput)>,
    /// The text part's draft is not a value of its codec.
    pub(crate) text_invalid: bool,
    /// The text part's `TextOptions::unparsed` while its draft does not parse: the refusal the pass reports when the
    /// binding gives none.
    pub(crate) text_unparsed: Option<crate::runtime::env::Message>,
    /// The text part takes the pointer this pass: the field is in edit, or the press under way began in edit. A body
    /// scrub (`Scrub::on_field`) leaves such a press to the text.
    pub(crate) text_takes_pointer: bool,
    /// The text part's engine paint of this run, kept on the node (`ImeKept::edit`) and replayed by its paint.
    pub(crate) edit_paint: Option<craft_ui_rt::text_edit::EditPaint>,
    /// The row and column counts of the collection, which the item step records for its node.
    pub(crate) grid: crate::runtime::access::GridPlace,
    /// The text part's completion list is open: Escape belongs to its popup, not to the component's gesture.
    pub(crate) escape_to_list: bool,
    /// The text facts step 16 gives AccessKit, from the text adapter or the editor's `editor_state`.
    pub(crate) text_access: RefCell<Option<crate::runtime::access::text::TextFacts>>,
    /// The value as the component words it (`Component::value_text`), noted at step 16 or in a sizing pass for the
    /// output.
    pub(crate) value_text: RefCell<Option<String>>,
    /// Enter was pressed in the text part.
    pub(crate) submitted: bool,
    /// Escape abandoned the text part's edit.
    pub(crate) escaped: bool,
    /// The facts the collection gave this row (`rt::items::row_facts`, a tracked read); `None` outside a collection's
    /// rows.
    pub(crate) row: Option<Arc<crate::runtime::env::ItemFacts>>,
    /// The component's popups and whether each is open, borrowed from its node (`Kept::popups`): `PopupHandle` reads
    /// and changes them, the overlay pass shows the open ones.
    pub(crate) popups: &'a mut crate::contract::rt::overlay::PopupStates,
}

/// The node side of one run (or of a kept node's settled `Cx`): the input queued since the last run, and the outputs
/// the hooks hand back, which the driver keeps with the node after the run (`Kept`).
pub(crate) struct RunIo<'a> {
    /// The inputs queued since the last run, oldest first; empty on a settled `Cx`.
    pub(crate) input: &'a [NodeInput],
    /// The node did not run: `finish` gets the kept facts.
    pub(crate) settled: bool,
    /// The run is a measuring run (the parent measures its children); it reaches no node.
    pub(crate) measuring: bool,
    /// The window's context handle.
    pub(crate) context: Context,
    /// The run asked for the next Escape (`hold_escape`); kept as `FocusKept::holds_escape`.
    pub(crate) holds_escape: Cell<bool>,
    /// The opacity target and its tween (`fade_to`); aimed by the driver after the run.
    pub(crate) level: Cell<Option<(f32, MotionSpec)>>,
    /// The editor's caret rect in window coordinates (`ime_cursor`); kept as `ImeKept::caret`.
    pub(crate) ime_cursor: Cell<Option<Rect>>,
    /// Focus moves the run asked for (`request_focus`, `focus_part`, `focus_own`), in order; the driver hands them to
    /// ui-rt's focus after the run.
    pub(crate) focus_requests: RefCell<Vec<FocusTarget>>,
    /// Commands the run fired (`Cx::fire`), with their source and modifiers; the driver runs their handlers on the
    /// window's `CommandBoard` after the run, in order.
    pub(crate) fires: Vec<(CommandRef, FireSource, craft_keymap::Mods)>,
    /// The value a toggle sweep sets on this toggle in this run (`Cx::swept_toggle`), from the sweep cell.
    pub(crate) swept: Option<bool>,
    /// Live announcements the run made (`Accessible::announce`); the driver gives them to the node's access entry.
    pub(crate) announcements: Vec<(String, crate::runtime::access::Politeness)>,
    /// The node's command inboxes as of the run, and the sets a take asked a cell for (`take_command`); the
    /// driver registers the asked cells after the run.
    pub(crate) inbox: crate::runtime::commands::InboxIn,
    /// The run registered the node as its window's menu bar (`Focus::register_menu_bar`), `Some(true)` when it
    /// claims Alt alone; the driver keeps it, and the window root routes F10, a lone Alt and Alt mnemonics to the
    /// node as `NodeInput::Menu`.
    pub(crate) menu_bar: Cell<Option<bool>>,
    /// The run took its page and end moves (`Focus::take_scroll_moves`): the node owns a scroll, and the driver keeps
    /// it as the scroll owner the page and end keys its focused descendants leave bubble to.
    pub(crate) scroll_owner: Cell<bool>,
    /// The run asks to run again in the next frame (a wheel ease paying out); the driver asks the node's build.
    pub(crate) wake: Cell<bool>,
    /// The run gives up key focus (`CxCore::surrender_focus`); the driver asks ui-rt after the run.
    pub(crate) surrender: Cell<bool>,
    /// `Pointer::take_wheel` took the run's wheel steps.
    pub(crate) wheel_taken: Cell<bool>,
    /// The drag the run started (`Pointer::start_drag`); the driver starts ui-rt's payload session from the node's
    /// press with it.
    pub(crate) drag_start: RefCell<Option<crate::contract::rt::drag::DragSource>>,
    /// The earliest frame time the run asked to build again at (`CxCore::wake_at`); the driver asks the node's build
    /// for it after the run.
    pub(crate) build_at: Cell<Option<f64>>,
    /// The editor's report of this run (`Cx::editor_state`); kept on `ImeKept` after the run.
    pub(crate) editor_facts: RefCell<Option<crate::contract::rt::ime::EditorFacts>>,
    /// The editor refused an input method edit (`Cx::ime_interrupt`): the composition ends.
    pub(crate) ime_refused: Cell<bool>,
    /// The editor changed its text for a reason other than the input method (`Cx::ime_application_change`).
    pub(crate) ime_app_change: Cell<bool>,
    /// Whether the editor had something to undo and to redo at its last report (`ImeKept::history`).
    pub(crate) ime_history: (bool, bool),
    /// The node's shared slot for a body's editor reports (`EditorHandle`), made when a run first hands out a handle;
    /// kept as `ImeKept::body`.
    pub(crate) ime_body: RefCell<Option<std::sync::Arc<std::sync::Mutex<crate::contract::rt::ime::BodyReport>>>>,
    /// The pointer's place (window coordinates) over the node or a node under it when the run began (`CxCore::hover_point`).
    pub(crate) hover_point: Option<craft_ui_rt::Pos2>,
    /// The drag over the node for the payload types its last run asked for, when the run began: its payload, the
    /// pointer in window coordinates and the keys held (`Pointer::dragging`).
    pub(crate) drag_over: Option<(craft_ui_rt::DragData, craft_ui_rt::Pos2, craft_ui_rt::Mods)>,
    /// The payload types the run asked for (`Pointer::drop_target`; `true` for `window_drop_target`); the driver
    /// keeps them, and the node takes their drags in ui-rt's routing.
    pub(crate) drop_types: RefCell<smallvec::SmallVec<[(std::any::TypeId, bool); 1]>>,
    /// The run read the context requests of the nodes under it (`Cx::context_requests`); kept as
    /// `Kept::context_watch`, so its watch takes them from its next build on.
    pub(crate) context_watch: Cell<bool>,
    /// Context requests the run made at a rect of its own (`CxCore::request_context`), in order, that its context
    /// step has not taken yet: the driver passes those made after it (by item content) on.
    pub(crate) context_asks: RefCell<Vec<Rect>>,
    /// Where the context request the run's context step made opens (a secondary click, an untaken Menu key, an
    /// earlier ask); the driver queues it on the watches above.
    pub(crate) context_made: Cell<Option<Rect>>,
}

impl<'a> RunIo<'a> {
    /// The node side of a run over `input`.
    pub(crate) fn new(input: &'a [NodeInput], context: Context) -> RunIo<'a> {
        RunIo {
            input,
            settled: false,
            measuring: false,
            context,
            holds_escape: Cell::new(false),
            level: Cell::new(None),
            ime_cursor: Cell::new(None),
            focus_requests: RefCell::new(Vec::new()),
            fires: Vec::new(),
            announcements: Vec::new(),
            swept: None,
            inbox: Default::default(),
            menu_bar: Cell::new(None),
            scroll_owner: Cell::new(false),
            wake: Cell::new(false),
            surrender: Cell::new(false),
            wheel_taken: Cell::new(false),
            drag_start: RefCell::new(None),
            build_at: Cell::new(None),
            editor_facts: RefCell::new(None),
            ime_refused: Cell::new(false),
            ime_app_change: Cell::new(false),
            ime_history: (false, false),
            ime_body: RefCell::new(None),
            hover_point: None,
            drag_over: None,
            drop_types: RefCell::new(smallvec::SmallVec::new()),
            context_watch: Cell::new(false),
            context_asks: RefCell::new(Vec::new()),
            context_made: Cell::new(None),
        }
    }

    /// The node side of a kept node's `finish`: no input, nothing handed back.
    pub(crate) fn settled(context: Context) -> RunIo<'a> {
        RunIo { settled: true, ..RunIo::new(&[], context) }
    }

    /// The node side of a measuring run: no node, no input.
    pub(crate) fn measuring(context: Context) -> RunIo<'a> {
        RunIo { settled: true, measuring: true, ..RunIo::new(&[], context) }
    }
}

/// Gesture ids are unique in the process, so two nodes' gestures never share an undo step.
static NEXT_GESTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl CxCore<'_> {
    /// The inputs queued for the node since its last run, oldest first; empty on a settled `Cx`.
    pub fn input(&self) -> &[NodeInput] {
        self.run.input
    }

    /// The intents queued for the node since its last run (activation, moves, edits, typed text, Submit, Cancel ...),
    /// each taken once. Horizontal moves are read in the node's direction.
    pub fn intents(&mut self) -> &mut Intents {
        &mut self.intents
    }

    /// Takes the next Escape while the node has focus (an open draft, a running gesture): the key reaches this node's
    /// next run instead of the scopes around it. A run that does not ask leaves Escape to them.
    pub fn hold_escape(&self) {
        self.run.holds_escape.set(true);
    }

    /// Why the window was asked to close since the last run, when this component is its close guard
    /// ([`Component::close_guard`]).
    pub fn close_request(&self) -> Option<CloseReason> {
        self.run.input.iter().rev().find_map(|input| match input {
            NodeInput::Close(reason) => Some(*reason),
            _ => None,
        })
    }

    /// The node did not run: its outputs are the kept ones and its response carries no input.
    pub fn is_settled(&self) -> bool {
        self.run.settled
    }

    /// Aims the node's opacity at `target` (0 to 1): the paint phase tweens toward it over `spec`'s duration, reading
    /// the frame time while it runs, so no hook runs per frame. The first target a node names is shown at once.
    pub fn fade_to(&self, target: f32, spec: MotionSpec) {
        self.run.level.set(Some((target.clamp(0.0, 1.0), spec)));
    }

    /// A copy of the node's memory slot of type `T`, or its default.
    pub fn recall<T: Slot>(&self) -> T {
        self.memory.recall::<T>(self.id)
    }

    /// The node's editor input since its last run, in arrival order: input method preedits and commits, and the
    /// intents an editor reads (text, moves `keep` keeps, deletes, edits, Enter, Tab, Escape) that no taker took, taken.
    pub fn editor_input(&mut self, keep: KeepKeys) -> Vec<EditorInput> {
        crate::contract::rt::intents::editor_input(self.run.input, &mut self.intents, keep)
    }

    /// Reports the caret rect (window coordinates) of the node's editor: the platform places the input method's
    /// candidate window there while the node has focus.
    pub fn ime_cursor(&self, caret: Rect) {
        self.run.ime_cursor.set(Some(caret));
    }

    /// The run only measures: it shows nothing and reaches no node.
    pub fn is_measuring(&self) -> bool {
        self.run.measuring
    }

    /// The window's context, for code that takes one.
    pub fn context(&self) -> &Context {
        &self.run.context
    }

    /// Moves key focus to `target` after the run. Returns whether the target names a component (a document key
    /// resolves at the move).
    pub fn request_focus(&self, target: FocusTarget) -> bool {
        self.run.focus_requests.borrow_mut().push(target);
        true
    }

    /// A gesture id no other gesture in the process has.
    pub fn next_gesture_id(&self) -> GestureId {
        GestureId::new(NEXT_GESTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
    /// The context request of this run, from its input: `Some(part)` for a secondary click (at the part the press
    /// went down on, `None` for the component's own shape) or an untaken Menu key or Shift+F10 (at the active part);
    /// `None` without one. A settled `Cx` has no input and makes none.
    pub(crate) fn context_request(&self) -> Option<Option<PartRef>> {
        let clicked = self.run.input.iter().rev().find_map(|input| match input {
            NodeInput::Pointer {
                gesture: craft_ui_rt::Gesture::Up { click: true, .. },
                button: craft_behavior::PointerButton::Secondary,
                part,
                ..
            } => Some(*part),
            _ => None,
        });
        if let Some(part) = clicked {
            let declared = self
                .results
                .keys()
                .map(|&(key, index)| PartRef { key, index })
                .find(|declared| crate::contract::rt::node::part_id(*declared) == part);
            return Some(declared);
        }
        if self.intents.untaken(&craft_behavior::Intent::ContextMenu) {
            return Some(self.active);
        }
        // A request the run made at a rect of its own, or one its last run's item content made after its context step.
        let asked = !self.run.context_asks.borrow().is_empty()
            || self.run.input.iter().any(|input| matches!(input, NodeInput::ContextRequest { id, .. } if *id == self.id));
        asked.then_some(None)
    }

    /// Makes a context request at `rect` (window coordinates) for the component itself: a collection's Menu key at its
    /// lead row, an item's menu button. The component's `Trigger::Context` opener reads it, and the region around
    /// it gets it through `Cx::context_requests`. Made after the context step (from item content), the component
    /// runs again in this frame and makes it its own then.
    pub(crate) fn request_context(&self, rect: Rect) {
        if !self.run.settled {
            self.run.context_asks.borrow_mut().push(rect);
        }
    }
}

impl<C: Component> Cx<'_, C> {
    /// The part a reset aimed at, when it declared `own_reset`; `update` proposes the value with that part at its
    /// default. Reads `None` when the reset was the whole binding's, which step 9 applied.
    pub fn reset_part(&self) -> Option<PartRef> {
        self.core.reset_part
    }

    /// Lays document `text` out `text.wrap` wide: the call `ContentPainterCx::layout` and `MeasureCx::layout_doc`
    /// make, so `update` finds a character at a point in the galley that `paint` draws.
    pub fn layout_doc(&self, text: &crate::primitives::DocText<'_>) -> TextLayout {
        let _ = text;
        todo!(
            "Cx::layout_doc: the run lends no text services to `update` (the build is borrowed per call by RunHost); needs a lead decision, INT-1a4 Left 1"
        )
    }

    /// Whether the control's link label was clicked in this pass.
    pub fn link_activated(&self) -> bool {
        self.core.env.field_label().is_some_and(|label| label.link)
    }

    /// Asks the platform to open `url`, in a new tab where the platform has tabs and `new_tab` is set.
    pub fn open_url(&mut self, url: &str, new_tab: bool) {
        self.core.context().open_url(url, new_tab);
    }

    /// Makes this component a member of the roving group of the body it is shown in: its stop and its focusable parts
    /// join the group, and Tab visits the group once. Called from `update`: the stops register after it, in the same
    /// pass. A component that does not call it in a pass keeps its own Tab stop in that pass.
    pub fn join_roving(&mut self) {
        self.core.roving_member = true;
    }

    /// The shape the component's own focus ring takes this pass, as a part's `focus_shape` shapes the part's: the
    /// circle inside its rect, or a polygon of points on screen. Called from `update`; a component that does not call
    /// it gets the ring round its rect.
    pub fn focus_shape(&mut self, shape: focus::FocusShape) {
        self.core.focus_shape = Some(shape);
    }

    /// Runs `command` as this component's own activation would, for a command that is not the component's bound
    /// one: its handler on the window's `CommandBoard`, looked up in the node's scope chain, runs at the end of this
    /// run; a disabled command's binding is passed over there. `source` is where the activation came from
    /// (`part_activated` gives it). Returns whether the command was queued: false on a settled or measuring `Cx`.
    pub fn fire(&mut self, command: impl Into<CommandRef>, source: FireSource) -> bool {
        if self.core.run.settled {
            return false;
        }
        let mods = self.core.activation.map_or(craft_keymap::Mods::NONE, |activation| activation.mods);
        self.core.run.fires.push((command.into(), source, mods));
        true
    }

    /// Where an activation of `part` came from this pass: `Key` for a keyboard activation while `part` is the
    /// active part, `Pointer` for a click on it, `Access` for an AccessKit click on its sub-node; `None` when
    /// `part` was not activated. A part that is not declared is never activated.
    pub fn part_activated(&self, part: impl Into<PartRef>) -> Option<FireSource> {
        let part = part.into();
        let result = self.core.part_if_declared(part)?;
        if self.core.activation.is_some_and(|activation| activation.by_key) && self.core.active_now() == Some(part) {
            Some(FireSource::Key)
        } else if result.press.phase.is_click() {
            Some(FireSource::Pointer)
        } else if self.core.intents.access_click(part) {
            Some(FireSource::Access)
        } else {
            None
        }
    }

    /// The part this run's context request was made at: the part under the pointer for a secondary click, the
    /// active part for the Menu key or Shift+F10; `None` without a request or when it was made at the component
    /// itself.
    pub fn context_part(&self) -> Option<PartRef> {
        self.core.context_request().flatten()
    }

    /// Whether the component was asked for its context menu in this run: a secondary click on it, or the Menu key
    /// or Shift+F10 it did not take. Read from the run's own input, so `update`, `overlay` and `finish` agree.
    pub fn context_requested(&self) -> bool {
        self.core.context_request().is_some()
    }

    /// The context requests the components under this one made, in order, then this component's own: each component
    /// asked and the rect its menu opens at (its part's, when it was made at one). A secondary click or the Menu key
    /// on a child reaches the region around it this way, whatever the child does with its own request. Reading them
    /// makes the component watch the nodes under it: a request one of them makes runs it in the same frame, and a
    /// frame without requests runs nothing more.
    pub fn context_requests(&self) -> Vec<(Id, Rect)> {
        self.core.run.context_watch.set(true);
        let own = self.core.id;
        let mut requests: Vec<(Id, Rect)> = self
            .core
            .run
            .input
            .iter()
            .filter_map(|input| match input {
                NodeInput::ContextRequest { id, rect } if *id != own => Some((*id, *rect)),
                _ => None,
            })
            .collect();
        requests.extend(self.core.run.context_made.get().map(|rect| (own, rect)));
        requests
    }

    /// Names the member of this container's roving group (`ContainerProps.roving`) that Tab arrives at; the
    /// first member when none is named.
    pub fn roving_arrival(&mut self, member: Id) {
        self.core.roving_arrival = Some(member);
    }

    /// Names the member of this container's roving group that the next move starts from, in place of the focused one:
    /// a menu's row that the pointer lit. The group registers after `update`, so a member named from the body's state
    /// as `update` reads it takes effect from the next pass.
    #[doc(hidden)]
    pub fn roving_lead(&mut self, member: Id) {
        self.core.roving_lead = Some(member);
    }

    /// Moves focus to one of this component's focusable parts, in this pass: the rest of the pass reads the new
    /// focus. For `update` after a press on the component or while it holds focus; it never takes focus from elsewhere.
    pub fn focus_part(&mut self, part: impl Into<PartRef>) {
        let part = part.into();
        debug_assert!(
            self.core.focus.focused || self.core.press_held() || self.core.results.values().any(|result| result.press.phase.is_press()),
            "`focus_part` is for a component that holds focus or was just pressed; `update` never steals focus"
        );
        self.core.request_focus(FocusTarget::Part(self.core.id, part));
    }

    /// Moves focus to the component's own stop (not a part), under the rule of `focus_part`.
    pub fn focus_own(&mut self) {
        debug_assert!(
            self.core.focus.focused || self.core.press_held(),
            "`focus_own` is for a component that holds focus or was just pressed; `update` never steals focus"
        );
        self.core.request_focus(FocusTarget::Id(self.core.id));
    }
}

impl sealed::Sealed for CxCore<'_> {}

impl StageAccess for CxCore<'_> {
    fn stage(&self) -> &MemoryStage {
        self.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.memory
    }

    fn base(&self) -> Id {
        self.id
    }
}

/// What component code reads and calls. It derefs to [`CxCore`] for reading only.
///
/// ```compile_fail,E0594
/// # extern crate craft_ui_contract as craft_ui;
/// use craft_ui::contract::{Component, Cx};
/// fn update<C: Component>(cx: &mut Cx<'_, C>) {
///     cx.rect = craft_ui_rt::Rect::NOTHING;
/// }
/// ```
#[non_exhaustive]
pub struct Cx<'a, C: Component> {
    /// The pass core, which the pipeline builds once per run and lends to every step.
    #[doc(hidden)]
    pub core: &'a mut CxCore<'a>,
    /// The component's cached style.
    pub style: &'a C::Style,
    /// The shown copy of the value, shared with shown memory.
    pub(crate) shown: Option<Arc<ValueOf<C>>>,
    /// The binding holds a mixed value; `shown` is empty.
    #[doc(hidden)]
    pub mixed: bool,
    /// The edit events of this pass, in order; `applied` of them have reached the binding.
    pub(crate) events: Vec<EditEvent<ValueOf<C>>>,
    pub(crate) applied: usize,
    /// What the binding's check says about the shown value.
    pub(crate) validity: Validity<ValueOf<C>>,
}

impl<'a, C: Component> Deref for Cx<'a, C> {
    type Target = CxCore<'a>;

    fn deref(&self) -> &CxCore<'a> {
        self.core
    }
}

impl<C: Component> sealed::Sealed for Cx<'_, C> {}

impl<C: Component> StageAccess for Cx<'_, C> {
    fn stage(&self) -> &MemoryStage {
        self.core.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.core.memory
    }

    fn base(&self) -> Id {
        self.core.id
    }
}

/// What a composite hands a child it shows or measures: its resolved `enabled` and `size`, and the inside label of
/// the side label the child is the control of (`FieldLayout::Inside`).
#[derive(Clone)]
pub(crate) struct Under {
    pub enabled: bool,
    pub size: craft_theme::ControlSize,
    pub inside: Option<std::sync::Arc<crate::contract::InsideLabel>>,
}

impl Under {
    /// The composite's `enabled` and `size`, with no inside label.
    pub(crate) fn new(enabled: bool, size: craft_theme::ControlSize) -> Self {
        Under { enabled, size, inside: None }
    }
}

/// An env derived for children `PaintUi` builds: the composite's resolved `enabled` and `size`, and only the inside
/// label `under` gives. The composite places its children itself, so they are in no egui grid's cell.
pub(crate) fn child_env(env: &Env, under: &Under) -> Env {
    env.derive_local(|scoped| {
        scoped.enabled &= under.enabled;
        scoped.size = under.size;
        scoped.inside_label = under.inside.clone().map(|inside| inside as crate::runtime::env::Held);
        scoped.cell_fills = false;
        scoped.egui_cell = false;
    })
}
