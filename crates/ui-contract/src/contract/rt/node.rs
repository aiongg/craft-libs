//! The retained half of contract content: `ComponentNode` and `ScopeNode`, the ui-rt elements that
//! keep what the last run produced (docs/contract-rt.md sections 2 and 5).

use std::any::Any;

use craft_behavior::{EditorInput, Intent, PartRef, PressStart};
use craft_keymap::Chord;
use craft_ui_rt::{
    AccessCx, Constraints, Cursor, DisplayList, DragData, DragOutcome, DropPhase, Element, Event, EventCx, Gesture, Handled, HitShape, ImePurpose,
    InteractFact, LayoutCx, Mods, NodeId, PaintCx, PointerButton, PointerInput, Pos2, Rect, Sense, State, Vec2,
};

use crate::contract::Id;

use super::drag::{DragSource, DropRule};
use super::focus::FocusKept;
use super::keys::Taken;
use super::overlay::PopupStates;
use super::sweep::Sweep;
use super::tip::TipKept;

/// The pointer state a paint variant was recorded for (section 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// Neither hovered nor pressed.
    Rest,
    /// A mouse or pen pointer is over the node.
    Hover,
    /// The node holds a press.
    Press,
}

/// An input a node's retained half queued for its next run (section 4): step 8 senses from it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NodeInput {
    /// A press phase. A `Down` is queued without marking the node (the press look is paint-only)
    /// unless the part it went down on sets its value on press; the phases after it mark the node.
    Pointer {
        /// The phase.
        gesture: Gesture,
        /// Where the pointer is, in the node's coordinates.
        pos: Pos2,
        /// The button that pressed.
        button: PointerButton,
        /// The modifiers held.
        mods: Mods,
        /// The part of the node the press went down on: the ui-rt part id of its kept shape, 0 for
        /// the node's own shape.
        part: u64,
        /// The press, with its origin in the node's coordinates.
        start: PressStart,
    },
    /// An assistive technology's action on the node or one of its sub-nodes.
    Access(super::access::AccessAction),
    /// The drag the node started ended.
    DragEnd(DragOutcome),
    /// A payload the node takes (its drop rule, or a type its run asked for with `Pointer::drop_target`) was
    /// dropped on it: the pointer in window coordinates (`None` for an OS drop with no position) and the keys held.
    Drop {
        /// The drag's payload.
        data: DragData,
        /// Where it was dropped, in window coordinates.
        pos: Option<Pos2>,
        /// The keys held, which pick the drop operation.
        mods: Mods,
    },
    /// A drag whose payload the node's run asked for (`Pointer::drop_target`) moved over the node: the pointer in
    /// window coordinates and the keys held. Every move runs the node, so its marks follow the pointer.
    DragOver {
        /// The drag's payload.
        data: DragData,
        /// The pointer, in window coordinates.
        pos: Pos2,
        /// The keys held.
        mods: Mods,
    },
    /// An intent the key stage took for the focused node (`super::keys`): from a key press, or typed
    /// or pasted text. The run reads the queued intents through `Cx::intents`.
    Intent {
        /// The intent.
        intent: Intent,
        /// A horizontal arrow sent it: the run reads its move in the node's direction.
        horizontal: bool,
        /// Shift was held at the press.
        shift: bool,
    },
    /// A press a recorder (`OwnerClass::Recorder`) or a claim (`KeyUse::claims`) took, as a chord.
    Chord(Chord),
    /// Text the clipboard delivered to the focused text part (an editor's Paste).
    Paste(std::sync::Arc<str>),
    /// An arrow press as a nudge of the node's content (`KeyUse::nudge`): its unit direction.
    Nudge {
        /// The unit direction.
        direction: Vec2,
        /// The modifiers held.
        mods: Mods,
    },
    /// Input method input for the node's editor ([`super::ime`]): a preedit, or a commit, whose text
    /// follows as an `Intent::Text`.
    Ime(EditorInput),
    /// The node is its window's close guard and the window was asked to close.
    Close(craft_ui_rt::CloseReason),
    /// The node gained or lost key focus, and why: the next run's `Response::gained_focus` and
    /// `lost_focus`, and a text part's `left_by_tab`. Queued without marking the node.
    Focus {
        /// Gained (`true`) or lost.
        gained: bool,
        /// What moved the focus.
        cause: craft_ui_rt::FocusCause,
    },
    /// A wheel step the node's wheel shape took, in points by its `wheel_notch`.
    Wheel(craft_behavior::Wheel),
    /// A menu key the window root routed to its menu bar node (F10, a lone Alt, an Alt mnemonic).
    Menu(crate::runtime::keys::MenuKey),
    /// A reveal a child of a container that owns its body's scroll asked for, in window coordinates (`Focus::take_reveal`).
    Reveal(craft_ui_rt::Rect),
    /// A press phase a node under a collection took (a band row's node, or a part one of its children declares),
    /// queued on the collection through its [`DescendantWatch`]. Every phase but the moves of a drag.
    DescendantPress {
        /// The node that took the press.
        owner: Id,
        /// The part of `owner` the press went down on; `None` for its own shape.
        part: Option<PartRef>,
        /// The phase.
        gesture: Gesture,
        /// Where the pointer is, in the collection's coordinates.
        pos: Pos2,
        /// The sense of the pressed part (or of `owner`'s own shape).
        sense: Sense,
    },
    /// Two or more fingers tapped together on the node (ui-rt's touch tracker), around a window point:
    /// `Pointer::take_multi_tap`.
    MultiTap(craft_behavior::MultiTap),
    /// A context request a node under this one made (a secondary click, the Menu key, an item's menu request),
    /// queued through its [`DescendantWatch`] for `Cx::context_requests`. A collection also queues a request its
    /// item content made after its own context step under its own id, so its next run makes it its own request.
    ContextRequest {
        /// The node that asked.
        id: Id,
        /// Where its menu opens, in window coordinates.
        rect: Rect,
    },
}

