# craft-menu

A Craft app's menus, laid out for macOS, Windows, Linux and the web. The app supplies its menu
tree and runs its commands; this crate lays the menus out the way each platform's apps do, takes
shortcuts from craft-keymap, drives a platform `Backend`, and turns the menu's key
equivalents back into the input egui would have had without it. It depends on egui,
[`craft-keymap`](../keymap/src/lib.rs) (commands and chords) and [`craft-quit`](../quit/src/lib.rs),
and on no app. Everything but the macOS backend
(`craft_menu::mac`, built on muda) is platform-free and also builds for `wasm32`. The layout and driver come from PhotoCraft's `native_menu.rs`, the egui bar from its `menu_nav.rs` and in-window menu bar.

## The menus

Build a `MenuBar` directly, or from the flat rows an in-window bar draws (`from_items`, with
`Row { path, command, label, shortcut, enabled, kind }`, `---` for a separator). Give each menu
its role (`MenuRole::{App, File, Edit, View, Window, Help, Other}`), then tag what macOS puts in
fixed places:

```rust
let mut bar = craft_menu::from_items(&["File", "Edit", "View", "Window", "Help"], &rows);
bar.tag_menu("File", MenuRole::File);
bar.tag_menu("Edit", MenuRole::Edit);
bar.tag_menu("View", MenuRole::View);
bar.tag_menu("Window", MenuRole::Window);
bar.tag_menu("Help", MenuRole::Help);
bar.tag_item(&"help.about".into(), ItemRole::About);
bar.tag_item(&"file.exit".into(), ItemRole::Quit); // the app's quit, with its unsaved-changes prompt
bar.tag_submenu(MenuRole::Edit, "Preferences", ItemRole::Settings);
bar.tag_submenu(MenuRole::View, "Theme", ItemRole::Appearance);
```

Each item runs a `craft_keymap::CommandRef`, and a click reports it. A command that takes a
parameter carries it in the `CommandRef`:
`CommandRef::with_params("file.open_recent", json!({"index": 3}))`. Items built directly use
`Item::new(command, label)` with `.shortcut(chord)`, `.enabled(..)`, `.check(..)` or
`.radio(..)` (the item's `ItemKind`), `.role(..)` and `.platforms(..)`; submenus use
`Node::submenu(label, children)`, with `.role(..)` for a tagged one. A shortcut is a
`craft_keymap::Chord`: `cmd` is ⌘ and `ctrl` the Control key. macOS shows radio items as check
items. The app's open documents go in `MenuBar::documents` (`Document::new(command, name)`,
`.active(..)` for the one in front, `hidden` for one kept open but not listed), and the layouts
list them in the Window menu. The items the
layout adds itself run a `Builtin` command (Hide, Hide Others, Minimize, Quit), which the crate
handles.

## Layouts

`layout(&bar, target, &LayoutOptions::new("PhotoCraft"))` lays the menus out for a `Target`
(`Mac`, `Windows`, `Linux`, `Web`; `Target::current()` for this build) and returns the bar and
the clashes it resolved. Every layout first drops the items and submenus whose `.platforms(..)`
leave the target out, and lists the open documents (but the hidden ones) last in the Window menu
as radio items, the one in front selected. A submenu left empty goes; for an empty dynamic list
(no recent files) the app supplies a disabled placeholder item, which keeps the submenu.

| Target | About | Settings | Quit | Also |
|---|---|---|---|---|
| `Mac` | app menu | app menu, Settings… (⌘,) | app menu, ⌘Q | Services, Hide, Minimize, Zoom, Close Window |
| `Windows` | last in Help | last in Edit, Preferences… | last in File, Exit | |
| `Linux` | last in Help | last in Edit, Preferences… | last in File, Quit | |
| `Web` | last in Help | last in Edit, Preferences… | none | browser shortcuts removed |

The layouts find these items by role, never by title, and add a File, Edit, Window or Help menu
when the app has none to put them in. Language and Appearance stay where the app put them off
macOS, and the Windows, Linux and web layouts keep the app's shortcuts. The browser keeps Ctrl or
⌘ with W, N, T and Q, and with Shift+N, Shift+T and Shift+W, for its tabs and windows, so the
web layout takes those shortcuts off the items and reports each as a `Clash`.

