# The component contract on craft-ui-rt

This document describes how craft's component contract (`craft-ui-contract`: `Component`, `Container`, `Collection`, `MemoView`, `Lazy`, the 17-step pipeline and the services) runs on the retained runtime `craft-ui-rt`. It is the design the bridge `crates/ui-contract/src/contract/rt/*` implements and the reference for the tasks that port the pipeline and the services (B2 to B11 of the spike plan). Section 3.6 of [craft-framework.md](craft-framework.md) states the run model in short.

Terms: a *run* of a component is one execution of its pipeline hooks. A *node* is a node of the ui-rt tree. A *scope* is a node whose content is a closure the app passes (the window root, a `MemoView`, a shell slot, a `Lazy`, a layer child). A node is *kept* when it does not run in a frame and its last outputs stand.

## 1. The constraint that sets the cut

Four facts decide where the hooks can run.

| Fact | Where |
|---|---|
| Component values borrow app data for the length of the caller's `show` call. 94 of the 268 `#[derive(Component)]` types in ui-contract, ui-widgets, ui-panels, ui-patterns, ui-shell, layers and color-ui have lifetime parameters: `IconButton<'a>` holds an `IconSpec<'a>` (`IconSource::Glyph(&'a dyn Glyph)`), `Slider<'a>` holds a `Bind<'a, f64>`, `Stack<'a>` holds its header and footer. | `ui-widgets/src/buttons/icon_button.rs:109`, `value_drag/slider.rs:64` |
| `MemoView` props borrow too: `Outbox<'a, E>`, `&mut Tracked<T>`, `Bind`, and the shell's slot view holds `&mut S` of the app. | `contract/memo_view.rs`, `ui-shell/src/shell/app_shell.rs:91` |
| ui-rt's `Element` is `'static`: the node keeps the newest element value, so layout, paint, events and access can run outside the build that created it. | `ui-rt/src/element.rs` |
| `show()` returns the component's output in the same call, with its `Response` (rect included), and a body closure is `FnMut` that captures `&mut app` and may run twice (measuring and real run). | `contract/mod.rs:783-860` |

So a component value exists only while its enclosing scope's build runs. Every hook that takes `self` (`measure`, `parts`, `sense`, `update`, `bind`, `scrub`, `activated`, `overlay`, `paint`, `paint_over`, `access`, `value_text`, `finish`) runs there. ui-rt keeps what those hooks produced, per node, and its own phases (layout, paint, events, access) work over the kept outputs:

- **Hooks run in the scope's build; outputs are retained per node.** A node whose inputs did not change does not run: it keeps its size, display lists, hit shapes, focus records, access node and memory. One that changed runs its whole pipeline, as today, inside the build.
- **A nested scope runs only inside the run of the scope around it.** Its props borrow from that scope's closure. A scope that must run therefore needs every enclosing scope up to the window root to run its closure; their other children cost a key check each.
- **Layout of contract content happens in the build,** top-down, with the room the parent gives, as today. That keeps `show()`'s output exact (its rect is this frame's) and lets the pipeline's layout code (`layout/body.rs`, `contract/body/*`) stay. ui-rt's layout phase places nodes at the offsets the build computed and moves scroll content.
- **Pointer feedback is paint-only.** A run records its display list once per pointer state its paint read (rest, hover, press), and ui-rt's paint phase picks the list by the node's `Interact` cell (section 8). Hover and press over a kept node repaint that node and run nothing.

## 2. Nodes, scopes and containers

### 2.1 Node kinds

| Node | One per | Element (`'static`) | Built by |
|---|---|---|---|
| `ComponentNode` | shown component (leaf, body or items shape) | the retained half: kept outputs plus the input queue (section 5) | the run of the component's caller |
| `ScopeNode` | window root, `MemoView`, shell slot, `Lazy`, dock panel body (a `MemoView` through `region_versions`), layer child (popup, menu, tooltip, toast, dialog, drag ghost) | kept key, place and returned value | the run of the enclosing scope, or the host for the window root |
| ui-rt fixed nodes | window root and its layers | ui-rt's own | ui-rt |

Every contract node is an **inline node**: its children are created by its owner's build, never by its own `Element::build`. A component's body children and a scope's content are reconciled into the node while the enclosing build runs (`BuildCx::inline`, section 9).

### 2.2 Marks

ui-rt has `BUILD`, `LAYOUT` and `PAINT` marks. The bridge adds `PATH`:

- `BUILD` on a node: it runs its hooks in the next build that reaches it (queued input, a changed read).
- `PATH` on a node: a node under it carries `BUILD`. A scope with `PATH` runs its closure; a component with `PATH` runs its body closure (it does anyway when its scope runs) and keeps its own outputs unless its own inputs changed.
- Marking an inline node `BUILD` marks each owner above it `PATH`, up to the window root, which is queued for the host.

### 2.3 When each scope runs its closure

| Scope | Key | Runs its closure when |
|---|---|---|
| Window root | none | `PATH` or `BUILD`: input queued under it, a read in its own code changed (viewport, scale, `ctx.input()`, env), `request_repaint`, a wake at its time |
| `MemoView` | `MemoView::key` (props, `fn`, scope key), unchanged | key moved, place moved (room, clip, layer, flow facts as `MemoView` compares them today), `PATH`, a read in its run changed |
| Shell slot (F3) | the slot's version | version moved, `PATH`, place moved |
| `Lazy` | its key plus out-of-view state | it meets the view and did not run, `PATH`, a focus request into it |
| Layer child | the layer key and placement | the owner's run shows it and it is new, `PATH`, its anchor moved (placement only: no run) |

A scope that does not run returns its kept value: `MemoView::show` returns the last run's `R`, moved with `Shift` when the node's place moved. Its node keeps its subtree; nothing is re-emitted.

### 2.4 Containers

A container's body closure runs whenever the build of its scope reaches it, because it captures `&mut app` and returns the body's value through `Inner`. The container's own hooks (its surface, header, footer, scroll parts, access node) are kept when its key, room and reads are unchanged. A measuring run of the body still happens where the layout needs it (fills and shares counted from the children, content-sized containers); a kept child answers it from its size cache.

## 3. The build

### 3.1 The `Ui` build cursor

`craft::ui::Ui<'a>` is the cursor a scope's build hands to app and component code. It holds:

- the `BuildCx` of the node being built (ui-rt);
- the layout region: max rect, cursor, clip, layer, layout direction, whether this is a measuring run;
- the `Env` scope and the id scope (the craft `Id` of the enclosing node).

Its methods are the egui `Ui` methods the contract and the component crates call today, with craft types: `ctx`, `clip_rect`, `set_clip_rect`, `layer_id`, `max_rect`, `available_rect_before_wrap`, `available_width`, `available_size_before_wrap`, `cursor`, `advance_cursor_after_rect`, `min_rect`, `set_min_size`, `set_max_width`, `expand_to_include_rect`, `expand_to_include_x`, `layout`, `with_layout`, `scope_id`, `unique_id`, `is_sizing_pass`, `opacity`, `set_opacity`, `is_enabled`, `child`, `new_child`, `scope_builder`, `spacing`, `allocate_exact_size`, plus the craft extensions (`draw`, `face`, `content_painter`, `show`, `selectable_text`, `closing`, `anchor_part`, `labelled`). ui-contract and the component crates call about 40 distinct `Ui` methods besides `ctx()` (377 calls). `ctx()` returns `craft::ui::Context`, the subset plan section 1.2 lists. `painter()` (29 calls) is replaced by `draw()`. The few egui widget and layout helpers still called (`label`, `button`, `horizontal`, `vertical_centered`, `with_style`, `set_row_height`, `end_row`: under 15 calls) become craft components.

Apps write `&mut Ui`; the lifetime is elided.

### 3.2 Reconcile by `IdSource`

| `IdSource` | ui-rt child key | craft `Id` of the node |
|---|---|---|
| `Auto` | `Keyed(type key, ordinal)`: the ordinal counts siblings of the same component type in the parent scope, as step 3 counts today | scope id joined with the type and ordinal, as today |
| `Salt(s)` | `Keyed(hash(s), 0)` | scope id joined with the salt |
| `Id(id)` | `Keyed(id, 0)` | `id` |

The type key is `typeid::of::<C>()` (lifetimes erased). The node keeps its craft `Id`, so services keyed by `Id` (memory, focus requests, AccessKit ids, tests' queries) keep their keys. The bridge keeps a per-window map from `Id` to `NodeId` for requests that name an id. A child that the build does not reach again is removed at the end of its owner's build, with what it put on layers (ui-rt `gone`).

### 3.3 The keep decision

For each `show` the bridge decides before running any hook. The node is kept when all of these hold:

1. the node exists with the same type key;
2. the component key is unchanged (section 6);
3. the place is unchanged: room, clip, layer, scale, the scope's style inputs and the body flow facts `MemoView` compares today;
4. the node carries no `BUILD` mark (no queued input, no changed read);
5. this is not the first real run after a measuring run that ran it with other facts.

A kept node runs no hook. Its output comes from `finish(self, cx)` with a *settled* `Cx`: the kept facts of its last run (rect, flow, state, flags, command, response) and no input. A kept node whose position moved within an unchanged size takes a new offset (no repaint; renderers translate). Anything else runs the full pipeline in the build, as today.

### 3.4 The window root and the host

The app's `ui` closure is the window root's content. ui-rt cannot store it (it borrows the app), so the host passes it per frame: `Runtime::frame_with(text, roots)` calls `roots(window, cx)` for each window whose root is queued, inside ui-rt's build pass, so later build rounds of the same frame (a layout that moved what a build reads, a `Lazy` that came into view) run it again in the same frame. `Driver::step_with` passes the closure through. `App::logic` runs before the build on every frame the driver runs.

## 4. Input and outputs in the same frame

1. **Route.** The platform event reaches ui-rt, which hit-tests the kept shapes of the last paint (top layer first), keeps a press with the node that took its `Down`, and routes keys to the focused node. This is egui's order: last frame's shapes, this frame's answer.
2. **Queue.** The `ComponentNode`'s `event` hook appends a `NodeInput` (gesture and part, position, modifiers, key, text, IME, paste, AccessKit action, focus change, dismissal) to its queue and marks itself `BUILD`; its owners take `PATH` up to the root. Hover and press feedback need no entry: ui-rt writes the node's `Interact` cell, which only paint reads.
3. **Rebuild.** The frame's build pass runs the root closure, which reaches the node through the `PATH` scopes. The node runs: step 8 senses from the queue instead of egui's input, `update`, `bind`, `scrub`, `activated` and `overlay` see it, and `finish` returns the output. `show()` returns `clicked()`, `Commit` and the rest in the frame the input arrived.
4. **Phases.** Layout, paint and access run over the nodes that changed.

Events that leave a `MemoView` through an `Outbox` in its props work unchanged: the view runs inside its parent's run, which holds the outbox's `Vec`.

A build that reads interaction state from an output (`Response::hovered()`, `contains_pointer()`, `is_pointer_button_down_on()`) records a build read of that one fact of the node's interaction state (`InteractFact`) for the reading scope, so the scope runs again when the fact flips and not when another fact (a press under a hover read) changes. `Response` logs the read into the running build (a per-thread log the bridge drains at the end of each scope's build).

Inputs a node's retained half resolves alone stay there: a wheel on a scroll body moves the scroll offset cell (layout moves the content; no run), and a wheel that reveals rows outside the built band marks the body `BUILD`.

## 5. `ComponentNode`: the retained half

`ComponentNode` is one non-generic `Element`. Its `Local` holds what the last run produced:

| Field | Filled by | Read by |
|---|---|---|
| type key, component key, place | the keep decision | the keep decision |
| size and child offsets | layout in the run | `Element::layout` |
| style (`Arc` of `C::Style` behind `dyn StyleView`) and the kept `CxCore` facts | steps 2, 6, 12 | the settled `Cx` |
| parts, hit shapes (local coordinates), roving groups, focus stops and scopes | steps 7, 8 | `Element::paint` (re-records the hits), the focus service |
| paint variants: content and overlay lists per pointer state, with the state bits the run read | steps 12, 13 and faces (section 8) | `Element::paint` |
| focus ring shape and look | step 13 | `Element::paint` |
| AccessKit node and sub-nodes | step 16 | `Element::access` |
| input queue | `Element::event` | step 8 of the next run |
| memory slots (`Remember`) | the memory stage | the next run |
| cursor request | step 8 | ui-rt hover |

Its `Element` hooks:

- `props_key`: the component key the bridge stored; `build` never runs on its own.
- `layout`: returns the kept size and places the children at the kept offsets. It is a relayout boundary: its size is decided in the build.
- `paint`: picks the variant for the current `Interact` (it reads the cell only when its variants differ), blends variants during a state fade (`time_until` until the fade ends), paints the focus ring when `focus_visible`, and re-records the kept hit shapes.
- `event`: queues and marks (section 4). A layer child's `Dismiss` is queued the same way and the owner's run closes it.
- `access`: the kept node.
- `focusable`: false. The craft focus service owns Tab order, Tab layers, roving and F6 (section 10) and moves ui-rt's key focus.
- `text_input`, `ime_cursor`: from the kept editor part.

## 6. The component key

`#[derive(Component)]` generates `fn key(&self, key: &mut MemoKey)` over the component's fields with the `MemoProps` field walker (`contract/memo.rs`), so the key covers everything the hooks read from `self`:

- `Common` and plain props by value; `&str` and `Label` by content; `IconSpec` by source and ink.
- a binding (`Bind`, `BindMut`) by the bound value it shows and the bound field's address. State that moves (an app value moved between frames, a `Vec` that reallocates) reruns its bound components once; bound app state should stay put, boxed or in stable storage;
- a reference to shared data (`&dyn Glyph`) by address, `Tracked<T>` by revision;
- a closure or other field the walker cannot read: `#[key(skip)]` when the hooks do not read it, `#[key(by = expr)]` otherwise.

A type whose key cannot be derived is marked `#[component(unkeyed)]` and runs at every build of its scope. Framework inputs a run reads (env values, tokens, commands, focus facts, overlay state) are tracked reads, not key fields.

## 7. Hooks and phases

| Hook today | Runs in | Kept output | The node runs again when |
|---|---|---|---|
| `show()` props, step 1 `Common`, `style`, `default_variant`, `content_env` | the run, in the scope's build | component key, style | key moved; a tracked env read moved (theme, density, scale, language, scope values) |
| `measure`, `padding`, `fills`, `cut_first`, `BodyLayout`, the measuring run | the run, top-down in the build | (room to size) cache, child offsets | key, room, text and token reads; a child's size changed |
| `parts(rect)`, `sense()`, `wheel_notch`, roving groups, focus stops | the run, after the rect is known | parts, hit shapes, stops | key, size |
| step 8 sensing (press machine, clicks, drags, long press) | ui-rt event routing (`Gesture` from its `PressMachine`) into the queue; consumed by step 8 of the next run | input queue | input |
| `update`, `bind`, `scrub`, `activated`, Escape, `overlay` | the run | bound-value writes, layer children, output fields | queued input, key |
| surface, state fill, faces, `paint`, `paint_over`, disabled veil | the run, once per pointer state read (section 8) | content and overlay lists per state | key, size, paint reads other than pointer state (focus, selection, flags, tokens, frame time) |
| pointer look (hover, press, their fades) | ui-rt paint phase | | `Interact` hover and press; the frame time during a fade |
| focus ring | ui-rt paint phase | | `Interact::focus_visible` |
| `access`, `value_text`, `Access` sub-nodes, `announce` | the run | AccessKit node and sub-nodes | the node ran; ui-rt sends bounds for moves |
| `finish(self, cx)` | every `show`: after the run, or on the settled `Cx` of a kept node | | |
| `Inner::settle` | the end of the container's body in the build | | |

## 8. Paint variants

The paint steps (surface, state fill, faces, `paint`, `paint_over`, veil) resolve looks against the `StateSet`. The bridge tracks what a run's paint read from it: `StateSet::resolve` on a `Stateful` look with a hover or press entry reads the pointer state; `Styled::state()` reads all of it.

- When paint read no pointer state, the run records one variant.
- When it read hover or press, the run records `Rest`, `Hover` and `Press` (only the ones reachable: `Press` when the component senses presses). Each is a full paint with that state, so what each variant shows is exactly what a run in that state paints.
- A fade between states (`StateMix`) blends the two lists item by item when their structure matches (same items, different colours), which is what `Mix` does for colours; otherwise it switches at the midpoint.
- Other state bits (focus, selection, open, checked, disabled) are build reads: their change runs the node. The focus ring is the exception: the node paints it in the paint phase from its kept shape.

Cost: a node that reads pointer state paints up to three times per run. A hover or press over a kept node costs one paint of that node (a variant swap plus its hits) and no run.

## 9. What ui-rt gains

No frozen trait changes. Additions:

| Addition | File | Lines |
|---|---|---|
| `Node::inline`; `Dirty::PATH`; `mark_build` on an inline node marks owners `PATH` up to the root and queues the root | `tree.rs` | ~40 |
| `BuildCx::inline(el, f)`: reconciles `el`, clears its marks and calls `f` with the node's `Local`, an `InlineState` (new, key moved, `BUILD`, `PATH`) and a nested `BuildCx`; `f` decides to run or keep, and `BuildCx::keep_children` leaves the subtree as it was; otherwise children `f` did not reach go. `mark_changed` and `request_layout` queue only what changed. `inline_layer(layer, key, spec, el, f)` does the same for a layer child the building node owns, and `peek_layer` reads one's `Local` | `build.rs` | ~150 |
| `Runtime::set_inline_root`, `Runtime::frame_with(text, roots)`; the build pass calls `roots` for queued inline roots and skips other inline nodes | `runtime.rs` | ~70 |
| `Driver::step_with(time, roots)`, `Driver::open_inline_window` | `driver.rs` | ~25 |
| `PaintCx::extend(&DisplayList)` for variants; later an `Arc` swap that skips the copy | `paint.rs` | ~10 |
| counters `runs` and `scopes` beside `built` (`BuildCx::count_run`, `count_scope`) | `runtime.rs`, `build.rs` | ~20 |
| `InteractFact` and per-fact interaction cells; `BuildCx::interaction` (no read), `interaction_cell`, `read_interaction(node, fact)` (a build read of one fact, so a scope reading `hovered()` does not rerun on a press), `BuildCx::peek_state` (a cell's value without a read) | `input.rs`, `build.rs`, `runtime.rs` | ~75 |
| `EventCx::hit(pos, accept)`, `tag_of(node)`, `request_build_of(node)`: a press owner hit-tests along its drag and marks what it crosses (toggle sweeps) | `event.rs` | ~20 |
| build wakes: `BuildCx::request_build_at`, `EventCx::request_build_at` (a tooltip's delay, a toast's timeout) and an untracked `BuildCx::now` | `build.rs`, `event.rs`, `runtime.rs` | ~40 |
| `Event::Hover(bool)` for nodes that ask (`BuildCx::hover_events`); `EventCx::request_owner_build` (a layer child tells its owner it was dismissed) | `event.rs`, `input.rs` | ~30 |
| `Item::Text.angle` (turned text), `ImageOptions.wrap` (`Wrap::{Clamp, Repeat}`: repeating images), `Mesh` builders, `TextEngine::{install_face, has_face}` with `FaceInstall` (document faces installed while the app runs); each renderer supports them | `display.rs`, `text.rs` | ~95 |
| B10 windows: `CloseReason`, `Event::CloseRequested`, `PaintCx::close_guard` (a node answers its window's close requests), `Runtime::{request_close, quit}` (the one close path: an app-modal block refuses and focuses the dialog's window, a guarded window asks its topmost guard, any other closes; replaces `take_close_requests`), `PaintCx::window_zone` and `Runtime::window_zones` (places the OS acts on, with the input shapes above them), `Runtime::{window_geometry, window_key}`, `WindowSpec::key`, `EventCx/BuildCx::open_inline_window`, `BuildCx::window_command`, `Driver::{set_wake_handler, quit}` (the host's job board delivers on `PlatformEvent::Wake`); a blocked window shows the arrow | `event.rs`, `paint.rs`, `hit.rs`, `runtime.rs`, `input.rs`, `driver.rs`, `build.rs`, `platform.rs`, `window.rs` | ~230 |
| `LayerSpec::{escape, focus_left, app_modal}`, `DismissReason::{Escape, FocusLeft}`: an unhandled Escape dismisses the topmost layer child that asks, focus leaving one dismisses it, an app-modal child blocks the other windows' input (`Runtime::blocked_by`); `LayerSpec::place` and `constraints` public so a build places its layer child as the runtime will | `layer.rs`, `input.rs` | ~110 |
| `BuildCx::{provide, provided}`, `PaintCx::provided` (values a node gives itself and what it owns: env cells, the job board), `BuildCx::slot` (a cell created by key, owned by the node), `BuildCx::derived`, `Derived::state`, `Runtime::store_and_images_mut` (a host delivers job results into cells and images together) | `build.rs`, `paint.rs`, `tree.rs`, `state.rs`, `runtime.rs` | ~90 |
| focus and keys for the craft focus service: `Runtime::{window_node, children, parent, local}` and `EventCx::runtime`/`BuildCx::runtime` (read-only walks over the nodes' `Local`s); `PaintCx::interaction_fact` (a paint read of one fact: the focus ring reads `FocusVisible` alone); a key nothing on a layer child's path took goes to the app root and its content's top nodes last; `BuildCx::focus_into` (focus enters a node's content after the build pass), `BuildCx::set_focus_return` (where focus goes when the node goes with focus inside), `BuildCx::focus_node` | `runtime.rs`, `event.rs`, `build.rs`, `input.rs`, `window.rs` | ~110 |
| `EventCx::forward_to(node)` and `forwarded_from()` (a handler hands the event on to another node after it returns, as to that node's target: a text field's list keys to the collection it drives) | `event.rs`, `input.rs` | ~25 |
| `EventCx::provided` (a key stage finds what its window root provides: the command board); with nothing focused, a key goes to an inline root's content (the window root scope: its Tab, F6 and commands) instead of the inline root | `event.rs`, `input.rs` | ~10 |

## 10. Services

| Service | Today | On ui-rt |
|---|---|---|
| Sensing (`primitives/sensing.rs`, `runtime/pointer.rs`, `wheel.rs`) | own hit test over shapes registered per pass, `PressMachine` per root and arena, pointer snapshot | ui-rt hit test on kept shapes and its `PressMachine`; the node queues gestures; arena members are parts of their root's node with sub-hits |
| Focus (`runtime/focus*`) | stops, scopes and roving groups registered per pass; transcripts for out-of-view content | `rt/focus.rs`: stops (`Component::keys`), scopes and roving groups declared by a node's run and kept with the node (no transcript); Tab, F6, roving moves and requests walk the kept nodes; a trapping scope wraps Tab, the window root moves along the window's order and enters the topmost trapping layer child; the service moves ui-rt's key focus, and the ring is painted from `FocusVisible`, so a move runs nothing |
| Keys (`runtime/keys*`) | input hook on egui raw input, `KeyUse`, key stage | `rt/keys.rs`: ui-rt bubbles a key from the focused node; each node claims from what its run kept. The focused node takes what its `KeyUse` takes as intents of the key table read in its role (activation, kept moves, deletes, Submit when it submits, Tab when it keeps Tab, its edit chords as `Intent::Edit`, a held Escape as Cancel, a recorder's presses and claimed chords as chords) and queues them (`NodeInput::Intent`, which marks its path); a typing node holds the plain keys whose text follows. Then a roving owner's moves, a trapping scope's Tab, a layer child's scope rules (`rt/intents.rs`: Enter is Submit under `EnterRule::Submit`, Escape clears then closes under `ClearThenClose`, both queued for the layer body's run: `Ui::take_submit`, `take_clear`, `clearable`), the window root's Tab and F6, then its command stage; an Escape nothing took dismisses the topmost layer child that asks (ui-rt). The run reads the queue through `Cx::intents` (the contract's `Intents`); `Activate` sets `clicked` |
| Commands (`runtime/commands`) | board, per-frame state, queue, gate | `rt/commands.rs`: one `CommandBoard` per runtime from the host's keymap, given through `WindowEnv::commands`. The window root looks up a chord no node took (typing when the focused stop types, so the keymap's typing rule holds back typing chords; `KeyUse::command_keys` lets command and function keys through) and runs the bound command's handler at event time with the store: a command changes app state by writing cells, so only their readers run. Each command's `CommandState` is a cell: a disabled command's bindings are passed over, read without recording; `Ui::command_state` reads it, so a state change runs exactly its readers |
| IME engine (`runtime/ime.rs`, `primitives/text_input.rs`) | egui IME events plus fork patches | `rt/ime.rs`: ui-rt's platform IME session per window follows the focused node's `Component::text_input` and the caret rect its run reports (`Cx::ime_cursor`); preedits and commits are queued on the node (`EditorInput`), a commit's text follows as `Intent::Text`; the node holds every key while composing and commits its composition itself when it loses focus (a click elsewhere), before ui-rt stops the session. Typed and pasted text reach a typing or type-ahead node as `Intent::Text`. `Cx::editor_input` gives an editor its IME input and intents in arrival order; `rt/text.rs` has the editor state and the stand-in `TextField`. The type tool uses the same engine (owner rule) |
| Drag (`runtime/drag.rs`) | one session per context with payload and preview | payload sessions on ui-rt input (A2); the ghost on the drag layer |
| Motion (`runtime/motion.rs`) | tweens over one clock, one repaint per viewport | a run names targets (`Cx::fade_to`, the pointer variants); the node's paint evaluates the tween at the frame time (`rt/motion.rs`): hover fades blend the kept variants, an opacity tween wraps the node. A running animation reads the frame time in paint, so ui-rt repaints that node per frame and asks for frames; the paint that finds it finished reads nothing and nothing is asked for. The mode and durations are env cells read in paint |
| Overlay (`runtime/overlay.rs`, `contract/overlay*`) | egui layers, order, open modal | ui-rt layers per window; layer children are inline nodes of the component that shows them; outside-press dismissal from ui-rt |
| Cursor (`runtime/cursor.rs`) | requests collected per pass | the node's kept cursor (`Component::cursor`, `PartBuilder::cursor`) recorded with its hit shapes; ui-rt picks it on hover and sets it through `Platform::set_cursor` only when it differs (`rt/cursor.rs`) |
| Memory (`runtime/memory.rs`) | slots in egui's `IdTypeMap`, eviction by seen marks | slots in the node's `Local` (`rt/memory.rs`), freed with the node; cells a scope or job needs are `BuildCx::slot` cells owned by the node; the saved tier stays in the host store keyed by `Id` |
| Env (`runtime/env*`) | `Env` walked per run, scope values | env values as cells (`rt/env.rs`: tokens, theme base, direction, density, motion mode, layer frame) provided by the window root (`BuildCx::provide`) and found by the nearest owner; a hook asks through its context (`Cx::direction`, `Paint::frame`), the run registers a build read of what was asked after the hooks, so a write marks only those nodes. `EnvScope`/`ThemeScope` give a subtree cells of its own: a theme scope's are derived cells, so a base write marks its readers at the start of the pass |
| Access (`runtime/access*`, `runtime/egui/access*`) | full tree every pass, cloned at settle | node-owned AccessKit nodes in ui-rt's incremental update, off until AT activation |
| Pending and wakes (`runtime/pending.rs`) | results polled per pass | `JobBoard` runs jobs on `host::Jobs`, wakes the driver through its `Waker`, and `deliver` (on `PlatformEvent::Wake`) writes each result into the cell the waiting node owns and reads (`Ui::job`) or into the `ImageStore` (`TextureCache`) |
| Windows (`runtime/windows*`) | egui viewports | ui-rt windows and the platform's multi-window (B10): `contract::rt::windows` has `Ui::{open_window, close_window}`, `WindowRegistry` (saved placement through `craft_window::restore::Store`, placement recording from `Runtime::window_geometry`, hit sinks and caption replay for the Windows custom-chrome hit test from `PartBuilder::exclusive` zones) and `Component::close_guard` / `Cx::close_request`; the egui viewport code stays until the integration task deletes `runtime/egui` |

## 11. What is deleted

| Module | Lines (non-test) | Replaced by |
|---|---|---|
| `runtime/egui/*` (36 files) | 8461 | the bridge, the platform and renderer crates, the craft test kit |
| `runtime/replay.rs` | 632 | kept nodes |
| `runtime/pass_state.rs` | 456 | node `Local`s and the bridge's per-build state |
| `runtime/audit.rs` | 103 | nothing: no replay to audit |
| `runtime/reads.rs` | 203 | ui-rt read tracking |
| `runtime/layout_memory.rs` | 1413 | the node size cache |
| `runtime/measure.rs` (egui sizing passes) | 664 | the cursor's measuring flag |
| focus transcripts in `runtime/lazy.rs` and `runtime/focus*` | about 900 | stops kept with nodes |

`runtime/shift.rs` (208) stays: a kept scope at a new place returns its value moved.

## 12. Sizing

Line counts are non-test code (`_tests.rs`, `_trials.rs`, `tests/` and `testing` excluded). Estimates are lines written or changed.

| Module | Lines | Change | Estimate | Task |
|---|---|---|---|---|
| `contract/rt/*` (new) | 0 | `Ui` cursor 700, reconcile and id map 300, scopes 500, `ComponentNode` 700, queue 200, variant recording 300, host 300, `Response` and read log 250 | +3.3k | B1 skeleton (about 1.5k), rest B3, B4, B6 |
| ui-rt additions | 0 | section 9 | +0.35k | B1 |
| `runtime/egui/*` | 8461 | deleted; textures go to B2, input, clipboard, files, IME to B6 and B10, access to B8, fixture and injection to B11, viewports to B10 | -8.5k | B2, B6, B8, B10, B11 |
| `contract/run.rs`, `run/typed.rs` | 3041 | keep decision first; hosts become nodes and layer children; the measuring flow stays | 1.2k | B1 (leaf path), B3, B4 |
| `contract/steps/*` | 2374 | sense from the queue (B3); paint into variants (B2); report into the node (B8); place and id on the cursor and reconcile (B1, B4) | 1.2k | B2, B3, B8 |
| `contract/cx/*`, `cx.rs` | 2932 | `CxCore` response, ctx and memory on the node; settled `Cx` | 0.6k | B3 |
| `contract/body/*`, `body.rs`, `container.rs` | 6785 | cursor; layout memory to the size cache; measuring run served by the cache | 1.8k | B4 |
| `layout/*` | 2786 | cursor; size cache | 0.6k | B4 |
| `contract/items/*`, `items.rs`, `collection/*` | 8104 | rows as keyed nodes in the band; `MEMBERS` rows as parts and variants of one node; reveal by kept extents | 2.5k | B5 |
| `contract/label/*` | 1715 | text through `TextEngine` | 0.4k | B2 |
| `contract/overlay/*`, `runtime/overlay.rs`, `tooltip.rs`, `contract/toast.rs`, `runtime/toast.rs` | 5158 | layer children on ui-rt layers, dismissal | 2.0k | B7 |
| `contract/memo_view.rs`, `contract/lazy.rs` | 649 | scope nodes; replayed becomes kept | 0.4k | B1 |
| `runtime/lazy.rs` | 939 | kept extents stay; transcripts go | 0.2k, -0.4k | B4 |
| `contract/memo.rs`, `ui-macros` component derive | 1128 + 3215 | component key walker and derive attributes | +0.5k | B1 (walker), B3 (derive over all types) |
| `runtime/{replay,pass_state,audit,reads,layout_memory,measure}.rs` | 3471 | deleted | -3.5k | B1 (replay, audit), B4 (layout memory, measure), B9 (pass state users) |
| `runtime/frame.rs` | 902 | frame plugin to driver hooks (input, logic, `frame_with`, end) | 0.5k | B1 (host), C2 |
| `runtime/windows*` | 2681 | platform multi-window | 1.5k | B10 |
| `runtime/focus*` | 3410 | node-kept stops; drives ui-rt focus | 1.2k, -0.5k | B6 |
| `runtime/keys*` | 2968 | key stage on platform events; intents into queues | 0.8k | B6 |
| `runtime/commands` | 3979 | tracked state | 0.3k | B6 |
| `runtime/ime.rs`, `primitives/{ime_surface,text_input}.rs` | 1027 | platform IME per window | 0.6k | B6 |
| `runtime/drag.rs` | 994 | ui-rt payload sessions | 0.4k | B7 (with A2) |
| `runtime/pointer.rs`, `runtime/wheel.rs` | 1847 | ui-rt routing | 0.4k, -0.8k | B3 |
| `primitives/sensing.rs` | 1307 | press machine in ui-rt; arena hits on kept shapes | 0.8k, -0.5k | B3 |
| `runtime/motion.rs`, `cursor.rs`, `memory.rs`, `pending.rs`, `inputs.rs` | 2449 | section 10 | 1.0k | B9 |
| `runtime/env*`, `style/*` | 6000 | env cells with read tracking | 0.9k | B9 |
| `runtime/access*` | 2280 | node-owned, incremental | 1.0k | B8 |
| other runtime (`inspect`, `sampler`, `toggle_sweep`, `pen`, `edit`, `requests`, `derived`, `install`) | 1940 | small re-plumbing | 0.4k | B3, B9 |
| `primitives/{draw,kit,surface,face,faces,state_layer,focus_ring,backdrop,record,paint_slot,content}.rs`, `contract/paint_ui.rs` | 3416 | `Draw` writes display items in local coordinates | 2.5k | B2 |
| `primitives/{text,text_adapter}.rs`, `fonts/*` | 6445 | `TextEngine` binding | 1.5k | B2 |
| `testing/*` | 11335 | Harness on headless | 3.0k | B11 |

Totals: about 3.9k new, 25k changed, 14k deleted in ui-contract and ui-rt. The component crates change through the codemod (types) and the derive attributes of section 6.

## 13. Trials

Count trials, relations only:

- Hover over a kept `IconButton`: one node painted, no run, no scope run.
- A click on an `IconButton`: `show()` returns `clicked()` in the frame of the release.
- A settled `Stack` inside a `MemoView` whose parent runs: the view's closure does not run, no node runs, no display list version moves, no AccessKit node is sent.
- A changed prop on one child of a `Stack`: that child runs, its siblings do not.
- R1 (A7): a synthetic PhotoCraft-shaped root on headless: hover runs nothing outside the hovered panel's paint; a click runs the root closure, the scopes on the path and the clicked node.

## 14. Alternatives

- **An owned retained half per component.** Each component would produce a `'static` record in its run, and `measure`, `parts`, `paint` and `access` would take that record in ui-rt's own phases. This changes the `Component` trait for every type (the 94 with lifetimes by hand), turns bindings into snapshots with queued writes, and makes `show()` return last frame's rect. Not taken: the trait is kept.
- **Deferred colours.** `cx.resolve` would return a colour that the paint phase resolves against `Interact`. Component code that computes with a resolved colour would break, and non-colour looks (widths) would still need runs. Variants (section 8) cover the same cases without touching component code.
- **Scopes that run on their own.** A `MemoView` whose props own their data and that returns `()` could run without its parents (`MemoView::owned`). Left for later: the borrowed form is the common one, and the path to the root costs a closure run per scope plus key checks.

## 15. Frozen interface for the integration

This section fixes the public interface the parallel integration tasks (INT-1a, 1b, 2, 3, 4, 5b, 6a, 7 and C1) write against. Each task codes to these signatures without seeing the others' work. Where a signature here differs from the code on spike-v4, this section wins. A task that finds a signature here it cannot implement stops and reports it; it does not change the signature on its own.

Sources: `contract/mod.rs` (the real traits), `contract/cx*`, `contract/paint_ui.rs`, `contract/rt/*` (the rt halves), the plan (briefs/spike-int-plan.md, sections 1-3 and the lead decisions on Q1-Q12) and greps of the component crates on 27 Nov (non-test `src`).

### 15.0 Conventions

- **Paths.** Every public name lives at its real path: `craft_ui_contract::contract::*`, `runtime::*`, `primitives::*`, `style::*`, `layout::*`. Nothing is public under `contract::rt::`; the bridge files stay there as private modules (Q11). Types that the rt halves made public under `contract::rt` and that survive are re-exported at the paths in 15.10.
- **The `rt` feature is gone.** `craft-ui-platform-headless` is a dependency under `testing` only.
- **Geometry.** `Pos2`, `Rect`, `Vec2`, `Align`, `pos2`, `vec2` are `craft_ui_rt`'s (its `emath` re-export). They are the same types as egui's, so code that used `egui::Rect` keeps its values.
- **`Ui` lifetimes.** `contract::Ui` is `Ui<'u, 'c>` (`'u` the borrow of the `BuildCx`, `'c` the `BuildCx`'s own). `&mut Ui` and `&Ui` elide in fn signatures and fn-pointer types. A struct field that holds a `Ui` names both lifetimes (`&'a mut Ui<'u, 'c>`).
- **One-lifetime contexts.** `PaintUi<'a>`, `OverlayUi<'a>`, `BodyCx<'a>`, `MeasureCx<'a, C>`, `PartsCx<'a, C>`, `Cx<'a, C>`, `CxCore<'a>` keep their single lifetime, so the 300 `PaintUi<'_>`, 37 `OverlayUi<'_>`, 364 `BodyCx<'_>` and 239 `MeasureCx<'_, Self>` sites in the crates compile unchanged. `PaintUi` and `OverlayUi` reach the build through an object-safe, crate-private trait implemented by `Ui<'u, 'c>`:

  ```rust
  pub(crate) trait UiHost {
      fn with_ui(&mut self, f: &mut dyn FnMut(&mut Ui<'_, '_>));
  }
  // PaintUi<'a> and OverlayUi<'a> hold `host: &'a mut (dyn UiHost + 'a)`;
  // `&'a mut Ui<'u, 'c>` coerces to it because `Ui<'u, 'c>: 'a`.
  ```

  Generic methods (`child<C: Show>`, `show<R>`) call `with_ui` with a closure that runs the generic code on the rebuilt `&mut Ui`; results leave the closure through a local `Option<R>`.
- **Read registration.** "Tracked" below means the read registers with ui-rt (a cell read, or `read_interaction(node, fact)` for one interaction fact), so a change reruns exactly the readers. "Untracked" means a peek.

### 15.1 Traits and contexts

#### `Meta`, `Show`, `ShowContainer`, `ShowCollection`

Unchanged from `contract/mod.rs`, with `Ui` the craft `Ui`. `Show::measure_in(&mut self, ui: &mut Ui, proposal: Proposal) -> Vec2` and `ShowContainer::measure_in(&mut self, ui: &mut Ui, proposal)` stay (a measuring run needs `&mut BuildCx`); they run a measuring run through the node's size cache (B4). `show_at_taking_room` stays.

Backed by: `Meta` is a plain trait (no rt state). The `Show*` entry points call the rt driver (`contract/rt/run.rs`: keep decision, `Kept`, variants, size cache) with the real steps (INT-1b, INT-2, INT-4).

#### `Component`

The real trait, unchanged, plus seven hooks from the rt cut. Frozen text (doc comments omitted):

```rust
pub trait Component: Meta + Sized {
    const ROLE: Role;
    const ALT_ROLES: &'static [Role] = &[];
    const INPUT: Input = Input::Pointer;
    const ESCAPE: EscapeEdit = EscapeEdit::Cancel;
    const OVERLAYS: bool = true;
    const PARTS: bool = true;
    const STOPS: bool = true;
    const FOCUS_SCOPE: crate::runtime::focus::FocusPolicy = crate::runtime::focus::FocusPolicy::NONE;
    const KEYS: crate::runtime::focus::KeyUse = crate::runtime::focus::KeyUse::of(Self::INPUT);
    const SHOWS_LABEL: bool = true;
    const LABELLED_AFTER: bool = false;
    const SINGLE_VALUE: bool = false;
    const BINDS_VIEW: bool = false;
    const DISABLED: DisabledLook = DisabledLook::States;
    const RULE: bool = false;
    type Style: ComponentStyle;
    type Output: Outcome;

    fn focus_scope(&self) -> crate::runtime::focus::FocusPolicy { Self::FOCUS_SCOPE }
    fn keys(&self) -> crate::runtime::focus::KeyUse { Self::KEYS }
    fn shows_label(&self) -> bool { Self::SHOWS_LABEL }
    fn label_detail(&self) -> Option<crate::runtime::env::Label> { None }
    fn paint_outset(_tokens: &crate::style::UiTokens) -> f32 { 0.0 }
    fn default_variant(_env: &crate::runtime::env::Env) -> Option<craft_theme::Variant> { None }
    fn style(cx: &StyleCx<'_>, key: Self::StyleKey) -> Self::Style;
    fn padding(&self, _rect: Rect, style: &Self::Style) -> craft_behavior::Insets { style.metrics().padding }
    fn measure(&mut self, _cx: &MeasureCx<'_, Self>) -> Vec2 { Vec2::ZERO }
    fn fills(&self) -> Option<craft_behavior::Axis> { /* as today */ }
    fn cut_first(&self) -> bool { false }
    fn sense(&self) -> Sense { /* as today */ }
    fn wheel_notch(&self) -> craft_behavior::WheelNotch { craft_behavior::WheelNotch::DEFAULT }
    fn parts(&mut self, _rect: Rect, _cx: &mut PartsCx<'_, Self>) {}
    fn bind(&mut self) -> Option<BindMut<'_, ValueOf<Self>>> { None }
    fn same_value(&self, proposed: &ValueOf<Self>, host: &ValueOf<Self>) -> bool { proposed == host }
    fn scrub(&self) -> Option<Scrub<ValueOf<Self>>> { None }
    fn scroll(&mut self) -> Option<BindMut<'_, ScrollPos>> { None }
    fn content_env(&self, _cx: &Cx<'_, Self>, _values: &mut crate::runtime::env::AppValues) {}
    fn update(&mut self, _cx: &mut Cx<'_, Self>) {}
    fn overlay(&mut self, _ui: &mut OverlayUi<'_>, _cx: &mut Cx<'_, Self>) {}
    fn flags(&self) -> Flags { Flags::empty() }
    fn activated(&self, cx: &Cx<'_, Self>) -> bool { cx.activation().is_some() }
    fn paint(&mut self, _ui: &mut PaintUi<'_>, _cx: &Cx<'_, Self>) {}
    fn paint_over(&mut self, _ui: &mut PaintUi<'_>, _cx: &Cx<'_, Self>) {}
    fn access(&self, _access: &mut Access, _cx: &Cx<'_, Self>) {}
    fn value_text(&self, _cx: &Cx<'_, Self>) -> Option<String> { None }
    fn word_value(&self, _value: &ValueOf<Self>, _cx: &Cx<'_, Self>) -> Option<String> { None }

    // Added from the rt cut. Each is kept with the node, so ui-rt reads it without a run.
    fn cursor(&self) -> Option<craft_behavior::Cursor> { None }
    fn drag_source(&self) -> Option<DragSource> { None }
    fn drop_target(&self) -> Option<DropRule> { None }
    fn close_guard(&self) -> bool { false }
    fn sweep_value(&self) -> Option<bool> { None }
    fn roving(&self) -> Option<crate::runtime::focus::Roving> { None }
    fn text_input(&self) -> Option<ImePurpose> { None }

    fn finish(self, cx: Cx<'_, Self>) -> Self::Output;
}
```

| Item | Backed by on rt |
|---|---|
| id source | not a hook: the driver takes it from `Common` as step 3 does (`steps/id.rs`); rt `id_source()` is deleted |
| tooltip | not a hook: `Common::tooltip`, shown by the rt tip (`rt/tip.rs`, `TipKept` on `Kept`); rt `tooltip()` is deleted |
| `STOPS`, `keys()` | `Kept.focus.stop = C::STOPS.then(\|\| self.keys())` (B6a decision 2); rt `keys() -> Option<KeyUse>` is deleted |
| `focus_scope`, `roving` | `Kept.focus` (`FocusKept`) |
| `cursor` | `Kept.cursor`, converted to `craft_ui_rt::Cursor` when recorded with the hit shapes (`rt/cursor.rs`) |
| `drag_source`, `drop_target` | `Kept.drag`, `Kept.drop` (`rt/drag.rs`) |
| `close_guard` | `Kept.close_guard`, `PaintCx::close_guard` (B10) |
| `sweep_value` | `Kept.toggle`, `rt/sweep.rs` |
| `text_input` | `Kept.ime` (`ImeKept`, `rt/ime.rs`) |
| `paint`, `paint_over` | variant recording (section 8, 15.6) into `Kept.variants` |
| `access`, `value_text` | built once per run, never per variant, into `Kept.access`, `Kept.access_subs` |
| `finish` | every `show`; for a kept node `finish` gets an owned settled `Cx` built from the kept facts |

`DragSource`, `DropRule` and `ImePurpose` are re-exported from `contract` (15.10).

#### `CxCore<'a>` and `Cx<'a, C>`

`Cx<'a, C>` keeps its fields (`core: &'a mut CxCore<'a>` hidden, `style: &'a C::Style`, `mixed` hidden) and `Deref<Target = CxCore<'a>>`. A kept node's `finish` gets a `Cx` that borrows a settled `CxCore` the driver builds from `Kept`.

`CxCore<'a>` public fields after the switch:

| Field | Type | Backed by on rt |
|---|---|---|
| `env` | `EnvView<'a>` | `EnvSnapshot` of the node; every `EnvView` accessor is a tracked read of the env cell it answers from |
| `resolved` | `&'a Resolved<'a>` | step 1, unchanged |
| `id` | `Id` | `Kept.id` |
| `rect`, `content_rect`, `visible` | `Rect` | the cursor's placement in the build (B4) |
| `flow` | `Flow` | as today |
| `response` | `Response` (craft, 15.3) | the node's `Response`: input facts of the run plus the interaction read log |
| `command` | `Option<Arc<ResolvedCommand>>` | `CommandBoard` resolve (15.8) |
| `intents` (hidden) | `Intents` | B6b's queue: the `NodeInput::Intent`, `Chord` and `Nudge` entries of the run |
| `memory` (hidden) | `&'a mut MemoryStage` | `Kept.memory` (`rt/memory.rs`) behind the `MemoryStage` API; the saved tier stays in the host store keyed by `Id` |
| `focus` (hidden) | `crate::runtime::focus::Facts` | `FocusKept` and the node's `Focused`/`FocusVisible`/`FocusWithin` facts (tracked) |
| `toggles` (hidden) | `Vec<Toggle>` | as today |

Deleted: `ctx: egui::Context` (41 `cx.ctx` sites in the crates move to the methods below or to `Ui`/`Context`, 15.9), `surface_slot: Option<egui::layers::ShapeIdx>` (becomes a `Canvas` mark, item 10), `member_of`, `pointer_group`, `root_sense` (no egui widget, no arena).

Methods moved from the rt `Cx` onto `CxCore` (they need no `C`):

```rust
impl CxCore<'_> {
    pub fn input(&self) -> &[NodeInput];                        // the run's queue, oldest first
    pub fn hold_escape(&self);
    pub fn close_request(&self) -> Option<CloseReason>;          // CloseReason = craft_ui_rt::CloseReason, re-exported at runtime::windows
    pub fn is_settled(&self) -> bool;                            // true on a kept node's finish
    pub fn fade_to(&self, target: f32, spec: MotionSpec);
    pub fn recall<T: Slot>(&self) -> T;
    pub fn editor_input(&mut self, keep: KeepKeys) -> Vec<craft_behavior::EditorInput>;
    pub fn ime_cursor(&self, caret: Rect);
    pub fn is_measuring(&self) -> bool;                          // replaces measure::active(ctx) on a Cx
    pub fn context(&self) -> &Context;                           // for the cx.ctx sites that pass a context on
}
```

The real `Cx` methods (`reset_part`, `layout_doc`, `link_activated`, `open_url`, `join_roving`, `focus_shape`, `fire`, `part_activated`, `context_part`, `context_requested`, `context_requests`, `roving_arrival`, `roving_lead`, `focus_part`, `focus_own`, `activation`, the chapter traits `Styled`, `Pointer`, `Focus`, `Edit`, `Accessible`, `Commands`) keep their signatures. `Cx::direction`, `density`, `motion_mode`, `frame` of the rt cut are deleted: the real code reads them through `cx.env` (tracked).

#### `MeasureCx<'a, C>`

Real fields `env`, `style`, `resolved`, `proposal`, `room`, `tokens`, `id` and all real methods stay. `ui: &'a egui::Ui` is deleted (23 sites). Text access (item 12) is through the existing methods, backed by `TextEngine` from `BuildCx::paint_services` with per-node layout reuse:

```rust
impl<C: Component> MeasureCx<'_, C> {
    pub fn wrap_limit(&self) -> Option<f32>;
    pub fn label(&self) -> Label;
    pub fn layout(&self, run: &TextRun<'_>) -> TextLayout;
    pub fn layout_at(&self, run: &TextRun<'_>, width: f32) -> TextLayout;
    pub fn least(&self, run: &TextRun<'_>) -> LeastLayout;
    pub fn field_text(&self, run: &TextRun<'_>) -> Vec2;
    pub fn layout_doc(&self, text: &crate::primitives::DocText<'_>) -> TextLayout;
    pub fn child<D: Show>(&self, key: impl Hash + Debug, child: &mut D, proposal: Proposal) -> Vec2;
    pub fn labelled<D: Show>(&self, key: impl Hash + Debug, side: &SideLabel, child: &mut D, proposal: Proposal) -> Vec2;
    pub fn face<S: Show>(&self, face: S) -> Vec2;
    /// The window's context (an owned handle): the OS, the item header's toggle names.
    pub fn context(&self) -> Context;
    // value, command, resolve_command, column_room, picture_width, picture_size, cell_width, icon, gap: unchanged
}
```

`Draw::services(&self) -> &dyn Services` gives a painter the services it lays a `DocText` out with before it paints.

`child`, `labelled` and `face` measure through the child's size cache (a kept child answers without a run). The rt `MeasureCx<'_>` (room, tokens, direction, density) is deleted.

#### `PartsCx<'a, C>` and `PartBuilder<'a>`

The real types and all their methods (15 + 27, including the Container `header_slot`/`header_part` and Collection `rect_of`/`offset`/`rows_view` impls) stay. Backed by: each `part(..)` emits a `KeptPart` (part, ui-rt part id, hit shape in node coordinates, sense, cursor, exclusive zone). `PartBuilder::cursor(impl Into<craft_behavior::Cursor>)` and `exclusive(WindowHit)` fill `KeptPart.cursor` and `.exclusive`. `PartsCx::hovered`, `focus_visible` are tracked reads of `Hovered`/`FocusVisible`. The rt `PartsCx`/`PartBuilder` are deleted.

#### `PaintUi<'a>`

Public methods (all kept, signatures unchanged):

```rust
impl PaintUi<'_> {
    pub fn draw(&self) -> Draw<'_>;
    pub fn draw_part(&self, part: impl Into<PartRef>) -> Draw<'_>;
    pub fn selectable_text(&mut self, part: impl Into<PartRef>, rect: Rect, run: &TextRun<'_>);
    pub fn clipped<R>(&mut self, rect: Rect, paint: impl FnOnce(&mut Self) -> R) -> R;
    pub fn with_style<R>(&self, variant: Variant, f: impl FnOnce(&StyleCx<'_>) -> R) -> R;
    pub fn child<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C) -> C::Output;
    pub fn inner<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C) -> C::Output;
    pub fn label_target(&mut self, key: impl Hash + Debug);
    pub fn child_with_tip<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C, tip: impl FnOnce(&mut Ui)) -> C::Output;
    pub fn labelled<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, label: Label, side: SideLabel, child: C) -> C::Output;
    pub fn face<C: Show>(&mut self, part: impl Into<PartRef>, face: C);
    pub fn face_in<C: Show>(&mut self, part: impl Into<PartRef>, face: C, ink: &Stateful<StyleColor>);
    pub fn face_faded<C: Show>(&mut self, part: impl Into<PartRef>, face: C, share: f32);
    pub fn reserve_slot(&self, clip: Rect) -> PaintSlot;
    pub fn content_painter<C: ContentPainter>(&self, cx: &Cx<'_, C>) -> ContentPainterCx<'_>;
}
```

Deleted: `slot_here`, `take_after` (item 10: `Canvas::mark`/`take_since`), the `ui: &mut egui::Ui`, `kids: Option<egui::Ui>` and `response: &egui::Response` fields. Backed by: the rt `Paint` fields (the variant's `Canvas` and `PaintRecord`, the variant's pointer state, the pointer-read flag, `EnvSnapshot`), plus `host: &'a mut (dyn UiHost + 'a)` for children (15.0). Faces record into their host's canvas at the call position (spike-v4 5d7dece500). The rt `Paint` is deleted.

#### `OverlayUi<'a>`

The real methods stay: `closing(&self, PopupHandle) -> Option<CloseReason>`, `is_rtl(&self) -> bool`, `anchor_part(&self, impl Into<PartRef>) -> Anchor`, `contain_popups_below(&mut self)`, `show<R>(&mut self, PopupHandle, Overlay, impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Option<R>`, `show_container<C: ShowContainer, R>(..)` as today. Popup open state stays on `PopupHandle::{open, close, trigger, changed, bind_open, is_open}(cx)` and `Popup::{open, toggle, opening}`. The rt `OverlayUi::{is_open, set_open, toggle, close}` are deleted. Backed by: rt layers (`rt/overlay.rs`, `PopupStates` on `Kept`), the layer body built as an inline layer child of the owner's node. `show`'s `R` is returned when the popup body ran this build or from the kept layer scope; event outputs follow 15.3 `Shown` rules inside the bridge.

#### `Container`, `BodyCx<'a>`, `ShowContainer -> Inner`

`Container` unchanged: `open`, `header(&mut self, &mut Ui, &mut BodyCx<'_>)`, `header_parts(&self, &MeasureCx<'_, Self>) -> HeaderParts`, `controls`, `footer`, `body_end`, `end`, `closed(CloseReason)`, `placement_changed(&PlacementChange)`. `Inner<O, R> { output, inner: Option<R>, settle }` with `settle`, `map_output` and `Deref` unchanged. `BodyCx<'a>` fields (`id`, `env`, `resolved`, `rect`, `memory`) and all methods unchanged.

Backed by: the container's `ComponentNode` (kept surface, header, footer, scroll parts, access); body region nodes, `ScrollNode`, the plan and size cache (`rt/container.rs`, `rt/scroll.rs`, B4). The body closure runs whenever the container's scope build reaches it; the container's own hooks are kept when key, room and reads are unchanged. The rt `Container` trait and the public `show_container` are deleted (INT-2).

#### `Collection`, `ShowCollection::show_items`, `ItemSource`

`Collection` unchanged (all items in `contract/mod.rs`, including `type Item<'r>`, `MEMBERS`, `item_layout`, `dress`, `rename_editor(&mut self, &mut Ui, Bind<'_, String>, Rect, &str)`, `header_cell(&mut self, &mut PaintUi<'_>, ..)`, `empty_height(&self, f32, &Ui, &StyleCx<'_>)`, `empty(&mut self, &mut Ui, &mut BodyCx<'_>)`). `ShowCollection::show_items<'r>(self, ui: &mut Ui, source: &dyn ItemSource<Self::Key>, item: &mut ItemFn<'r, Self>) -> Self::Output` unchanged. `ItemFn`, `ItemCx<'a, 'r, K>` (fields and methods) unchanged. `ItemSource<K>` is `craft_behavior`'s, unchanged. Node shape in 15.5.

### 15.2 `Ui`

`pub struct Ui<'u, 'c>` at `contract::Ui` (and the facade's `craft_ui::Ui`). It holds the `BuildCx`, the `Flow` (region, cursor, clip, layer, direction, measuring flag, view, sweep, scope intents), the env scope and the craft `Id` of the enclosing node, and a `Context` handle (15.3).

Methods the component crates call (counts are `ui.` calls in non-test `src` of the crates on the contract; calls on `PaintUi` and `OverlayUi` are listed under those types):

| Method | Signature | Reads | Tracked |
|---|---|---|---|
| `ctx` (343) | `fn ctx(&self) -> &Context` | the window's `Context` handle | no |
| `scope_id` (41) | `fn scope_id(&self) -> Id` | the enclosing node's craft `Id` | no |
| `id` | `fn id(&self) -> Id` | same as `scope_id` (rt name; both kept) | no |
| `pixels_per_point` (34) | `fn pixels_per_point(&self) -> f32` | the window scale cell | yes |
| `available_rect_before_wrap` (27) | `fn available_rect_before_wrap(&self) -> Rect` | `Flow` region from the cursor | no (room is part of the place) |
| `available_width` (21) | `fn available_width(&self) -> f32` | `Flow` | no |
| `available_height` (1) | `fn available_height(&self) -> f32` | `Flow` | no |
| `available_size` (1) | `fn available_size(&self) -> Vec2` | `Flow` | no |
| `set_max_width` (18) | `fn set_max_width(&mut self, width: f32)` | writes `Flow` region | |
| `set_max_height` (1) | `fn set_max_height(&mut self, height: f32)` | writes `Flow` region | |
| `set_min_size` (1) | `fn set_min_size(&mut self, size: Vec2)` | writes `Flow` extent | |
| `set_min_height` (1) | `fn set_min_height(&mut self, height: f32)` | writes `Flow` extent | |
| `is_measuring` (16, was `is_sizing_pass`) | `fn is_measuring(&self) -> bool` | `Flow.measuring` | no |
| `draw` (12, was `painter`) | `fn draw(&self) -> Draw<'_>` | records into the enclosing node's canvas at the call position | no |
| `draw_at` (4, was `painter_at`) | `fn draw_at(&self, clip: Rect) -> Draw<'_>` | same, clipped | no |
| `scope_builder` (11) | `fn scope_builder<R>(&mut self, builder: UiBuilder, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R` | a child `Flow` | no |
| `max_rect` (10) | `fn max_rect(&self) -> Rect` | `Flow` | no |
| `min_rect` (1) | `fn min_rect(&self) -> Rect` | `Flow` extent | no |
| `cursor` (3) | `fn cursor(&self) -> Rect` | `Flow` cursor as egui's (min = next position) | no |
| `clip_rect` (6) | `fn clip_rect(&self) -> Rect` | `Flow` clip | no |
| `set_clip_rect` (1) | `fn set_clip_rect(&mut self, clip: Rect)` | writes `Flow` clip | |
| `expand_to_include_rect` (5) | `fn expand_to_include_rect(&mut self, rect: Rect)` | writes `Flow` extent | |
| `advance_cursor_after_rect` (2) | `fn advance_cursor_after_rect(&mut self, rect: Rect)` | writes `Flow` | |
| `add_space` (1) | `fn add_space(&mut self, amount: f32)` | writes `Flow` | |
| `allocate` (was `allocate_space`, 1) | `fn allocate(&mut self, size: Vec2) -> Rect` | `Flow::allocate` | no |
| `push_id` (1) | `fn push_id<R>(&mut self, salt: impl Hash, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R` | id scope | no |
| `scroll_to_rect` (1) | `fn scroll_to_rect(&mut self, rect: Rect)` | queues a reveal that bubbles to the nearest `ScrollNode` (item 6) | |
| `response` (7) | `fn response(&self) -> Response` | the enclosing node's `Response`; its fact reads are tracked as for any `Response` | per fact |
| `tokens` | `fn tokens(&self) -> &Tokens` | the scope's tokens cell (read at scope entry) | yes |
| `window` | `fn window(&self) -> WindowId` | | no |
| `command_state` | `fn command_state(&mut self, command: impl Into<CommandRef>) -> CommandState` | the command's state cell | yes |
| `take_submit`, `take_clear` | `fn take_submit(&mut self) -> bool`, `fn take_clear(&mut self) -> bool` | the layer body's scope intents (B6b) | queued input |
| `clearable` | `fn clearable(&mut self, has_content: bool)` | | |
| `job` | `fn job<T, E>(&mut self, key: impl Hash, work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Pending<T, E>` | a node-owned result cell (`JobBoard`) | yes |
| `open_window`, `close_window` | `fn open_window(&mut self, spec: WindowSpec)`, `fn close_window(&mut self)` | B10 | |
| `on_command` | `fn on_command(&mut self, command: impl Into<CommandRef>, handler: impl FnMut(&Fired, &mut Store) + 'static)` | registers a handler for the node's lifetime (15.8) | |

Service methods (the `runtime::*::f(ctx)` replacements) are listed in 15.9; they are `Ui` methods unless the table says `Cx` or `Context`.

Not provided; hand-written per site (C1 leaves them, 15.11): `spacing` (4), `spacing_mut` (1), `with_layout` (3), `horizontal` (3), `layout` (2), `vertical_centered` (1), `horizontal_wrapped` (1), `allocate_exact_size` (3), `allocate_ui` (1), `layer_id` (2), `input` (2), `label` (1), `button` (1). Each becomes a craft component (`Row`, `Column`, a leaf) or a `Flow` call at its site.

`UiBuilder` is a craft type at `contract::UiBuilder`: `new()`, `max_rect(Rect)`, `id_salt(impl Hash)`, `sense(Sense)`, each `fn(self, ..) -> Self`. No `layout` (the egui `Layout` is gone; sites that set one are hand-written).

Methods ui-contract itself calls on `Ui` that are crate-private (INT-1b, INT-2 own their use; INT-3 provides them as `pub(crate)`): `layer_id` (25, becomes `layer() -> LayerKey`), `unique_id` (3), `opacity`/`set_opacity` (1), `new_child`/`child` (2), `is_enabled` (1), `expand_to_include_x` (1), `set_row_height`/`end_row` (Grid, 2), `available_size_before_wrap` (1), `build_cx` (`pub(crate) fn build_cx(&mut self) -> &mut BuildCx<'c>`).

`Env::of(ui: &Ui) -> Env` and `Env::of_root(ctx: &Context) -> Env` keep their names. `Env::of` returns an owned `Env` with public fields, so it registers a read of every env cell it copies (tokens, theme base, direction, density, motion, scale, layer frame); a write to any of them reruns every `Env::of` caller under the writing scope and nothing else.

### 15.3 `Context`, `Id`, `Shown`, `Response`, `Outcome`, `EditResponse`

#### `Context` (Q2, Q7 subset)

```rust
#[derive(Clone)]
pub struct Context { /* window id, Waker, a handle to the window root's provided services */ }

impl Context {
    pub fn window(&self) -> WindowId;                              // was viewport_id() and pass::current_viewport(ctx)
    pub fn pixels_per_point(&self) -> f32;                         // tracked inside a build, untracked outside
    pub fn request_repaint(&self);                                 // in a build: a build wake of the calling scope now; outside: the Waker
    pub fn request_repaint_after(&self, after: std::time::Duration);  // BuildCx::request_build_at
    pub fn time(&self) -> f64;                                     // BuildCx::now, untracked
    pub fn modifiers(&self) -> craft_keymap::Mods;                 // tracked
    pub fn window_focused(&self) -> bool;                          // tracked
    pub fn caps_lock(&self) -> bool;                               // tracked
    pub fn os(&self) -> Os;                                        // was windows::os_of(ctx)
    pub fn content_rect(&self) -> Rect;                            // window inner rect, tracked
    pub fn copy_text(&self, text: String);
    pub fn open_url(&self, url: &str, new_tab: bool);
    pub fn window_command(&self, command: craft_ui_rt::WindowCommand);  // BuildCx::window_command / Runtime
}
```

`Context` lives at `contract::Context` (new file `contract/context.rs`, INT-3). It is a cheap handle: cloning it is how the 93 `ctx.clone()` sites keep working. Tracked reads go through the per-thread read log the bridge drains at the end of each scope's build (as `Response` does); outside a build they are plain reads. Deleted, hand-written per site: `input(|i| ..)` (to `time`, `modifiers`, `window_focused`), `cumulative_frame_nr`, `cumulative_pass_nr`, `current_pass_index`, `request_discard`, `wants_keyboard_input`, `zoom_factor`, `native_pixels_per_point`, `send_viewport_cmd_to` (to `window_command`), `run_ui`, `set_os`, `set_embed_viewports`, `all_styles_mut`. No generic `data` store.

#### `Id` (Q1)

```rust
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(u64);
impl Id {
    pub const NULL: Id;
    pub fn new(source: impl Hash) -> Id;
    pub fn with(self, child: impl Hash) -> Id;
    pub fn value(self) -> u64;
    pub fn short_debug_format(&self) -> String;
}
pub struct IdSalt(u64);   // IdSalt::new(impl Hash), From<&str>, From<String>
```

At `contract::Id` and `contract::IdSalt` (re-exported from `primitives`; the type lives in `primitives/id.rs` so `primitives` and `runtime` use it without depending on `contract`). Hashing matches egui's (`ahash` of the source, `with` hashes `(self, child)`), so ids stay stable across the switch and saved state keyed by `Id` keeps its keys. `IdSource { Auto, Salt(IdSalt), Id(Id) }` unchanged. ui-rt keeps `NodeId`; the bridge's `Id -> NodeId` map (item 7) is per window.

#### `Shown<R>` (Q4)

```rust
#[derive(Clone, Debug)]
pub struct Shown<R> { pub value: R, pub fresh: bool }
impl<R> Shown<R> {
    pub fn value_if_fresh(self) -> Option<R>;   // event outputs: only when the closure ran in this build
    pub fn map<S>(self, f: impl FnOnce(R) -> S) -> Shown<S>;
}
```

At `contract::Shown`. `fresh` is true when the scope's closure ran in this build. Pick, result and event outputs are read only through `value_if_fresh`; state values are read from `value` either way.

#### `Response`

The rt `Response` becomes `contract::Response` (the `egui::Response` re-export goes):

```rust
#[derive(Clone, Debug)]
pub struct Response { pub id: Id, pub rect: Rect, /* input facts, node, read log link */ }
impl Response {
    pub fn clicked(&self) -> bool;
    pub fn double_clicked(&self) -> bool;
    pub fn secondary_clicked(&self) -> bool;
    pub fn long_pressed(&self) -> bool;
    pub fn drag_started(&self) -> bool;
    pub fn dragged(&self) -> bool;
    pub fn drag_stopped(&self) -> bool;
    pub fn drag_delta(&self) -> Vec2;
    pub fn interact_pointer_pos(&self) -> Option<Pos2>;
    pub fn hovered(&self) -> bool;                     // tracked: InteractFact::Hovered
    pub fn contains_pointer(&self) -> bool;            // tracked: Hovered
    pub fn is_pointer_button_down_on(&self) -> bool;   // tracked: Pressed
    pub fn has_focus(&self) -> bool;                   // tracked: Focused
    pub fn gained_focus(&self) -> bool;                // from the run's focus-change input
    pub fn lost_focus(&self) -> bool;                  // from the run's focus-change input
    pub fn union(&self, other: Response) -> Response;  // rect union; facts OR-ed; id of self
}
```

Input facts (`clicked` to `interact_pointer_pos`, `gained_focus`, `lost_focus`) come from the run's queue and are false on a settled `Cx`. Interaction facts are read per fact (15.6). Deleted: the `ctx` field (22 `response.ctx` sites become `cx.context()` or `ui.ctx()`), `layer_id`. `hover_pos`, `highlight`, `request_focus`, `context_menu`, `changed`, `mark_changed` are not `Response` methods; sites are hand-written (`request_focus` to `Ui::request_focus`).

`values.rs` keeps `clicked(&impl Outcome) -> bool`, `pressed`, `response_of`, `Hover`, `ItemOutput`, `NoValue`, `Validity`, `ValueOf`, `Bind`, `BindCheck`, `BindMut`.

#### `Outcome`, `EditResponse<T>`

`Outcome` and `EditResponse<T>` are the real ones from `values.rs` (with `unshown` and `submitted` kept); their `response` is the craft `Response`. The rt `Outcome`/`EditResponse` are deleted. `EditResponse::value(&self) -> Option<&T>` stays.

### 15.4 Show entry points

| Entry | Signature | Node |
|---|---|---|
| leaf | `Show::show(self, ui: &mut Ui) -> Self::Output`, `show_at(self, ui, rect)`, `show_with_tip(self, ui, tip: impl FnOnce(&mut Ui))`, `show_at_with_tip` | one `ComponentNode`, keyed by `IdSource` (3.2) |
| container | `ShowContainer::show<R>(self, ui: &mut Ui, body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Inner<Self::Output, R>`, plus `show_at`, `show_at_taking_room`, `show_with_tip`, `show_at_with_tip`, `measure_in` | `ComponentNode` with inline body region nodes |
| collection | `ShowCollection::show_items<'r>(self, ui: &mut Ui, source: &dyn ItemSource<Self::Key>, item: &mut ItemFn<'r, Self>) -> Self::Output` | 15.5 |
| `MemoView` | `MemoView::<P, R>::new(id: impl Hash, props: P, show: fn(&mut P, &mut Ui) -> R) -> MemoView<P, R>`; `fn show(self, ui: &mut Ui) -> Option<Shown<R>>`; bounds `P: MemoProps`, `R: Clone + PartialEq + Shift + Send + Sync + 'static` | `ScopeNode` (Lazy-wrapped); `None` when out of view |
| `Lazy` | `Lazy::new(key: impl Hash, estimate: f32) -> Lazy`, `reveal(self, bool) -> Lazy`, `show<R>(self, ui: &mut Ui, add: impl FnMut(&mut Ui) -> R) -> Option<R>` | rt lazy `ScopeNode` with kept extents |
| dialog | `show_dialog<R: Clone + 'static>(ui: &mut Ui, key: u64, overlay: &Overlay, body: impl FnMut(&mut Ui) -> R) -> Result<Shown<R>, CloseReason>` at `contract::overlay::show_dialog` | inline layer child `ScopeNode` |
| window root | `window_root_with<R>(cx: &mut BuildCx<'_>, env: WindowEnv, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R` at `runtime::windows::window_root_with` (crate-public to the host; INT-7) | the window's inline root |
| `EnvScope`, `ThemeScope` | `fn show<R: Clone + 'static>(self, ui: &mut Ui, content: impl FnMut(&mut Ui) -> R) -> R` | `ScopeNode` providing env cells |

`Lazy::show` takes `FnMut` (the rt form): a lazy body may run in two build rounds of one frame. `MemoView` props keep `&mut P` so an `Outbox` sends. `WindowEnv { cells: Option<EnvCells>, jobs: Option<JobBoard>, commands: Option<CommandBoard> }` is unchanged and public at `runtime::windows::WindowEnv`.

### 15.5 Collection node shape (INT-4)

- **The collection** is one `ComponentNode`. Its `Kept` holds the `ItemLayout` (row extents), the visible band (row range plus overscan) and the scroll offset cell. Its run reads the selection binding, the source and the room. It calls the item fn only for rows in the band.
- **Band rows** (`MEMBERS = false`): each row is a `ComponentNode` child of the collection node, ui-rt key `Keyed(hash(Key), 0)`, craft `Id = item_node(collection_id, key)`. A row outside the band has no node. A row's facts (`RowState`: selected, lead, drop_target, dragging, renaming, emphasized, current, striped) reach it only through `dress(item, &RowState)`, so they are part of its component key. A row never reads the selection model, the list's interaction or another row.
  - hover over a row: the row's `Interact` cell; a variant swap of that row; runs 0;
  - select: the list runs (its binding changed); the item fn is called for the band; only rows whose `dress`ed key moved run; the rest are kept (`finish` on a settled `Cx`);
  - wheel inside the band: the offset cell moves the content in layout; runs 0. A wheel that brings unbuilt rows into the band marks the collection `BUILD`;
  - reveal (`Collection::reveal`, `request_rename`, type-ahead) uses the kept extents; it runs the list only when the target row is outside the band.
- **Member rows** (`MEMBERS = true`): rows are parts of the collection's one node, not nodes. Each row declares its hit shape as a part `(ROW part key, row index)` with its own sense and cursor. The collection's paint records one sub-variant per row and pointer state the row's paint read (keyed by row index), so a hover over a member row repaints that row's sub-list and runs nothing. A row's `RowState` change runs the collection.
- **Selection, rename, reorder, columns, type-ahead** stay in the collection's own run (item step); the rename editor is a row-scoped child node created by `rename_editor` while renaming.

### 15.6 Pointer reads and children shown from `paint` (Q3, A7e)

These rules apply the same way in INT-1b (leaf steps), INT-2 (body chrome) and INT-4 (rows):

1. **Per-fact reads.** A read of interaction state is a read of one `InteractFact` (`Hovered`, `Pressed`, `Focused`, `FocusVisible`, `FocusWithin`). `StateSet::resolve` on a `Stateful` look reads `Hovered` only if the look has a hover entry and `Pressed` only if it has a press entry. `Styled::state()` reads all pointer facts. `Response::hovered`/`contains_pointer` read `Hovered`; `is_pointer_button_down_on` reads `Pressed`; `has_focus` reads `Focused`.
2. **Where the read happens decides its effect.** In `paint`, `paint_over`, faces, surface, state fill and veil, a `Hovered`/`Pressed` read makes the run record variants. Everywhere else (sense, update, overlay, measure, parts, access, finish, app code reading a `Response`), the read is a build read: `BuildCx::read_interaction(node, fact)` for the reading scope.
3. **Which variants.** `Rest` always. `Hover` when paint read `Hovered`. `Press` only when paint read `Pressed` and the node takes presses (its `sense` or a part's has click or drag). At most three paints per run. During a variant's recording, the pointer facts answer that variant's state.
4. **Focus in paint.** `Focused`, `FocusWithin` and selection, open, checked and disabled bits read in paint are build reads (the node reruns). `FocusVisible` is read only by the focus ring in ui-rt's paint phase (`PaintCx::interaction_fact`).
5. **Children shown from `paint`** (`PaintUi::child`, `inner`, `labelled`, `child_with_tip`, the 65 sites):
   - children are reconciled only in the first recording, the one in the current pointer state. In later recordings, the same `child` call returns `C::finish` on the child's settled `Cx` at the rect the first recording gave it, without reconciling (the keep path); `labelled` keeps its label step from the first recording too;
   - a paint that shows children still records its other looks (rule 3). The run compares each later look's children (their ids and the rects the paint gave, in order) with the first look's. While they are the same, the looks are kept and a pointer flip repaints only;
   - when a later look's children differ (a hover that shows, hides or moves a child), the run keeps the first recording alone, as its `Rest` look, and registers each pointer fact the paint read as a build read (rule 2), so the flip reruns the node;
   - faces (`face`, `face_in`, `face_faded`) are not children: they record into the host's canvas and stay variants.
   - `paint` and `paint_over` give the same result each time they run: they run once per look, so a paint never moves a prop out of the component (no `take()` of a child's props). A child's props are borrowed or rebuilt in each look; a hosted leaf (`Hosted::show`) shows its component in the first look and keeps it in the later ones (`PaintUi::keep_shown`).
6. **Access** is built once per run, outside variant recording; reads in `access` follow rule 2.

### 15.7 Style cache

- **Where.** One `StyleCache` per window, provided by the window root (`BuildCx::provide(StyleCaches(Rc<RefCell<..>>))`) and found with `BuildCx::provided::<StyleCaches>()`. It holds today's `StyleCache` (style/cache.rs), the `GroupCache` (style/cx.rs) and the app's style replacements (style/replacements.rs). All three leave `pass_state`.
- **Key.** Unchanged: `(TypeId::of::<K>(), TypeId::of::<S>(), key, EnvKey, Variant, ControlSize)`. A `ThemeScope`/`EnvScope` subtree has its own `EnvKey`, so its entries live beside the root's.
- **Clearing.** The window root records the revision of its tokens and theme-base cells when it fills the cache. When its run sees either revision moved (a theme, contrast, density or scale write), it clears the whole cache before any child runs. Nothing else clears it; entries of an `EnvKey` no longer shown stay until the next clear.
- **`StyleCx`.** Its `ctx` field and `frame_ctx()` go. `StyleCx::of_env(env: &'a Env) -> StyleCx<'a>`; `StyleCx::with_root<R>(ctx: &Context, f: impl FnOnce(&StyleCx<'_>) -> R) -> R` and `with_root_scoped<R>(ctx: &Context, scope: &ThemeScope, f) -> R` take the craft `Context`. `ContentPainterCx::egui_ctx` goes. `resolve_style` takes the cache handle instead of `&egui::Context`.

### 15.8 Commands (Q5 as decided)

One mechanism, B6b's: a fired command goes at event time to a handler that writes state cells, so only their readers rerun. No `Fired` queue is read by the build and no root `PATH` mark is made.

- **Board.** `CommandBoard` (at `runtime::commands::CommandBoard`) as in `rt/commands.rs`: `new(Keymap)`, `with_edits::<S: CommandSet>()`, `on(&self, command: impl Into<CommandRef>, handler: impl FnMut(&Fired, &mut Store) + 'static)`, `set_state(&self, cx: &mut impl Writes, command, CommandState)`, `state<C: Reads + Writes>(&self, cx: &mut C, command) -> CommandState`. One board per runtime, built by the host from the app's keymap and given through `WindowEnv::commands`.
- **Library commands** register with `board.on(..)` at setup, or from a node's run with `Ui::on_command(command, handler)`, which registers for the node's lifetime and its command scope and unregisters at `gone`.
- **App commands.** The host registers one handler for the app scope that calls `App::logic` at event time:

  ```rust
  pub trait App: 'static {
      fn input(&mut self, event: &craft_ui_rt::PlatformEvent, cx: &mut LogicCx<'_>) -> bool { false }  // true = consumed
      fn logic(&mut self, fired: &Fired, cx: &mut LogicCx<'_>) {}       // one call per fired app command, at event time
      fn frame(&mut self, cx: &mut LogicCx<'_>) -> Option<f64> { None } // before each build the driver runs, also while hidden; next wake time
      fn ui(&mut self, window: WindowId, ui: &mut Ui<'_, '_>);
      fn exit(&mut self) {}
  }

  pub struct LogicCx<'a> { /* &'a mut Store, the window registry, the job board */ }
  impl LogicCx<'_> {
      pub fn store(&mut self) -> &mut Store;            // write the cells the command changes
      pub fn request_build(&mut self, window: WindowId); // for app state outside cells: the window root runs
      pub fn window_command(&mut self, window: WindowId, command: craft_ui_rt::WindowCommand);
      pub fn quit(&mut self);
  }
  ```

  at `craft_ui::app::{App, LogicCx, run}` (INT-7). The host holds the app in an `Rc<RefCell<A>>`; events are routed before the build in `Driver::step_with`, so the handler's borrow never overlaps the roots closure's.
- **Firing from code.** `Cx::fire(command, source) -> bool` and `Ui::fire(command, source) -> bool` run the handler at once (the board looks the command up in the node's scope chain).
- **Reading state.** `Ui::command_state(command) -> CommandState` (tracked). `Commands::publish(command, state)` writes the state cell. `Commands::resolve_command` resolves through the board.
- **`take_command`, `take_step`** (58 crate sites): see "Open for the lead" item 1.

### 15.9 Service mapping (INT-3)

Every service free function `f(ctx, ..)` of ui-contract called from the component crates, with its replacement. "Ui" means a method on `contract::Ui`; the same method exists on `CxCore` where a hook needs it (marked "Ui, Cx"). Counts are crate call sites. Signatures drop the `ctx` argument and keep the rest unless stated.

| Today | Sites | Replacement | Backed by |
|---|---|---|---|
| `frame::request_repaint(ctx)` | 25 | `Context::request_repaint()` | build wake |
| `frame::request_repaint_after(ctx, secs: f64)` | 8 | `Context::request_repaint_after(Duration)` | `request_build_at` |
| `frame::clock(ctx) -> f64` | 4 | `Context::time()` | `BuildCx::now` |
| `frame::running(ctx)` | 4 | deleted; sites use `Ui::is_measuring()` or drop | |
| `frame::discard_pass`, `discardable`, `will_rerun` | 9 | deleted (no discard passes); site code removed | |
| `measure::active(ctx)` | 21 | Ui, Cx `is_measuring()` | `Flow.measuring` |
| `measure::effect(ctx, what)` | 10 | `!is_measuring()` at the site | |
| `layout_memory::text`, `emit_text` | 2 | deleted; the node size cache | |
| `pass::current_viewport(ctx)` | 7 | `Context::window()` | |
| `pointer::frame(ctx) -> PointerFrame` | 14 | Ui, Cx `pointer() -> PointerFrame` (tracked read of the window pointer cell; `Cx::pointer` as well) | ui-rt input |
| `pointer::mods(ctx)` | 14 | `Context::modifiers()` | tracked |
| `pointer::hover_point_in(ctx, rect)` | 4 | Ui `hover_point_in(rect) -> Option<Pos2>` (tracked) | |
| `pointer::hovers(ctx, id)` | 1 | Ui, Cx `hovers(id) -> bool` | `read_interaction(node_of(id), Hovered)` |
| `pointer::pressed_part(ctx, owner)` | 2 | Ui `pressed_part(owner: Id) -> Option<PartRef>` | owner's `Kept.press` |
| `pointer::shapes_at(ctx, at)` | 1 | Ui `shapes_at(at) -> Vec<(Id, Option<PartRef>)>` | ui-rt hit test on kept shapes, untracked |
| `pointer::with_pass_pointer`, `withhold_members` | 3 | deleted (no pass pointer, no arena) | |
| `primitives::cancel_press_of(ctx, owner)` | 1 | Ui `cancel_press_of(owner: Id)` | ui-rt press machine |
| `primitives::member_gesture_running` | 1 | `PartsCx::gesture_running` | |
| `cursor::request`, `request_within` | 3 | `Component::cursor`, `PartBuilder::cursor`; code without a component: Ui `set_cursor(Cursor)` | `Kept.cursor` |
| `focus::request(ctx, FocusTarget) -> bool` | 22 | Ui, Cx `request_focus(target: FocusTarget) -> bool` | `rt/focus.rs` |
| `focus::focus_in_scope(ctx, scope) -> Option<bool>` | 2 | Ui, Cx `focus_in_scope(scope: Id) -> Option<bool>` | tracked `FocusWithin` |
| `focus::focused_rec(ctx)` | 3 | Ui, Cx `focused_stop() -> Option<StopRec>` (tracked through the focused-node cell) | `FocusKept` |
| `focus::is_focused(ctx, id)` | 2 | Ui `is_focused(id) -> bool` | tracked `Focused` |
| `focus::open_scope(ctx)` | 2 | Ui, Cx `open_focus_scope() -> Option<Id>` (tracked through the open-layer cell) | |
| `focus::registration::drive(ctx, field, list, empty)` | 1 | Cx `drive_list(list: Id, empty: bool)`: while the field has focus its list keys go to the collection `list` (Up, Down, Page Up, Page Down, Enter; Home, End, Left and Right while empty), which takes them as its own moves: Left and Right only as a tree, Enter only with a lead row, and a key it declines goes on up the field's path | `FocusKept::{drives, list_keys}`, ui-rt `EventCx::forward_to` |
| `focus::return_from_pick(ctx, picked)` | 2 | Ui, Cx `return_from_pick(picked: PickedIn)` (Cx: recorded, applied after the run before its focus requests) | `set_focus_return` |
| `focus::running_menu(ctx)` | 2 | Ui, Cx `running_menu() -> Option<Id>` | |
| `focus::focus_came_with_open(ctx)` | 1 | Cx `focus_came_with_open() -> bool` (tracked through the focused-node cell) | ui-rt `FocusCause::Entered`, `FocusKept::entered` |
| `focus::keyboard_in_menu(ctx)` | 3 | Cx `keyboard_in_menu() -> bool` (tracked: focused node, its `FocusVisible`) | |
| `overlay::pressed_outside(ctx, id)` | 1 | Cx `pressed_outside() -> bool` (a press outside one of the component's `Outside::Keep` popups since its last run) | ui-rt `OutsidePress::Report`, `Event::PressedOutside` |
| `drag::payload::<P>(ctx)` in `parts` | 1 | `PartsCx::dragging::<P>() -> Option<(Arc<P>, Pos2)>`, as `Pointer::dragging` | `RunIo::drag_over` |
| `keys::capture`, `claim`, `claim_alt_tap` | 6 | `KeyUse::claims` on the component (kept); window-wide while facts the run reads hold: Cx `capture_in_window(chord: Chord)` (a dialog footer's button keys) | `rt/keys.rs` |
| `keys::take_claimed(ctx, id, chord)`, `take_alt_tap` | 8 | `NodeInput::Chord` through Cx `intents()` | |
| `keys::held(ctx)`, `keyed(ctx)` | 5 | `Context::held_keys() -> Vec<Key>`, `Context::keyed() -> bool` (tracked) | |
| `keys::intents_of(ctx, id, rtl)`, `settle` | 4 | Cx `intents()` | B6b queue |
| `keys::take_submit(ctx)` | 1 | Ui `take_submit()` | scope intents |
| `keys::with_canvas_window_intents` | 2 | the canvas node's `KeyUse`; its intents through Cx `intents()` | |
| `lock_keys::caps_lock(ctx)` | 2 | `Context::caps_lock()` | tracked |
| `commands::fire(ctx, cmd, source)` | 3 | Ui, Cx `fire(cmd, source) -> bool` | `CommandBoard` |
| `commands::resolve(ctx, cmd)` | 6 | Ui `resolve_command(cmd) -> Arc<ResolvedCommand>` | |
| `commands::shown(ctx, cmd)` | 1 | Ui `command_shown(cmd: &CommandRef) -> Option<Shown>` (`commands::Shown`) | |
| `commands::holds(ctx, layer, rect, cmds, label)` | 1 | Ui `hold_commands(layer, rect, cmds, label)` | |
| `commands::holder(ctx, chord)` | 1 | Ui `command_holder(chord: &Chord) -> Option<CommandRef>` | |
| `commands::keymap_generation(ctx)` | 1 | Ui `keymap_generation() -> u64` (tracked) | |
| `commands::declare_child(ctx, child, parent)` | 1 | Ui `declare_command_scope(child, parent)` | |
| `commands::note_page_root(ctx, viewport, root)` | 1 | deleted: the window root is the page root | |
| `commands::fire_beneath`, `resolve_beneath` | 2 | Ui `fire_beneath(layer, cmd, source) -> bool`, `resolve_beneath(layer, cmd)` | |
| `memory::with_session(ctx, id, f)` | 8 | Ui `with_session<T: Slot, R>(id: Id, f: impl FnOnce(&mut T) -> R) -> R` | host store, by `Id` |
| `memory::set_saved_value(ctx, key, value)` | 1 | Ui `set_saved_value<T: Savable>(key: &SaveKey, value: T)` | host store |
| `derived::derived(ctx, id, key, build)` | 3 | Ui `derived<K: MemoProps, V: Send + Sync + 'static>(key: &K, build: impl FnOnce() -> V) -> Arc<V>` | node slot |
| `overlay::raise(ctx, id)` | 2 | Ui `raise_layer(id)` | rt layers |
| `overlay::opening(ctx, id)`, `is_open`, `entry` | 4 | Ui `layer_opening(id)`, `layer_open(id)`, `layer_entry(id) -> Option<OpenEntry>` (tracked) | |
| `overlay::any_open(ctx, filter)` | 2 | Ui `any_layer_open(filter: OpenFilter) -> bool` (tracked) | |
| `overlay::top`, `drawing_parent`, `drawing_top_modal`, `is_top_modal`, `running_dialog` | 6 | Ui `top_layer()`, `layer_parent()`, `in_top_modal()`, `is_top_modal(id)`, `running_dialog()`; Cx `in_top_modal()`, `is_top_modal(id)`, `running_dialog()` (one function each with Ui's, `rt/reads.rs`) | |
| `overlay::request_close(ctx, id, why)` | 1 | Ui, Cx `close_layer(id, why: CloseReason)` (Cx: recorded, applied after the run) | |
| `overlay::end_stack_dialog`, `stack_place`, `take_keys_for_new_dialog` | 7 | Ui `end_stack_dialog(id)`, `stack_place(place, id, show)`; `take_keys_for_new_dialog` deleted (layer scopes take keys) | |
| `contract::overlay::opened_from(ctx, opener)` | 1 | Ui `opened_from(opener: Id) -> bool` | |
| `toast::post`, `cards`, `dismiss`, `end`, `keep`, `set_progress`, `shown` | 16 | Ui `toasts() -> ToastQueue` and its methods | `rt/toast.rs` |
| `drag::start(ctx, source, env, payload, also, preview)` | 2 | `Component::drag_source`; code without a component: Ui `start_drag(source, payload, also, preview)` | ui-rt payload session |
| `drag::started`, `cancel`, `mark`, `moves`, `next_ordinal`, `winning`, `sourced_by`, `cross_window_pointer` | 11 | Ui `drag_started()`, `cancel_drag(reason)`, `drop_mark()`, `drag_moves()`, `drag_ordinal(owner)`, `drag_winning(owner, ordinal)`, `drag_sourced_by(source)`, `drag_pointer()` | `rt/drag.rs` |
| `edit::next_gesture_id(ctx)` | 1 | Ui, Cx `next_gesture_id() -> GestureId` | |
| `edit::cancel_all(ctx, reason)` | 1 | Ui `cancel_edits(reason)` | |
| `edit::watch_gestures(ctx, body)` | 1 | Ui `watch_gestures<R>(body: impl FnOnce() -> R) -> (R, bool)` | |
| `inspect::recorded`, `pointed`, `point_at` | 5 | Ui `inspect_recorded(key)`, `inspect_pointed(key)`, `inspect_point_at(key, index)` | |
| `access::report_sub(ctx, decl)` | 2 | the component's `access` hook (`Access` sub-nodes); hand-written | `Kept.access_subs` |
| `ghost::readout(ctx, env, at, rows)` | 3 | Ui `ghost_readout(at, rows)` | drag layer |
| `layout::ambient_width(ctx, column, content)` | 3 | Ui `ambient_width(column, content) -> Option<f32>` | |
| `sampler::pass_tool_gesture(ctx, canvas, gesture)` | 1 | Ui `pass_tool_gesture(canvas, gesture)` | |
| `contract::request_rename(ctx, collection, key)` | 1 | Ui `request_rename<K: Clone + Send + Sync + 'static>(collection: Id, key: &K)` | |
| `Env::of_root(ctx)` | 24 | `Env::of_root(&Context)` (same name) | root env cells |
| `StyleCx::of_env(ctx, env)` | 6 | `StyleCx::of_env(env)` | |
| `StyleCx::with_root`, `with_root_scoped` | 6 | same names, craft `Context` | 15.7 |
| `WindowHost::of(ctx)` and its methods | 40 | Ui `window_host() -> WindowHost`, method for method on `WindowRegistry`; `current()` to `Context::window()`; `act(window, WindowAction)` to `Context::window_command`; `inject_*` and `screenshot*` to platform injection (C2, hand-written) | B10 |
| `windows::os_of(ctx)` | 2 | `Context::os()` | |
| `windows::shell_of(ctx, viewport)` | 2 | Ui `shell() -> ShellInfo` | |
| `windows::logic`, `hold_close`, `root_close_requested`, `route_root` | 4 | `Component::close_guard`, Cx `close_request()`; host side in INT-7 | B10 |
| `TextureCache::texture(ctx, ..)`, `TextureStore::in_context`, `Thumbnails::in_context` | 20 | `TextureCache` by `TextureKey` (Q10); code with a `Context` and no build: `TextureStore::in_context(&Context, salt, TextureStoreOptions)`, `Thumbnails::in_context(&Context, salt, ThumbnailOptions)`; reads take `&mut impl TextureReads`: `BuildCx`, `CxCore` (hooks; effects when the node's build ends), `&MeasureCx` (`measure`, at once through the run's `Ui`); `TextureStoreOptions::packed_small` and `CachedTexture::uv()` deleted (`key` is a field, the uv the unit rect) | `ImageStore` |
| `ThumbSource::revision(key, request)` | | `ThumbSource::key_revision(&self, key: &K) -> Revision` (required: the version of the key's pixels at every request, which a kept view's version includes); `revision(key, request)` is provided and defaults to it | |
| ListRow thumbnail painters | | `ThumbPicture::new(read: FnOnce(&mut CxCore, &ThumbRequest) -> TextureState, paint: FnOnce(&ContentPainterCx, Rect, &TextureState))`, `ThumbPicture::painter(FnOnce(&ContentPainterCx, Rect))`; `ThumbPaint<'a> = Versioned<ThumbPicture<'a>>`; `ListRow::thumb_picture(size, ThumbPaint)`, `extra_thumb_picture(ThumbPaint)`; RowThumb reads in `update`, paints the state in `paint` | |
| `Picture::new(label, key, &dyn Fn() -> ColorImage)` | | `Picture::new(label, key, &dyn Fn() -> RgbaImage)`; Picture reads its texture in `update` and paints the kept state; the `DevicePixels` measure reads through `&MeasureCx` | |
| `SubNode::text_run(text, positions, widths, rtl)` | | same signature; the rt access writer makes the sub-node a `TextRun` with the text as its value and AccessKit's character positions, widths and text direction | |
| `testing::StoryPictures::of(ctx)` | 10 | `StoryPictures::of(ui: &Ui)` | Harness-provided value |
| `replay::journal` | 1 | deleted | |
| `install`, `pen::install` | 3 | deleted; `craft_ui::app::run` (INT-7) | |

The `CxCore` reads and requests above reach the run's build, which the run's hosts share (`RunIo::build`); each is the one function `Ui`'s method calls (`rt/reads.rs`, `rt/ids.rs`, `overlay::close_layer`). A hook given a `Ui` (`content`, `overlay`) reads through the `Ui`; measuring and settled runs have no build, so their reads answer as for no pointer, no menu and no focus, and their requests are dropped. Collection members' runs have none either.

Out of scope for INT-3 (crate-local services that take a context; their owners port them in P2 to P6): ui-panels `native::*` and `cross_window::*` (docking floats, onto B10), ui-shell `embed::*`, `viewports::*`, `title_bar::*` (INT-7, P2), ui-widgets `outer::keep`, `menu::any_open`, `bar::is_open`, ui-canvas `zoom_motion::steer`, color-ui `capture::store`, `craft_quit::on_menu_event`.

### 15.10 Survivor table (final)

| Concept | Survives at | Re-backed by | Deleted |
|---|---|---|---|
| `Component`, `Meta`, `Show*` | `contract/mod.rs` (15.1) | `contract/rt/run.rs` driver, real steps (`contract/run.rs`, `run/typed.rs`, `steps/*`) | rt trait cut in `rt/run.rs`, `rt::tooltip()`, `rt/skeleton.rs` (moves to `tests/rt` as test-local types) |
| `STOPS`/`KEYS` | `contract/mod.rs` | `Kept.focus.stop` (`rt/focus.rs`) | rt `keys() -> Option` |
| `Cx`, `CxCore` | `contract/cx/context.rs` | `Kept`, `EnvSnapshot`, B6b queue, `Memory` | `CxCore.ctx`, rt `Cx` |
| `MeasureCx` | `contract/cx/measure.rs` | rt room, tokens, env; `TextEngine` | `MeasureCx.ui`, rt `MeasureCx` |
| `PartsCx`, `PartBuilder` | `contract/cx/parts.rs` | `KeptPart` | rt copies |
| `PaintUi` | `contract/paint_ui.rs` | rt `Paint` fields, variant recording, `UiHost` | rt `Paint`, `slot_here`, `take_after` |
| `OverlayUi`, `PopupHandle`, `Overlay` | `contract/overlay/*` | `rt/overlay.rs` layers, `PopupStates` | rt `OverlayUi` open/close methods, rt `PopupHandle` |
| `Response`, `Outcome`, `EditResponse` | `contract/values.rs` (`Response` from `rt/response.rs`) | read log, `InteractFact` | `egui::Response`, rt `Outcome`/`EditResponse` |
| `Container`, `BodyCx`, `Inner` | `contract/mod.rs`, `contract/cx/body.rs`, `contract/container.rs` | `rt/container.rs`, `rt/scroll.rs`, plan and size cache | rt `Container`, public `show_container` |
| `Collection`, `show_items`, `ItemCx` | `contract/mod.rs`, `contract/collection*`, `contract/items*` | new `rt/items.rs` (15.5) | egui items paths |
| `Pattern`, `StackDialog` | unchanged (take the craft `Ui`) | | |
| `MemoView`, `Outbox` | `contract/memo_view.rs` (returns `Option<Shown<R>>`) | `rt/scope.rs` `ScopeNode` | rt `MemoView` |
| `Lazy` | `contract/lazy.rs` (body `FnMut`) | `rt/lazy.rs` | `runtime/lazy.rs` transcripts |
| `show_dialog` | `contract/overlay` (15.4) | `rt/dialog.rs` | |
| `Shown` | `contract::Shown` (new, `contract/values.rs`) | | |
| `Ui`, `UiBuilder` | `contract::Ui` (from `rt/ui.rs`), `contract::UiBuilder` | `BuildCx`, `Flow` | `egui::Ui` |
| `Context` | `contract::Context` (`contract/context.rs`, new) | `BuildCx` services, `Waker` | `egui::Context` |
| `Id`, `IdSalt`, `IdSource` | `primitives/id.rs` (new), re-exported from `contract` | | `egui::Id` |
| `Env::of`, `Env::of_root`, `EnvView` | `runtime/env.rs`, `runtime/env/view.rs` | `rt/env.rs` cells, tracked | env walked from the egui ctx |
| `EnvScope`, `ThemeScope`, `LayerFrame`, `EnvCells`, `ThemeBase` | `runtime::env` | `rt/env.rs` | `contract::rt::{EnvScope, ..}` paths |
| `StyleCx`, style cache | `style/cx.rs`, `style/cache.rs` | window-root provided `StyleCaches` (15.7) | `StyleCx.ctx`, `frame_ctx`, `ContentPainterCx::egui_ctx`, `pass_state` tier |
| `CommandBoard`, `Commands`, `CommandSet`, `Fired`, `CommandState` | `runtime::commands` | `rt/commands.rs` | `runtime/commands` board, queue and per-frame state (INT-5a) |
| `DragSource`, `DropRule` | `contract::{DragSource, DropRule}` | `rt/drag.rs` | `contract::rt::*` paths |
| `ImePurpose` | `contract::ImePurpose` (re-export of `craft_ui_rt::ImePurpose`) | `rt/ime.rs` | |
| `JobBoard`, `Pending`, `Delivery` | `runtime::pending` | `rt/pending.rs` | `runtime/pending.rs` polling |
| `TextureCache`, `TextureKey`, `TextureLook` | `primitives::textures` | `rt/textures.rs` | the egui `TextureCache`/`TextureStore`/`TextureHandle` |
| `ToastHost`, `ToastQueue` | `contract::toast` | `rt/toast.rs` | `runtime/toast.rs` |
| `NodeInput`, `Pointer` | `contract::{NodeInput, Pointer}` (hidden; tests and the editor read them) | `rt/node.rs` | |
| `WindowEnv`, `window_root_with`, `WindowRegistry` | `runtime::windows` | `rt/scope.rs`, `rt/windows.rs` | `runtime/windows/{host,native,viewport,placement,shell}.rs` |
| `Harness` | `testing::Harness` (from `rt/harness.rs`) | headless platform | `testing::Fixture`, `replay_trials`, egui_kittest |
| `App`, `LogicCx`, `run` | `craft_ui::app` (ui-shell `shell/app*`) | `Driver::step_with` | `runtime/{frame,install}.rs`, `ui-shell/src/shell/embed.rs` |
| derive(Component) | ui-macros | | the generated `egui::Widget` impl (`ui-macros/src/component.rs:563`) |

### 15.11 C1 rename table

C1 (`tools/codemod-v4`, syn) applies these to every `.rs` under a crate's `src`, `tests`, `benches` and `examples`. Each rule is idempotent.

| # | Source pattern | Target |
|---|---|---|
| 1 | `use egui::{.., Ui, ..}`, `egui::Ui` in paths | `craft_ui::Ui` (inside ui-contract: `crate::contract::Ui`) |
| 2 | `egui::Response` | `craft_ui::Response` |
| 3 | `egui::Id`, `egui::IdSalt` | `craft_ui::Id`, `craft_ui::IdSalt` |
| 4 | `egui::Context` | `craft_ui::Context` |
| 5 | `egui::{Pos2, Rect, Vec2, Align, pos2, vec2, Rangef, lerp}` | `craft_ui::geom::{..}` (re-export of `craft_ui_rt`'s `emath` items; `Rangef` and `lerp` added to that re-export) |
| 6 | `egui::Color32`, `egui::CornerRadius` | `craft_ui::{Color32, CornerRadius}` (see Open item 3) |
| 7 | `egui::Key`, `egui::Modifiers` | `craft_ui::input::{Key, Mods}`; `Modifiers::NONE`/`COMMAND`/`SHIFT`/`ALT`/`CTRL` to `Mods::NONE`/`COMMAND`/`SHIFT`/`ALT`/`CTRL` (`craft_keymap`) |
| 8 | `egui::PointerButton`, `egui::CursorIcon` | `craft_ui::input::{PointerButton, CursorIcon}` (`craft_behavior`) |
| 9 | `egui::ViewportId`, `egui::ViewportId::ROOT` | `craft_ui::WindowId`, `craft_ui::WindowId::ROOT` (Open item 4) |
| 10 | `egui::ViewportCommand::{Title, InnerSize, MinInnerSize, OuterPosition, Visible, Decorations, WindowLevel, Minimized, Maximized, Fullscreen, Focus, StartDrag, Close}` | `craft_ui::WindowCommand::{Title, Size, MinSize, Position, Visible, Decorations, Level, Minimized, Maximized, Fullscreen, Focus, DragMove, Close}` |
| 11 | `egui::UiBuilder::new().max_rect(r)` and `.id_salt(s)` chains | `craft_ui::UiBuilder::new().max_rect(r).id_salt(s)`; a chain with `.layout(..)` is left (hand-written) |
| 12 | `ui.is_sizing_pass()` | `ui.is_measuring()` |
| 13 | `ui.painter()` | `ui.draw()` |
| 14 | `ui.painter_at(r)` | `ui.draw_at(r)` |
| 15 | `ui.allocate_space(s)` | `ui.allocate(s)` |
| 16 | `ui.ctx().viewport_id()`, `pass::current_viewport(ctx)` | `ui.ctx().window()`, `ctx.window()` |
| 17 | `ctx.request_repaint()` | unchanged (same name on `Context`) |
| 18 | `frame::request_repaint(c)`, `frame::request_repaint_after(c, s)` | `c.request_repaint()`, `c.request_repaint_after(std::time::Duration::from_secs_f64(s))` |
| 19 | `measure::active(c)` | `c.is_measuring()` when `c` is `ui.ctx()` (rewritten to `ui.is_measuring()`) or `cx.ctx`/`&cx.ctx` (to `cx.is_measuring()`); other receivers left |
| 20 | `StyleCx::of_env(c, e)` | `StyleCx::of_env(e)` |
| 21 | the 15.9 rows marked "same name" or with a plain receiver move (`f(ui.ctx(), a..)` to `ui.f2(a..)`, `f(&cx.ctx, a..)` to `cx.f2(a..)`) | as listed in 15.9 |
| 22 | `cx.ctx` passed on as a context (`&cx.ctx`, `cx.ctx.clone()`), `response.ctx` | `cx.context()`, `cx.context().clone()`; `response.ctx` left (hand-written) |
| 23 | `craft_ui::testing::Fixture` and `Fixture::new(f)` | `craft_ui::testing::Harness`, `Harness::new_ui(f)` |
| 24 | Fixture methods with a Harness twin: `frame(vec![])` to `step()`, `idle(n)`, `settle()`, `advance(s)`, `by_label`, `by_role`, `click`, `click_at`, `press`, `release`, `drag`, `drag_at`, `hold(m)`, `key(k, m)`, `type_text`, `access(kind, t)` to `access(action, t)`, `focused()` | same names on `Harness` (`Modifiers` args per rule 7) |

C1 must not touch (hand-written per site, plan 1.3):
- `runtime::*::f(ctx)` calls whose replacement in 15.9 is "deleted", "hand-written" or changes arguments beyond dropping `ctx`;
- textures: `TextureHandle`, `ColorImage`, `load_texture`, `SizedTexture`, `TextureId`, `TextureOptions` (about 135 sites);
- raw egui paint: `egui::Shape`, `Painter`, `Stroke`, `Mesh`, `epaint::*`, `text_layout::on_painter` (about 45);
- viewport calls other than rule 10 (`ViewportBuilder`, `ViewportInfo`, `send_viewport_cmd_to`, about 70; automation 39);
- egui widgets and layout helpers: `ui.label`, `ui.button`, `ui.horizontal*`, `ui.vertical*`, `ui.with_layout`, `ui.layout()`, `ui.spacing*`, `ui.allocate_exact_size`, `ui.allocate_ui`, `egui::Layout`, `Align2`, `egui::Grid` (6 in the crates, 195 in cloud-ui);
- `ctx.input(..)` and `ui.input(..)` reads (about 20), `egui::Event`, `RawInput`, `TouchPhase`, `MouseWheelUnit`;
- the 3 picture-slot sites (`slot_here`, `take_after`);
- `response.ctx`, `response.layer_id`, `.hover_pos()`, `.highlight()`, `.request_focus()`, `.context_menu()`;
- struct fields that hold a `Ui` (`&'a mut Ui`, `&'a Ui`: 14 sites) and need `Ui<'u, 'c>`;
- `take_command`/`take_step` sites (Open item 1);
- Fixture methods without a Harness twin (`frame_shapes*`, `take_paint`, `texts`, `shape_bounds`, `ctx`, `cursor_icon`, `story*`, `hosted`, `probe_purity`) until INT-6a adds them;
- craft-cloud-ui and craft-update (Q6: ported after P1).

### 15.12 File ownership for group (b)

Each task edits only its files. A shared file lists who owns which item; a task that needs a one-line addition in another task's file (a `Kept` field, a re-export) makes the smallest edit and names it in its report; the lead merges.

| Task | Files owned |
|---|---|
| INT-1a | `contract/mod.rs` (traits, re-exports), `contract/values.rs` (`Response`, `Shown`, `Outcome`, `EditResponse`), `contract/paint_ui.rs`, `contract/cx.rs`, `contract/cx/*`, `rt/response.rs`, `rt/key.rs`, `primitives/id.rs` (new: `Id`, `IdSalt`) |
| INT-1b | `contract/run.rs`, `contract/run/typed.rs`, `contract/steps.rs`, `contract/steps/*`, `rt/run.rs`, `rt/node.rs`, `rt/skeleton.rs` (moved to tests), `rt/cursor.rs`, `rt/sweep.rs`, `rt/focus.rs` (STOPS/KEYS), `tests/rt/{main,input,access,content}.rs` |
| INT-2 | `contract/body.rs`, `contract/body/*`, `contract/container.rs`, `contract/lazy.rs`, `contract/memo_view.rs`, `layout/*`, `rt/container.rs`, `rt/scroll.rs`, `rt/lazy.rs`, `rt/scope.rs` (except `window_root_with`), `rt/dialog.rs`, `runtime/lazy.rs`, `runtime/shift.rs`, `tests/rt/layout.rs` |
| INT-3 | `rt/ui.rs`, `contract/context.rs` (new), `runtime/env.rs`, `runtime/env/*`, `runtime/memory.rs`, `runtime/derived.rs`, `runtime/requests.rs`, `runtime/inspect.rs`, `style/cx.rs`, `style/cache.rs`, `style/replacements.rs`, `primitives/content.rs`, `rt/memory.rs` |
| INT-4 | `contract/items.rs`, `contract/items/*`, `contract/collection.rs`, `contract/collection/*`, `rt/items.rs` (new), `tests/rt/items.rs` (new) |
| INT-5b | `rt/access.rs`, `rt/env.rs`, `rt/overlay.rs`, `rt/tip.rs`, `rt/toast.rs`, `rt/textures.rs`, `rt/pending.rs`, `primitives/canvas.rs`, `primitives/textures/*`, `contract/overlay/*` (popups and dismissal only as item 9 needs) |
| INT-6a | `testing/*` (not the `it` content), `rt/harness.rs`, `crates/story-render`, `crates/ui-test-kit` |
| INT-7 | `ui-shell/src/shell/app*.rs`, `ui-shell/src/shell/embed.rs` (deleted), `ui-shell/Cargo.toml`, `ui-gallery/src/main.rs`, `rt/scope.rs` (`window_root_with` only), `rt/windows.rs`, `rt/commands.rs` (`App` handler registration only), `crates/ui/src/lib.rs` (the `craft_ui::app` module and facade re-exports) |
| C1 | `tools/codemod-v4/*`, `crates/design-lint` (`app/no-egui`) |

Shared items:

| File | Item | Owner |
|---|---|---|
| `contract/mod.rs` | trait text, `Show*`, `IdSource`, re-exports for 15.10 | INT-1a |
| `contract/mod.rs` | `ShowCollection`, `ItemFn` | INT-4 (body of `show_items` only) |
| `rt/mod.rs` | module list and visibility (all private, 15.0) | INT-1a; others add their own module line |
| `rt/node.rs` | `Kept`, `ComponentNode`, `ScopeNode` | INT-1b; INT-2, INT-4, INT-5b add fields with one-line edits |
| `rt/ui.rs` | `Ui`, `Flow` | INT-3; INT-2 adds `Flow` fields only through `rt/container.rs` structs |
| `rt/scope.rs` | `MemoView`, scopes | INT-2; `window_root_with` is INT-7's |
| `rt/commands.rs` | board | INT-7 (app handler), INT-3 (`Ui::on_command`, `command_state`) |
| `contract/overlay/*` | `OverlayUi`, `show_dialog` path | INT-2 (`show_dialog` returning `Shown`), INT-5b (popup frame, item 9) |
| `runtime/commands/*` | `take_command`/`take_step` re-backing | INT-1b (per Open item 1) |
| `crates/ui-contract/Cargo.toml` | features | INT-1a removes the `rt` feature; egui stays until INT-5a |

### 15.13 Open for the lead

1. **`take_command`/`take_step` under Q5** (58 sites: imaging 12, ui-patterns 6, ui-widgets 3, ui-shell 3, ui-canvas 3, text 3, and others). The decision removes the build-time `Fired` queue, but these sites pull commands for a scope inside a component's or panel's build. Recommendation: keep both names; re-back them as a read of a node-owned cell. The command scope a node declares registers a board handler (`Ui::on_command` per `CommandSet` scope) that writes the fired command into that cell; `take_command` reads and clears it. The read is tracked, so only the declaring node reruns, with no root `PATH` mark. The alternative (rewrite each site as a handler) changes 58 sites by hand in P1-P6.
2. **`App` signature** (15.8). The decision names `App::logic` as the app's handler. The freeze splits the old per-frame role into `App::frame` and makes `logic(&mut self, &Fired, &mut LogicCx)` the event-time handler. Recommendation: accept as frozen.
3. **`Color32` and `CornerRadius`** (169 and 113 crate uses; `Draw` takes them). Dropping egui from ui-contract needs a home for both. Recommendation: `ecolor::Color32` (no egui dependency) re-exported at `craft_ui::Color32`, and a craft `CornerRadius` in `primitives` with the subset the crates use (`ZERO`, `same(u8)`, the four `u8` fields, `From<u8>`), so ui-contract needs no `epaint`.
4. **`WindowId::ROOT`.** ui-rt window ids are allocated at open time; the crates name `ViewportId::ROOT` (12 sites). Recommendation: `WindowId::ROOT` names the host's first window (the host opens it first, so its id is fixed); `Context::window()` answers the current one.
5. **`show_dialog` result.** Q4 says `show_dialog -> Shown<R>`; the rt form returns `Result<R, CloseReason>` for a dismissal. Frozen as `Result<Shown<R>, CloseReason>`. Recommendation: accept.
6. **`Env::of` read granularity.** `Env` has public fields, so `Env::of` registers a read of every env cell it copies (15.2). A density write reruns all `Env::of` callers under the writing scope. INT-3's trial ("reruns only readers") holds with "readers" meaning `Env::of` callers. Recommendation: accept; per-field tracking would need accessor methods on `Env`, a later change.
7. **`Ui::cursor` return type.** egui returns a `Rect`, the rt cut a `Pos2`. Frozen as egui's `Rect` (3 crate sites keep compiling); the bridge's internal uses move to `Flow::next_min`. Recommendation: accept.

#### Lead decisions on 15.13 (27 Nov)

1. **`take_command`/`take_step`: accepted with one change.** The node-owned cell holds the last fired command with a sequence number; the board handler the node's command scope registers writes it at event time. `take_command` returns the command when its sequence is newer than the last one this node consumed, and records the consumed sequence in the node's memory (not in a cell). The take never writes the cell it read, so it causes no same-frame rerun; only the declaring node reruns on a fire. INT-1b implements it in `runtime/commands/*` on `CommandBoard::on`.
2. **`App`: accepted as frozen** (`logic` at event time, `frame` for the per-frame role).
3. **`Color32`, `CornerRadius`: accepted.** `ecolor::Color32` re-exported at `craft_ui::Color32`; a craft `CornerRadius` in `primitives` (INT-1a; render backends convert).
4. **`WindowId::ROOT`: accepted.**
5. **`show_dialog -> Result<Shown<R>, CloseReason>`: accepted.**
6. **`Env::of` granularity: accepted.** Per-field env reads are a later change, recorded in spike-int-items.md.
7. **`Ui::cursor -> Rect`: accepted.**

Also fixed: `Ui::on_command` called again in a later run of the same node replaces that node's handler for the command (keyed by node and command); it never adds a second registration.