/// The window's menu bar, as the window root provides it to every node of the window: the node that registered as
/// the bar (`Focus::register_menu_bar`), the menu keys routed to it since its last run, and the stop F10 took the
/// focus from, which Escape on the bar gives it back to and whose text part keeps its selection meanwhile.
#[derive(Clone, Default)]
pub(crate) struct MenuBar(std::rc::Rc<std::cell::RefCell<MenuBarState>>);

#[derive(Default)]
struct MenuBarState {
    /// The bar's node, and whether it registered by claim (no mnemonic letters of its own).
    bar: Option<(NodeId, bool)>,
    keys: Vec<crate::runtime::keys::MenuKey>,
    returns_to: Option<NodeId>,
}

impl MenuBar {
    /// The menu keys routed to the node since its last run, when it is the bar: they join the run's input.
    pub(crate) fn take(cx: &craft_ui_rt::BuildCx<'_>) -> Vec<NodeInput> {
        let Some(bar) = cx.provided::<MenuBar>() else { return Vec::new() };
        let mut state = bar.0.borrow_mut();
        if state.bar.is_none_or(|(own, _)| own != cx.node()) {
            return Vec::new();
        }
        state.keys.drain(..).map(NodeInput::Menu).collect()
    }

    /// After a run: registers the node as the bar (`claims`: by claim) or, when the run registered nothing, forgets it
    /// if it was the bar.
    pub(crate) fn register(registered: Option<bool>, cx: &craft_ui_rt::BuildCx<'_>) {
        let Some(bar) = cx.provided::<MenuBar>() else { return };
        let node = cx.node();
        let mut state = bar.0.borrow_mut();
        match registered {
            Some(claims) => state.bar = Some((node, claims)),
            None if state.bar.is_some_and(|(own, _)| own == node) => state.bar = None,
            None => {}
        }
    }

    /// The stop the bar took the focus from, while the focus is on the bar or in what it opened (`within`).
    pub(crate) fn returns_to(&self, within: impl Fn(NodeId) -> bool) -> Option<NodeId> {
        let state = self.0.borrow();
        state.bar.filter(|(bar, _)| within(*bar)).and(state.returns_to)
    }

    /// The window root's menu keys, off the focused node's path (spec 9.7): F10 on every OS, and Alt with a letter off
    /// a Mac for a bar with mnemonics of its own, reach the bar as `NodeInput::Menu`. F10 moves the focus to the bar
    /// and remembers the stop it left; Escape with the focus on the bar itself gives it back. A lone Alt needs a
    /// platform signal ui-rt does not have yet.
    pub(crate) fn key(cx: &mut EventCx<'_>, k: &craft_ui_rt::KeyInput) -> Handled {
        let Some(menu) = cx.provided::<MenuBar>() else { return Handled::No };
        let Some((bar, claims)) = menu.0.borrow().bar else { return Handled::No };
        if !k.down || cx.window_rect(bar).is_none() {
            return Handled::No;
        }
        let plain = !(k.mods.cmd || k.mods.ctrl || k.mods.alt || k.mods.shift);
        let mac = cx.provided::<crate::contract::context::Context>().is_some_and(|ctx| ctx.os().is_mac());
        let focused = cx.focused();
        let routed = match k.key {
            craft_keymap::Key::F10 if plain => {
                if focused != Some(bar) {
                    menu.0.borrow_mut().returns_to = focused;
                    cx.focus_node(bar);
                }
                crate::runtime::keys::MenuKey::Focus
            }
            craft_keymap::Key::Escape if plain && focused == Some(bar) => {
                let back = menu.0.borrow_mut().returns_to.take();
                match back.filter(|to| cx.window_rect(*to).is_some()) {
                    Some(to) => cx.focus_node(to),
                    None => cx.blur(),
                }
                return Handled::Yes;
            }
            key if key.is_letter() && k.mods.alt && !(k.mods.cmd || k.mods.ctrl) && !mac && !claims => match key.name().chars().next() {
                Some(letter) => crate::runtime::keys::MenuKey::Letter(letter.to_ascii_lowercase()),
                None => return Handled::No,
            },
            _ => return Handled::No,
        };
        menu.0.borrow_mut().keys.push(routed);
        cx.request_build_of(bar);
        Handled::Yes
    }
}

/// What the nodes under a node do that it watches, for its next run: the presses its rows and their parts take (a
/// collection, [`NodeInput::DescendantPress`]) and the context requests they make (a node whose run reads
/// `Cx::context_requests`, [`NodeInput::ContextRequest`]). The node provides it to what it shows; a node under it
/// queues what the watch takes and marks it, so a press a row's part wins still moves the collection's lead, and a
/// secondary click on a control reaches the region around it.
#[derive(Clone)]
pub(crate) struct DescendantWatch {
    node: NodeId,
    shared: std::rc::Rc<WatchShared>,
}