`mac_layout(&bar, &LayoutOptions::new("PhotoCraft"))` returns the Mac menu bar and the clashes
it resolved:

- **App menu:** About, Settings (⌘,), Language, Appearance, Services, Hide (⌘H), Hide Others
  (⌥⌘H), Show All, Quit (⌘Q). The tagged items leave their old menus. Without a tagged Quit item
  the layout adds one with `craft_quit::QUIT_MENU_ID`. A menu the app tagged `MenuRole::App` is
  merged in: its other items follow Appearance.
- **Window:** Minimize, Zoom, the app's own Window items, Bring All to Front, then the open
  documents. A Window menu is added if the app has none. **File** gets Close Window (⌘W).
- **Clashes:** when the app binds ⌘H, ⌥⌘H, ⌘M, ⌘W, ⌘, or ⌘Q to a command, the command keeps
  it. Hide and Minimize move to ⌃⌘H and ⌃⌘M, the others go without a shortcut, and each case is
  reported as a `Clash`. Shortcuts bound outside the menus (tool and canvas keys) count when the
  app lists them: `LayoutOptions::new("PhotoCraft").extra_shortcuts(&[(chord, command), ..])`.

The labels the layouts write (`About {app}`, `Preferences…`, `Exit`, `Window`, `Minimize`, …)
are English. To translate them, pass `LayoutOptions { label, ..LayoutOptions::new(app_name) }`,
where `label` maps an English label such as `"Quit {app}"` to the app's translation; the crate
substitutes `{app}`.

## Keymap

`apply_keymap(&mut bar, &keymap)` gives each item the shortcut its command has in the app's
`craft_keymap::Keymap`, the user's overrides included: the first single-press binding, one in
the command's home scope first. A command the keymap knows but binds to nothing loses its
shortcut; an item whose command the keymap doesn't know keeps its own. Items of toggle commands
(`Bindable::toggle`) become check items, keeping their checked state; radio items stay radio
items.

Each frame, `apply_state(&mut bar, |command| ItemState { enabled, checked })` sets enabled and
checked from the app's state. `checked: Some(..)` makes a plain item a check item and selects or
clears a radio item; `None` leaves the kind as it is. The built-in items and the open documents
are left alone. Both run on the app's bar, before the layout.

## Windows, Linux and web: the egui bar

`MenuBarUi` draws a laid-out bar with egui, in a top panel, and returns the commands chosen that
frame:

```rust
let laid_out = craft_menu::layout(&menu_bar, Target::current(), &LayoutOptions::new("PhotoCraft")).bar;
egui::Panel::top("menu-bar").show(ui, |ui| {
    let gate = Gate { modal: dialog_open, recording_shortcut, text_editing: ui.ctx().text_edit_focused() };
    for command in MenuBarUi::new(&laid_out).gate(gate).label(&tr).show(ui) {
        if !craft_quit::on_menu_event(ui.ctx(), &command.id) {
            app.run(&command);
        }
    }
});
```

- **Mouse:** a press on a title opens its menu and releasing over an item runs it; a click on
  the title again, Esc or a click outside closes it. While a menu is open, moving onto another
  title opens that one. Submenus open on hover and stay open while the pointer heads for them
  across other rows. A menu taller than the window scrolls, with the wheel or by resting on the
  ▲ and ▼ arrows.
- **Keys, while a menu is open:** ↑ and ↓ move the highlight, → opens a submenu or the next
  menu, ← closes the submenu or opens the previous menu, ↩ and Space run the item, Esc closes.
  `craft_menu::bar::is_open(ctx)` tells the app's shortcut handling to wait meanwhile.
- **Items:** disabled items, and the items the `Gate` forbids (`Gate::allows`), are greyed and
  never run. Check items show ✓, the selected radio item a dot, and the shortcut column shows
  every shortcut, the display-only ones too, in the target's names (⌘ glyphs on `Target::Mac`,
  Ctrl names elsewhere, the web included; `.target(..)` to choose).
- **Narrow windows:** the menus that don't fit move into a trailing `⋯` menu; when not even the
  first fits, one `Menu` title holds them all. `.label(&tr)` translates `⋯`, `Menu` and the
  scroll arrows' names, with the same hook as `LayoutOptions::label`.
