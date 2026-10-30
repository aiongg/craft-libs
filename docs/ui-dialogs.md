# Dialogs

`craft_ui::dialog` draws modal and modeless dialogs and the behaviour a host would otherwise
repeat: placement memory, button order, Enter and Esc, nesting, and canvas input under a modal
dialog. The host keeps its fields and calls `show` every pass; the dialog keeps chrome state only
(`DialogState`). Sizes are the `dialog.*` tokens; colours are the popup family's.

## Choosing a type

| Type | Use |
|---|---|
| `Dialog` | A command dialog or tool window with a title, body and footer. |
| `Prompt` | A modal question with answers and accelerator keys. Keeps its own state. |
| `Alert` | A modal notice with one OK button. Keeps its own state. |
| `FullWindowDialog` | A workspace that fills the window or a given rect (Liquify, Filter Gallery). |
| `BlockingOverlay` | Blocks input while work runs; no chrome. |
| `DialogFrame` | The look alone, for a host that draws its own behaviour. |

## Dialog

```rust
use craft_ui::dialog::{Dialog, DialogState, Footer, Outcome};

let shown = Dialog::new("new-document", "New Document")
    .footer(Footer::standard().ok("Create").cancel("Cancel"))
    .show(ctx, &mut state, &mut fields, |ui, fields| { /* controls */ });
match shown.outcome {
    Some(Outcome::Ok) => { /* commit, then close */ }
    Some(Outcome::Cancel) => { /* revert */ }
    _ => {}
}
```

- **Outcome.** `Ok`, `Cancel`, `Alternate` (`DialogResponse::alternate` says which of the two); `Outcome::Alternate` has no payload,
  `Apply` and `Reset` (the dialog stays open), `Close` (the close box of a modeless dialog) and
  `Answer(role)` (a dialog built for a question). `Outcome::closes` tells whether the host takes
  the dialog away.
- **Placement.** The first open is centred or at `first_place`, else at the offset the user
  dragged it to, remembered under the `PlaceKey` in egui's persisted memory. After its first
  laid-out pass the top-left stays put, so a body that grows extends down and right. The stored
  offset is kept on screen by `keep_on_screen` and is never rewritten by it.
- **Moving.** The title band is the only handle: dialogs move by dragging it. There is no
  keyboard move, and the band is not a Tab stop.
- **Size.** `Width` classes cap the content width at the window less margins. `resizable` adds a
  grip, fills the area it gives and remembers the size the user left (`remember_size`).
  `Body::Scroll` makes the body a scroll area that leaves room for the frame's other parts and for the region `Dialog::show_with_below` draws under it (measured, not reserved).
- **Modal.** Uses egui's modal layer; `Backdrop::None` draws nothing, `Backdrop::Scrim` draws
  `surface.scrim`. A click outside does nothing. The dialog node is AccessKit `Dialog` with
  `modal` set.
- **Modeless.** Blocks nothing, raises when pressed, answers `Close` from the close box. It takes
  Enter and Esc only while it holds the keyboard focus, and a footerless tool window ignores Esc.
- **Initial focus.** `InitialFocus::Auto` focuses the first enabled field (or the default button
  when there is none) once, on the first laid-out pass; `FirstField`, `DefaultButton` and `None`
  override it. `DialogResponse::initial_focus` reports the pass.
- **Edits.** `outer_gesture(true)` runs the body in `edit::scope` under one outer gesture, which
  the host commits on Ok and cancels on Cancel. `history(true)` keeps a `LocalHistory` of the
  fields for Ctrl/Cmd+Z and Shift+Ctrl/Cmd+Z while the dialog takes the keys and no text field is
  editing. `Preview` is the Preview check box with hold-to-compare and a Before toggle.

## Footer

| Shape | Layout |
|---|---|
| `Footer::standard()` | Role buttons at the trailing edge, with `leading`, `preview` and `trailing_extra` controls. |
| `Footer::side()` | A column at the body's trailing side: OK above Cancel, `extra` buttons under them, all as wide as the longest label. The same order on every platform. |
| `Footer::custom(draw)` | The host draws; `FooterFx::buttons` places role buttons. |
| `Footer::none()` | No row. |

Button roles (`ButtonRole`) decide the place and look:

| Role | Does | Outcome |
|---|---|---|
| `Default` | Primary button (OK, Save, Yes); the AccessKit description says it is the default. | `Ok` |
| `Alternate` | Another answer that closes (Don't Save). Two at most. | `Alternate` |
| `Cancel` | Closes without acting. | `Cancel` (modal), `Close` (modeless) |
| `Apply` | Acts and keeps the dialog open. | `Apply` |

Order follows `Platform`: Windows and Linux put the default action first (OK Cancel Apply); macOS
puts it last in the corner with Cancel beside it (Don't Save, Cancel, Save). `Reset::AltCancel`
makes Cancel show the Reset label while Alt (Option) is held; a click or Space then answers
`Reset` and Esc stays Cancel. `ok_enabled(false)` takes no click and no Enter.

Keys (`resolve_keys`, a pure function over `KeyInput`):

- Esc cancels a modal dialog that is the top modal, also while a text field is edited. Esc wins
  over Enter pressed in the same pass.
- Enter answers `Ok` when the default button is enabled, no footer button has the focus (egui
  presses that one) and the body does not own Enter (`EnterRule::Body`: a multi-line field, a
  list). A key repeat never counts. A field that already took Enter leaves none.
- With a popup or craft-ui list open, both keys belong to it.
- Tab and Shift+Tab visit fields only (`FieldTab`); Ctrl+Tab reaches every control. Arrow keys
  move between footer buttons; Space and Enter press the focused one. A disabled button takes no
  focus.

## Prompt and Alert

`Prompt::new(id, title, message)` takes `answer(role, label, key)` for each button. Show it every
pass until it returns `Some(role)`. It is centred, 420 points wide; the Default answer takes the
focus. Esc and the close box answer the `escape` role: Cancel when there is one, else the first
Alternate. Answer keys are plain keys, read only while the prompt is the top modal and no popup
is open. `accelerator_labels` draws them on Windows and Linux in one style per prompt (`(Y)es`
when every label starts with its key, else `Ja (Y)`); macOS shows plain labels and the keys still
work. They are also the buttons' AccessKit keyboard shortcuts.

`Alert::new(id, title, message)` shows one OK button (`ok_label`) and returns true on the pass it
closes: OK, Esc, the close box and Enter all close it. An Enter that raised the alert, or any
key repeat, does not. Its AccessKit role is `AlertDialog`, described by the message.

## FullWindowDialog

`FullWindowDialog::new(id)` fills the window's content rect, or `rect`, with `surface.panel`.
`title` adds a strip `size.panel_header` tall with `line.divider` under it, and `title_actions`
puts controls at its trailing end. The body places its own footer with `FullWindowFx::buttons(ui,
rect, buttons)`. It is modal, has no close box, no placement or size memory and no drag, and
Apply keeps it open. Esc and Enter follow `resolve_keys`; `enter(EnterRule::Body)` leaves Enter
to the workspace. `initial_focus` acts only on `DefaultButton`; the default is to leave the
keyboard on the view. `outer_gesture` works as on `Dialog`.

## DialogStack

The host keeps a `DialogStack` next to its dialog states and passes it to `Dialog::stack`,
`Prompt::stack` and `Alert::stack`.

- `push(id, parent)` opens a dialog on top; pushing an id already present changes nothing, so a
  host may push every pass. `dialog_id(key)` gives the id before the first `show`.
- Only the top dialog is interactive. Older dialogs are disabled whole, and a dialog that becomes
  the top comes to the front.
- A nested dialog is pushed with its `parent` and shown from the parent's body. While a parent
  has a child, its OK is out of reach, and Enter and Esc go to the child.
- `remove(id)` removes one entry and leaves its children; `remove_tree(id)` removes the entry and
  the dialogs opened from it. The host removes an entry on its closing outcome.
- A dialog that was never pushed is not blocked.

## Canvas input under a dialog

A modal dialog leaves the rest of the window inert, but an image can still be panned and zoomed
under it. The `pass` helpers read raw input outside the rects of the dialogs shown in the last
frame in which dialogs were shown (this frame or the one before). The canvas draws before the
dialogs of a pass, so the rects are one frame old. While any popup is open the helpers report
nothing.

| Helper | Returns |
|---|---|
| `pass::any_modal(ctx)` | Whether a modal dialog, full-window dialog or blocking overlay is open; hosts hold menus and shortcuts while it is. Also `dialog::any_modal`. |
| `pass::free_pointer_over(ctx, canvas)` | The pointer when it is over `canvas` and not over a dialog. |
| `pass::pan_delta(ctx, canvas, hand)` | This pass's movement of a Space-drag, middle-drag or (with `hand`) plain drag that began on the free canvas. |
| `pass::free_buttons(ctx, canvas)` | `Buttons` (`started`, `dragged`, `stopped`, `clicked`): the primary button on the free canvas, for a held Zoom. |
| `pass::free_press(ctx, canvas)` | Where the pointer is for a primary click or drag that began on the free canvas with Space not held. |

A press that began on a dialog never counts, even after the dialog follows the pointer while its
title is dragged. `BlockingOverlay` is reported by `any_modal` and nothing passes through under
it.

## Viewport windows

`craft_ui::viewport::ViewportWindow` shows a window in its own OS viewport, or in a resizable
`Dialog` (`Footer::none()`, place key the viewport id) where `Context::embed_viewports` is set or
`embedded(true)` forces it. The body gets a `Hosting` value for either case. `app_modal(scrim)`
makes the main window ignore input while the window is open: a native window puts a layer over
the parent's content rect that shows the not-allowed cursor and focuses the child on a press;
an embedded window is a modal dialog with `Backdrop::Scrim` when `scrim` is set. Hosts hold
shortcuts while `ViewportWindow::blocks_main(ctx)` is true. `close_requested` reports the OS
close button, Alt+F4, the embedded close box, or Esc in an app-modal window with no popup open.