#[derive(Default)]
struct WatchShared {
    /// The watch takes presses (a collection).
    presses: std::cell::Cell<bool>,
    /// The watch takes context requests (the last run read `Cx::context_requests`).
    context: std::cell::Cell<bool>,
    queue: std::cell::RefCell<Vec<NodeInput>>,
}

impl DescendantWatch {
    /// Provides the node's watch to what it shows in this build, when it watches anything: `presses` for a collection,
    /// context requests when its last run read them (`Kept::context_watch`). A container provides it before its body
    /// builds, a collection before its item content.
    pub(crate) fn provide(kept: &mut Kept, cx: &mut craft_ui_rt::BuildCx<'_>, presses: bool) {
        let context = kept.context_watch;
        if let Some(watch) = &kept.watch {
            // A provided watch that takes nothing any more is passed over by the nodes under it.
            watch.shared.presses.set(presses);
            watch.shared.context.set(context);
        }
        if !(presses || context) {
            return;
        }
        let node = cx.node();
        let watch = kept.watch.get_or_insert_with(|| DescendantWatch { node, shared: Default::default() });
        watch.shared.presses.set(presses);
        watch.shared.context.set(context);
        let watch = watch.clone();
        cx.provide(watch);
    }

    /// What was queued since the node's last run, oldest first.
    pub(crate) fn take(kept: &Kept) -> Vec<NodeInput> {
        kept.watch.as_ref().map(|watch| std::mem::take(&mut *watch.shared.queue.borrow_mut())).unwrap_or_default()
    }

    /// After a build of the watching node that left something queued (a request a node in its body or item content
    /// made while it built, which marking would not have built again): the node builds again in this frame and takes
    /// it as input. Nothing is queued in a frame without requests.
    pub(crate) fn after_build(kept: &Kept, cx: &mut craft_ui_rt::BuildCx<'_>) {
        if kept.watch.as_ref().is_some_and(|watch| !watch.shared.queue.borrow().is_empty()) {
            cx.rebuild();
        }
    }

    /// Queues a press phase the node `cx` handles on every collection above it, nearest first, so a press inside a
    /// nested collection (on its own shape or on one of its rows) still moves each outer collection's lead. A
    /// collection also queues the presses on its own parts (a section row's part) on its own watch, as it sees the
    /// presses on its rows' parts.
    fn note(kept: &Kept, cx: &mut EventCx<'_>, p: &PointerInput, part: u64) {
        if matches!(p.gesture, Gesture::Drag { .. }) {
            return;
        }
        let pressed = kept.parts.iter().find(|kp| part != 0 && kp.id == part).map(|kp| kp.part);
        let sense = kept.sense_of(part);
        if pressed.is_some()
            && let Some(own) = kept.watch.as_ref().filter(|own| own.shared.presses.get())
        {
            own.queue_press(cx, kept.id, pressed, p, sense);
        }
        let mut from = cx.node();
        while let Some(watch) = cx.provided_above::<DescendantWatch>(from) {
            if watch.shared.presses.get() {
                watch.queue_press(cx, kept.id, pressed, p, sense);
            }
            from = watch.node;
        }
    }

    /// Queues one press phase `owner` took on this watch's collection and marks it to build.
    fn queue_press(&self, cx: &mut EventCx<'_>, owner: Id, part: Option<PartRef>, p: &PointerInput, sense: Sense) {
        let Some(origin) = cx.window_rect(self.node).map(|rect| rect.min) else { return };
        let input = NodeInput::DescendantPress { owner, part, gesture: p.gesture, pos: Pos2::ZERO + (p.pos - origin), sense };
        self.shared.queue.borrow_mut().push(input);
        cx.request_build_of(self.node);
    }

    /// Queues the context requests the node `cx` builds made in its run on every watch above it that takes them,
    /// nearest first. A watcher whose build is under way (the request came from its body) takes them in that run or,
    /// when the run is past its input, in the build [`Self::after_build`] asks for; any other is marked.
    pub(crate) fn note_context(cx: &mut craft_ui_rt::BuildCx<'_>, requests: &[(Id, Rect)]) {
        if requests.is_empty() {
            return;
        }
        let mut from = cx.node();
        while let Some(watch) = cx.provided_above::<DescendantWatch>(from) {
            if watch.shared.context.get() {
                let mut queue = watch.shared.queue.borrow_mut();
                queue.extend(requests.iter().map(|&(id, rect)| NodeInput::ContextRequest { id, rect }));
                drop(queue);
                if !cx.is_building(watch.node) {
                    cx.request_build_of(watch.node);
                }
            }
            from = watch.node;
        }
    }
}

/// The room a node was shown in. A node shown in other room runs again.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Place {
    pub(crate) room: Vec2,
}

/// A child's place in its owner, as the owner's build laid it out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) node: NodeId,
    pub(crate) offset: Vec2,
    pub(crate) size: Vec2,
}

/// One recorded paint of a node: its lists under and over its children.
#[derive(Clone, Debug)]
pub(crate) struct Variant {
    pub(crate) look: Pointer,
    pub(crate) content: DisplayList,
    pub(crate) overlay: DisplayList,
}