- **Quit:** without an app Quit item, the Windows and Linux layouts add `Builtin::Quit`
  (`craft_quit::QUIT_MENU_ID`). The bar returns it like any other command; pass it to
  `craft_quit::on_menu_event`, which asks the window to close so the unsaved-changes check runs.
- **Accessibility:** AccessKit sees a menu bar of menu items, the open menus, check and radio
  items with their state, disabled items, and each item's shortcut.
- **Style:** `BarStyle::from_style(ui.style())` by default; `.style(..)` sets title padding and
  color, the menu width, separator room and the check and radio marks.

## macOS

Install the menu bar once, on the main thread, in eframe's creator closure (so winit's default
menu doesn't stay up). `None` means AppKit refused the menus: keep the in-window ones.

```rust
#[cfg(target_os = "macos")]
let native_menu = craft_menu::mac::install(&cc.egui_ctx, &craft_menu::mac_layout(&menu_bar, &LayoutOptions::new("PhotoCraft")).bar);
```

The backend owns muda's menu event handler. There is one per process and the first one set
stays, so `mac::install` and `craft_quit::install_app_menu` are alternatives: an app calls one of
them, once, and a second `mac::install` returns `None`. craft-menu's own Quit item already asks
the window to close. Each menu event wakes egui, so a chosen item runs at once rather than with
the next input event.

While a menu is open, a click or Escape in the window closes it and does not reach egui (no
canvas gesture, no tool action); pointer moves pass through. Only one click is kept this way
until the menu reports closed, so a menu wrongly reported open costs one click. Menu updates wait
until the menu closes. A rebuild that fails keeps the menus as they were.

## Per frame

Key equivalents stay with egui: the backend reports them as `Event::Key`, and `raw_input` feeds
each one to egui as egui-winit would have, so the app's own key handling (text fields, dialogs,
tool-specific undo) runs as without a native menu. ⌘C, ⌘X and ⌘V become egui's Copy, Cut and
Paste (`ClipboardKeys::Egui`, the default), so text fields copy and paste; an app that runs its
clipboard commands from the keys itself sets `menu.clipboard_keys(ClipboardKeys::Raw)` to get
key presses. A shortcut the layout assigned (⌘Q on the app's Quit item, ⌘, on Settings) comes
back as a click instead, since the app's key handling doesn't bind it.

While a text field has the keys, an app's key handling leaves shortcuts to the field, but a Mac
app's menu commands (⌘S, ⌘N) run while typing. So the app reports each frame whether text has
the keys (`set_text_editing`), and while it does, a key equivalent comes back as a click, except
the keys the field handles itself: ⌘C, ⌘X and ⌘V (as `ClipboardKeys` says) and
`craft_keymap::text_editing_chords` (⌘A, ⌘Z, ⇧⌘Z, ⌘←, …) stay key presses.

Each frame: `raw_input` before egui reads the input, then the commands from `take_clicks`, then
`sync_with`, then `set_text_editing` for the next frame's `raw_input`.

```rust
// eframe::App::raw_input_hook, before egui reads the input:
if let Some(menu) = &mut self.native_menu {
    menu.raw_input(raw_input);
}

// In the frame:
if let Some(menu) = &mut self.native_menu {
    for command in menu.take_clicks(ctx) {
        self.run_command(ctx, &command); // Minimize and Quit are already handled
    }
    let state = self.menu_state_hash(craft_menu::had_input(ctx).then_some(ctx.cumulative_frame_nr()));
    for c in menu.sync_with(state, || craft_menu::mac_layout(&self.menu_bar(), &LayoutOptions::new("PhotoCraft"))) {
        log::info!("menu: {} is {} on macOS and {} in the app; {}", c.shortcut, c.system, c.command, c.resolution);
    }
    menu.set_text_editing(ctx.wants_keyboard_input());
}
```

`sync_with` builds the layout only when `state` changed or a menu event arrived, because
building the rows usually walks every command's enabled state. The backend then updates labels,
enabled and checked in place and rebuilds only when `structure_key` (or, for the open
documents, `window_key`) changes.
