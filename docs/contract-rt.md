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

## 10. Services

| Service | Today | On ui-rt |
|---|---|---|
| Sensing (`primitives/sensing.rs`, `runtime/pointer.rs`, `wheel.rs`) | own hit test over shapes registered per pass, `PressMachine` per root and arena, pointer snapshot | ui-rt hit test on kept shapes and its `PressMachine`; the node queues gestures; arena members are parts of their root's node with sub-hits |
| Focus (`runtime/focus*`) | stops, scopes and roving groups registered per pass; transcripts for out-of-view content | `rt/focus.rs`: stops (`Component::keys`), scopes and roving groups declared by a node's run and kept with the node (no transcript); Tab, F6, roving moves and requests walk the kept nodes; a trapping scope wraps Tab, the window root moves along the window's order and enters the topmost trapping layer child; the service moves ui-rt's key focus, and the ring is painted from `FocusVisible`, so a move runs nothing |
| Keys (`runtime/keys*`) | input hook on egui raw input, `KeyUse`, key stage | `rt/keys.rs`: ui-rt bubbles a key from the focused node; each node claims from what its run kept: the focused node's `KeyUse` and held Escape (queued as `NodeInput::Key`, which marks its path), a roving owner's moves, a trapping scope's Tab, a cancelling layer child's Escape, the window root's Tab and F6; an Escape nothing took dismisses the topmost layer child that asks (ui-rt). Intents from the queued keys: B6b |
| Commands (`runtime/commands`) | board, per-frame state, queue, gate | unchanged; command state read through tracked cells, so a node that read a state runs when it moves |
| IME engine (`runtime/ime.rs`, `primitives/text_input.rs`) | egui IME events plus fork patches | platform IME requests and events per window into the same engine; the type tool uses it (owner rule) |
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