/// The rooms a node's size cache keeps: a measuring run's and the real run's, and one more for a
/// fill whose share moved.
const SIZE_CACHE_ROOMS: usize = 3;

/// A part `parts` declared, kept as a hit shape in the node's coordinates. Paint records it with
/// `id` as its ui-rt part id, so the presses that go down on it carry that id.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeptPart {
    pub(crate) part: PartRef,
    pub(crate) id: u64,
    pub(crate) shape: HitShape,
    pub(crate) sense: Sense,
    pub(crate) cursor: Option<Cursor>,
    /// What the operating system does with a press on the part (a title bar's drag area, a caption
    /// button, a resize edge).
    pub(crate) exclusive: Option<crate::runtime::windows::WindowHit>,
}

/// The ui-rt part id of `part`: never 0, which names the node's own shape.
pub(crate) fn part_id(part: PartRef) -> u64 {
    craft_ui_rt::hash_key(&part) | 1
}

/// What the last run of a node produced; the `Local` of both node elements.
pub(crate) struct Kept {
    /// The craft `Id` services key by.
    pub(crate) id: Id,
    /// The component key after the last run's writes; `None` before the first and for unkeyed types.
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) sense: Sense,
    /// The label's rect, in node coordinates, a control that draws only its mark offers as part of its hit target
    /// (`FieldLabel::claim`): it takes the node's own sense and lies outside the node's bounds.
    pub(crate) claim: Option<craft_ui_rt::Rect>,
    /// Empty until the first run; one entry when the paint read no pointer state.
    pub(crate) variants: smallvec::SmallVec<[Variant; 1]>,
    pub(crate) parts: Vec<KeptPart>,
    pub(crate) access: Option<accesskit::Node>,
    /// The sub-nodes of the access node; empty with it.
    pub(crate) access_subs: Vec<super::access::BuiltSub>,
    pub(crate) queue: Vec<NodeInput>,
    pub(crate) placed: Vec<Placed>,
    /// A scope's returned value.
    pub(crate) value: Option<Box<dyn Any>>,
    /// The node size cache: the size the node takes in each room it was measured or shown in since
    /// its last run. It holds while the node keeps its key and carries no mark: a measuring run
    /// asks a kept node for its size here instead of running it.
    pub(crate) sizes: smallvec::SmallVec<[(Vec2, Vec2); 2]>,
    /// A container's last measurement of its body, reused while its key, room and content are
    /// unchanged.
    pub(crate) plan: Option<super::container::Plan>,
    /// A container node's body state (open state of a collapsing body).
    pub(crate) body: super::container::BodyKept,
    /// Whether the last real run of the body under this node met fill children; `None` before it.
    pub(crate) body_fills: Option<bool>,
    /// The node's memory slots (`Remember`, the pipeline's records), freed with the node.
    pub(crate) memory: crate::runtime::memory::MemoryStage,
    /// The window's toggle sweep record: the window root creates it, every run copies it from the
    /// flow it was shown in.
    pub(crate) sweep: Option<State<Sweep>>,
    /// The value a sweeping toggle showed at its last run (`Component::sweep_value`).
    pub(crate) toggle: Option<bool>,
    /// The press the node owns, from its `Down` to its release or cancel.
    pub(crate) press: Option<PressStart>,
    /// A drag of the node's press runs, as of the inputs its runs consumed.
    pub(crate) dragging: bool,
    /// The component's popups and whether each is open.
    pub(crate) popups: PopupStates,
    /// The component's tooltip and where it stands.
    pub(crate) tip: Option<TipKept>,
    /// What a drag from the component carries, from its last run.
    pub(crate) drag: Option<DragSource>,
    /// The payloads the component takes as drops, from its last run.
    pub(crate) drop: Option<DropRule>,
    /// A ui-rt drag session the node started is running: its press moves are the session's, not
    /// input for a run.
    pub(crate) drag_session: bool,
    /// The component answers its window's close requests, from its last run.
    pub(crate) close_guard: bool,
    /// The env cells the node's window root provided (window root only).
    pub(crate) env: Option<super::env::EnvCells>,
    /// The node's entry in its window's `Id -> NodeId` map ([`super::ids`]); it leaves the map with the node.
    pub(crate) ids: Option<super::ids::IdEntry>,
    /// The ui-rt part id of the part the open press went down on, 0 for the node's own shape.
    pub(crate) press_part: u64,
    /// The node is a window root scope: it owns the window's Tab and F6 order.
    pub(crate) window_root: bool,
    /// The command board the host gave the window root (window root only).
    pub(crate) commands: Option<super::commands::CommandBoard>,
    /// The cursor over the node's own shape, from `Component::cursor`.
    pub(crate) cursor: Option<Cursor>,
    /// The distance of one wheel line over the node (`Component::wheel_notch`).
    pub(crate) notch: craft_behavior::WheelNotch,
    /// The node registered as its window's menu bar (`Focus::register_menu_bar`), `Some(true)` when it claims a
    /// lone Alt: the window root routes F10, Alt and Alt mnemonics to it as `NodeInput::Menu`.
    pub(crate) menu_bar: Option<bool>,
    /// The window's menu bar record (window root only), provided to the window's nodes.
    pub(crate) menu: Option<MenuBar>,
    /// The window's pending focus part (window root only), provided to the window's nodes.
    pub(crate) focus_part: Option<super::focus::PendingPart>,
    /// The component's type name and bound command, from its last run (`runtime::inspect` records them).
    pub(crate) type_name: &'static str,
    pub(crate) command: Option<craft_keymap::CommandRef>,
    /// Paint-time state: the fade between pointer looks.
    pub(crate) fade: super::motion::FadeCell,
    /// The opacity a run aimed at and its tween, evaluated at paint time.
    pub(crate) level: std::cell::RefCell<Option<super::motion::Level>>,
    /// The node's stop, scope and roving group, and its focus ring.
    pub(crate) focus: FocusKept,
    /// The node's input method state: the composition and the caret rect the platform places its
    /// candidate window at.
    pub(crate) ime: super::ime::ImeKept,
    /// A members collection's rows: one sub-list per row and pointer look (`rt/items.rs`).
    pub(crate) members: Vec<super::items::MemberLooks>,
    /// The column group reports the node's last build or run made, handed to the open groups again while it is kept.
    pub(crate) column_reports: Vec<crate::layout::Report>,
    /// The payload types the node's last run asked for (`Pointer::drop_target`, `window_drop_target`: `true` for a
    /// window-wide target): the node records drop shapes for them and takes their drags in ui-rt's routing.
    pub(crate) drop_types: smallvec::SmallVec<[(std::any::TypeId, bool); 1]>,
    /// A target of the node's last run takes OS file drops under a modal layer (`DropTarget::under_modal`).
    pub(crate) drop_under_modal: bool,
    /// The drag over the node for those types, as the last offer left it: its payload, the pointer in window
    /// coordinates and the keys held. Cleared when it leaves or drops.
    pub(crate) drag_over: Option<(DragData, Pos2, Mods)>,
    /// The node's last run took its page and end moves (`Focus::take_scroll_moves`): the page and end keys a focused
    /// descendant leaves bubble to it and queue as moves for its next run.
    pub(crate) scroll_owner: bool,
    /// The node's watch over the presses and context requests of the nodes under it ([`DescendantWatch`]).
    pub(crate) watch: Option<DescendantWatch>,
    /// The node's last run read the context requests of the nodes under it (`Cx::context_requests`): its watch takes
    /// them from its next build on.
    pub(crate) context_watch: bool,
    /// The node's control folds into its owner's access node: its entries leave the window's folds with the node.
    pub(crate) fold: Option<crate::runtime::access::fold::FoldEntry>,
}

impl Kept {
    pub(crate) fn new() -> Kept {
        Kept {
            id: Id::NULL,
            ids: None,
            press_part: 0,
            key: None,
            place: Place::default(),
            size: Vec2::ZERO,
            sense: Sense::empty(),
            claim: None,
            variants: smallvec::SmallVec::new(),
            parts: Vec::new(),
            access: None,
            access_subs: Vec::new(),
            type_name: "",
            command: None,
            queue: Vec::new(),
            placed: Vec::new(),
            value: None,
            sizes: smallvec::SmallVec::new(),
            plan: None,
            body: super::container::BodyKept::default(),
            body_fills: None,
            memory: Default::default(),
            sweep: None,
            toggle: None,
            press: None,
            popups: PopupStates::new(),
            tip: None,
            drag: None,
            drop: None,
            drag_session: false,
            close_guard: false,
            dragging: false,
            env: None,
            window_root: false,
            commands: None,
            cursor: None,
            notch: craft_behavior::WheelNotch::DEFAULT,
            menu_bar: None,
            menu: None,
            focus_part: None,
            fade: Default::default(),
            level: Default::default(),
            focus: FocusKept::default(),
            ime: Default::default(),
            members: Vec::new(),
            column_reports: Vec::new(),
            watch: None,
            context_watch: false,
            fold: None,
            scroll_owner: false,
            drop_types: smallvec::SmallVec::new(),
            drop_under_modal: false,
            drag_over: None,
        }
    }

    /// The size cached for `room`.
    pub(crate) fn cached(&self, room: Vec2) -> Option<Vec2> {
        self.sizes.iter().find(|(r, _)| *r == room).map(|(_, size)| *size)
    }

    /// Caches `size` for `room`, keeping the newest few rooms.
    pub(crate) fn cache(&mut self, room: Vec2, size: Vec2) {
        if let Some(entry) = self.sizes.iter_mut().find(|(r, _)| *r == room) {
            entry.1 = size;
            return;
        }
        if self.sizes.len() == SIZE_CACHE_ROOMS {
            self.sizes.remove(0);
        }
        self.sizes.push((room, size));
    }

    /// Places the children where the last build laid them out, moved by `shift` (a scroll body's
    /// offset); a child keeps its size.
    pub(crate) fn place_children(&self, cx: &mut LayoutCx<'_>, shift: Vec2) {
        for p in &self.placed {
            cx.layout_child(p.node, Constraints::tight(p.size));
            cx.place_child(p.node, p.offset - shift);
        }
    }

    /// The sense of the node's own shape (`part` 0) or of the kept part `part`.
    fn sense_of(&self, part: u64) -> Sense {
        if part == 0 { self.sense } else { self.parts.iter().find(|p| p.id == part).map_or(Sense::empty(), |p| p.sense) }
    }

    fn takes_press(&self) -> bool {
        self.sense.takes_press() || self.parts.iter().any(|p| p.sense.takes_press())
    }

    /// The node or one of its parts takes a wheel axis.
    fn takes_wheel(&self) -> bool {
        let wheel = Sense::WHEEL | Sense::WHEEL_X | Sense::WHEEL_Y | Sense::WHEEL_ZOOM;
        !self.sense.intersection(wheel).is_empty() || self.parts.iter().any(|p| !p.sense.intersection(wheel).is_empty())
    }
}

/// A shown component (leaf or container). Its hooks run in its owner's build; this element replays
/// their outputs in ui-rt's phases.
pub(crate) struct ComponentNode {
    pub(crate) key: u64,
    /// The toggle sweep group of a sweeping toggle (its flow's id), 0 otherwise.
    pub(crate) tag: u64,
    /// The component is a Tab stop (`Component::keys`): ui-rt focuses it on a press and falls back
    /// to it when focus inside it goes.
    pub(crate) stop: bool,
    /// The component takes text input while focused (`Component::text_input`): ui-rt runs a
    /// platform IME session for it.
    pub(crate) text: Option<ImePurpose>,
}

impl Element for ComponentNode {
    type Output = ();
    type Local = Kept;

    fn props_key(&self) -> u64 {
        self.key
    }

    fn init(&self, _: &mut craft_ui_rt::BuildCx<'_>) -> Kept {
        Kept::new()
    }

    fn layout(&self, kept: &mut Kept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        kept.place_children(cx, Vec2::ZERO);
        kept.size
    }

    /// Its size is decided in the build.
    fn relayout_boundary(&self) -> bool {
        true
    }

    fn tag(&self) -> u64 {
        self.tag
    }

    fn focusable(&self) -> bool {
        self.stop
    }

    fn text_input(&self) -> Option<ImePurpose> {
        self.text
    }

    /// The caret rect the last run reported (`Cx::ime_cursor`), in the node's coordinates.
    fn ime_cursor(&self, kept: &Kept) -> Option<Rect> {
        kept.ime.caret()
    }

    fn ime_surrounding(&self, kept: &Kept, since: Option<u64>) -> Option<craft_ui_rt::ImeSurrounding> {
        kept.ime.surrounding_since(since)
    }

    /// Shows the kept looks for the node's pointer state (reading it only when the variants differ),
    /// fades between them and tweens the node's opacity at the frame's time ([`super::motion`]), and
    /// re-records the kept hit shapes: the node's own, then its parts above it.
    fn paint(&self, kept: &Kept, cx: &mut PaintCx<'_>) {
        // Each fact is read only when a look for it was recorded, so a press does not repaint a node
        // that keeps no press look, and a single look reads nothing.
        let has = |look: Pointer| kept.variants.iter().any(|v| v.look == look);
        let look = if has(Pointer::Press) && cx.interaction_fact(InteractFact::Pressed) {
            Pointer::Press
        } else if has(Pointer::Hover) && cx.interaction_fact(InteractFact::Hovered) {
            Pointer::Hover
        } else {
            Pointer::Rest
        };
        super::motion::paint_level(kept, cx);
        super::motion::paint_looks(kept, cx, look);
        if !kept.members.is_empty() {
            // Member rows: each row's sub-list for the pointer's place; the read flips only when the
            // hovered row changes, and the press is read only while a row is hovered.
            let hovered = cx.hovered_part();
            let pressed = hovered.is_some() && cx.interaction_fact(InteractFact::Pressed);
            for list in super::items::member_lists(&kept.members, hovered, pressed) {
                cx.extend(list);
            }
        }
        if let Some(edit) = &kept.ime.edit {
            edit.paint(cx);
        }
        super::cursor::record_own_shape(kept, cx);
        if let Some(claim) = kept.claim.filter(|_| !kept.sense.is_empty()) {
            cx.hit(HitShape::Rect(claim), kept.sense, 0);
        }
        if kept.close_guard {
            cx.close_guard();
        }
        for p in kept.parts.iter().chain(kept.members.iter().flat_map(|row| &row.parts)) {
            if let Some(hit) = p.exclusive {
                cx.window_zone(p.shape, super::windows::zone_tag(hit));
            }
            match p.cursor {
                Some(cursor) => cx.hit_cursor(p.shape, p.sense, p.id, cursor),
                None => cx.hit(p.shape, p.sense, p.id),
            }
        }
        if kept.drop.is_some() || kept.drop_types.iter().any(|(_, window)| !window) {
            let shape = HitShape::Rect(cx.bounds());
            match kept.drop_under_modal {
                true => cx.drop_target_under_modal(shape, 0),
                false => cx.drop_target(shape, 0),
            }
        }
        if kept.drop_types.iter().any(|(_, window)| *window) {
            match kept.drop_under_modal {
                true => cx.window_drop_target_under_modal(),
                false => cx.window_drop_target(),
            }
        }
        // The ring reads only the focus-visible fact: a focus move repaints the node, a hover does not.
        if let Some(ring) = kept.focus.ring
            && cx.interaction_fact(InteractFact::FocusVisible)
        {
            let bounds = cx.bounds();
            kept.focus.ring_shape.draw(cx, ring, bounds);
        }
    }

    /// Queues what the next run consumes and marks the node, which marks the path to its window
    /// root. A press's `Down` is queued without marking unless the part it went down on sets its
    /// value on press. Keys and focus requests go through the key stage and the focus service.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Focus { gained: on, cause } => {
                if self.stop {
                    kept.queue.push(NodeInput::Focus { gained: *on, cause: *cause });
                }
                if *on {
                    kept.focus.took_focus();
                    // A move from another node that named one of this node's parts.
                    if let Some(part) = super::focus::PendingPart::take(cx) {
                        kept.focus.part = Some(part);
                    }
                } else {
                    super::ime::focus_left(kept, cx);
                }
                // The window's edit commands resolve against the focused stop's edits (#3569). A move
                // clears them on the node that loses focus, then the node that gains it writes its own.
                if let Some(ctx) = cx.provided::<crate::contract::context::Context>() {
                    let edits = if *on { super::commands::FocusedEdits::of(kept.focus.stop) } else { super::commands::FocusedEdits::NONE };
                    super::commands::FocusedEdits::set(&ctx, cx, edits);
                }
                Handled::Yes
            }
            Event::Text(_) | Event::ImePreedit { .. } | Event::ImeCommit(_) | Event::Paste(_) => super::ime::event(self.text.is_some(), kept, cx, ev),
            Event::Key(k) => self.key(kept, cx, k),
            Event::Access(a) if a.target == cx.node() && a.action == accesskit::Action::Focus && a.sub.is_none() => super::focus::request(kept, cx),
            Event::Hover(on) => {
                if let Some(tip) = kept.tip.as_mut() {
                    tip.hover(*on, cx);
                }
                Handled::Yes
            }
            Event::MultiTap(tap) => {
                kept.queue.push(NodeInput::MultiTap(*tap));
                cx.request_build();
                Handled::Yes
            }
            Event::HoverPart(part) => {
                if let Some(tip) = kept.tip.as_mut() {
                    tip.hover_part(*part, cx);
                }
                Handled::Yes
            }
            Event::CloseRequested(reason) => {
                kept.queue.push(NodeInput::Close(*reason));
                cx.request_build();
                Handled::Yes
            }
            Event::DragEnd(outcome) => {
                kept.drag_session = false;
                kept.queue.push(NodeInput::DragEnd(*outcome));
                cx.request_build();
                Handled::Yes
            }
            Event::Drop(d) => {
                let rule = kept.drop.filter(|rule| rule.accepts(&d.data));
                // A type the run asked for: the run decides from the pointer what the drop does, so every move reaches it.
                let asked = kept.drop_types.iter().any(|(ty, _)| super::drag::carries(&d.data, *ty));
                if rule.is_none() && !asked {
                    return Handled::No;
                }
                match d.phase {
                    DropPhase::Over => {
                        if asked && let Some(pos) = d.pos {
                            kept.drag_over = Some((d.data.clone(), pos, d.mods));
                            kept.queue.push(NodeInput::DragOver { data: d.data.clone(), pos, mods: d.mods });
                            cx.request_build();
                        }
                        let op = rule.map_or_else(|| cx.drop_operation(craft_behavior::CopyKeys::Alt), |rule| rule.op);
                        cx.accept_drop(op)
                    }
                    DropPhase::Leave => {
                        if kept.drag_over.take().is_some() {
                            cx.request_build();
                        }
                        Handled::Yes
                    }
                    DropPhase::Drop => {
                        kept.drag_over = None;
                        kept.queue.push(NodeInput::Drop { data: d.data.clone(), pos: d.pos, mods: d.mods });
                        cx.request_build();
                        Handled::Yes
                    }
                }
            }
            Event::Wheel(w) if kept.takes_wheel() => {
                kept.queue.push(NodeInput::Wheel(wheel_of(w, kept.notch, kept.size)));
                cx.request_build();
                Handled::Yes
            }
            Event::Pointer(p) if kept.takes_press() => {
                self.pointer(kept, cx, p);
                Handled::Yes
            }
            Event::Access(a) if a.target == cx.node() && !matches!(a.action, accesskit::Action::Focus | accesskit::Action::ScrollIntoView) => {
                if a.action == accesskit::Action::Click && !kept.takes_press() && a.sub.is_none() {
                    return Handled::No;
                }
                let action = super::access::AccessAction { action: a.action, data: a.data.clone(), sub: a.sub };
                let input = crate::runtime::access::text::selection_input(&kept.ime.text_runs, &action);
                kept.queue.push(input.unwrap_or(NodeInput::Access(action)));
                cx.request_build();
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, kept: &Kept, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        for s in &kept.access_subs {
            let mut sub = s.node.clone();
            kept.resolve_relations(&mut sub);
            cx.sub(s.key, s.parent, sub);
        }
        let mut node = kept.access.clone()?;
        kept.resolve_relations(&mut node);
        Some(node)
    }
}

impl ComponentNode {
    /// A key on this node's path: taken when the node is focused and its kept `KeyUse` says so,
    /// then by its roving group or its trapping scope.
    fn key(&self, kept: &mut Kept, cx: &mut EventCx<'_>, k: &craft_ui_rt::KeyInput) -> Handled {
        if cx.target() == cx.node() {
            // While the input method composes, its keys are the composition's.
            if kept.ime.composing() {
                return Handled::Yes;
            }
            match super::keys::take(&kept.focus, cx, k) {
                Taken::Queue(input) => {
                    kept.queue.push(input);
                    cx.request_build();
                    return Handled::Yes;
                }
                Taken::Held => return Handled::Yes,
                Taken::No => {}
            }
            return super::focus::trap_tab(kept, false, cx, k);
        }
        if let Handled::Yes = super::focus::rove(kept, cx, k) {
            return Handled::Yes;
        }
        if kept.scroll_owner
            && let Some(input) = super::keys::scroll_move(cx, k)
        {
            kept.queue.push(input);
            cx.request_build();
            return Handled::Yes;
        }
        super::focus::trap_tab(kept, false, cx, k)
    }

    /// The press machine's phases on this node. `Down` takes the press; a sweeping toggle's drag
    /// runs the sweep instead of queuing moves.
    fn pointer(&self, kept: &mut Kept, cx: &mut EventCx<'_>, p: &PointerInput) {
        let pos = cx.to_local(p.pos);
        let part = p.hit.filter(|h| h.node == cx.node()).map_or(0, |h| h.part);
        let start = match p.gesture {
            Gesture::Down { clicks } => {
                let start = PressStart::new(pos, p.button, cx.time(), clicks);
                kept.press = Some(start);
                kept.press_part = part;
                start
            }
            _ => kept.press.unwrap_or_else(|| PressStart::new(pos, p.button, cx.time(), 1)),
        };
        match p.gesture {
            Gesture::Down { .. } => {
                super::ime::pressed(kept, cx);
                if let Some(tip) = kept.tip.as_mut() {
                    tip.pressed(cx);
                }
            }
            Gesture::DragStart { .. } if kept.drag.is_some() => {
                let tokens = cx.env().tokens;
                let tokens = tokens.get(cx);
                if let Some(source) = &kept.drag {
                    kept.drag_session = cx.start_drag(source.start(&tokens));
                }
            }
            // The session moves the preview; the source does not run per move.
            Gesture::Drag { .. } if kept.drag_session => return,
            _ => {}
        }
        DescendantWatch::note(kept, cx, p, part);
        if super::sweep::on_pointer(self.tag, kept, cx, p) {
            return;
        }
        let marks = match p.gesture {
            Gesture::Down { .. } => kept.sense_of(part).contains(Sense::SETS_ON_PRESS),
            Gesture::Up { .. } | Gesture::Cancel(_) => {
                kept.press = None;
                true
            }
            Gesture::DragStart { .. } | Gesture::Drag { .. } | Gesture::LongPress => true,
        };
        kept.queue.push(NodeInput::Pointer { gesture: p.gesture, pos, button: p.button, mods: p.mods, part, start });
        if marks {
            cx.request_build();
        }
    }
}

/// A scope: the window root's content, a `MemoView`, a container's region. It keeps its returned value and its
/// children's places; it paints nothing and has no access node of its own. With `clip` it is as large as its parent
/// places it and clips its children to that (a collapsing body shows the moving share of its height).
pub(crate) struct ScopeNode {
    pub(crate) key: u64,
    pub(crate) clip: bool,
}

impl Element for ScopeNode {
    type Output = ();
    type Local = Kept;

    fn props_key(&self) -> u64 {
        self.key ^ u64::from(self.clip).rotate_right(1)
    }

    fn init(&self, _: &mut craft_ui_rt::BuildCx<'_>) -> Kept {
        Kept::new()
    }

    fn layout(&self, kept: &mut Kept, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Vec2 {
        kept.place_children(cx, Vec2::ZERO);
        if self.clip { constraints.constrain(kept.size) } else { kept.size }
    }

    fn paint(&self, _: &Kept, cx: &mut PaintCx<'_>) {
        if self.clip {
            let bounds = cx.bounds();
            cx.clip(bounds);
        }
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    /// The window root is the last claimant on every key's path: Tab and F6, then the command the
    /// key is bound to.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Key(k) if kept.window_root => match super::focus::window_keys(kept, cx, k) {
                Handled::No => match MenuBar::key(cx, k) {
                    Handled::No => super::commands::window_command(kept, cx, k),
                    handled => handled,
                },
                handled => handled,
            },
            _ => Handled::No,
        }
    }
}

/// A wheel step as the node's run reads it (`Pointer::take_wheel`): line steps in points of the
/// node's notch, a two-finger gesture as touch pan, zoom and rotation.
fn wheel_of(w: &craft_ui_rt::WheelInput, notch: craft_behavior::WheelNotch, extent: Vec2) -> craft_behavior::Wheel {
    let (scroll, lines) = match w.unit {
        craft_behavior::WheelUnit::Line => (Vec2::new(w.delta.x * notch.points(extent.x), w.delta.y * notch.points(extent.y)), w.delta),
        _ => (w.delta, Vec2::ZERO),
    };
    let mut wheel = craft_behavior::Wheel::new(if w.touch { Vec2::ZERO } else { scroll }, w.mods, w.unit, w.phase);
    wheel.lines = lines;
    wheel.source = w.source;
    wheel.at = Some(w.pos);
    if w.touch {
        wheel.touch_pan = w.delta;
        wheel.touch_zoom = w.zoom;
        wheel.touch_rotation = w.rotation;
    } else {
        wheel.zoom = w.zoom;
        wheel.rotation = w.rotation;
    }
    wheel
}
