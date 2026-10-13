# craft-menu

A Craft app's menus as a macOS menu bar. The app supplies its menu tree and runs its commands;
this crate lays the menus out the Mac way, drives a platform `Backend`, and turns the menu's key
equivalents back into egui key presses. It depends on egui and
[`craft-quit`](../quit/src/lib.rs), and on no app. Everything but the macOS backend
(`craft_menu::mac`, built on muda) is platform-free and also builds for `wasm32`. The layout and driver come from PhotoCraft's `native_menu.rs`.

## The menus

Build a `MenuBar` directly, or from the flat rows an in-window bar draws (`from_items`, with
`Row { path, id, label, shortcut, enabled, checked }`, `---` for a separator). Then tag what
macOS puts in fixed places:

```rust
let mut bar = craft_menu::from_items(&["File", "Edit", "View", "Window", "Help"], &rows);
bar.tag_item("help.about", ItemRole::About);
bar.tag_item("file.exit", ItemRole::Quit); // the app's quit, with its unsaved-changes prompt
bar.tag_submenu("Edit", "Preferences", ItemRole::Settings);
bar.tag_submenu("View", "Theme", ItemRole::Appearance);
bar.tag_menu("File", MenuRole::File);
bar.tag_menu("Window", MenuRole::Window);
bar.tag_menu("Help", MenuRole::Help);
```

Items built directly use `Item::new(id, label)` with `.shortcut("Cmd+Shift+Z")`, `.enabled(..)`,
`.checked(..)` and `.role(..)`. Shortcuts are written `Cmd+Shift+Z`; `Ctrl` is the Control key,
not ⌘. Open documents in the Window menu are items with `ItemRole::Document`.

`mac_layout(&bar, &LayoutOptions::new("PhotoCraft"))` returns the Mac menu bar and the clashes
it resolved:

- **App menu:** About, Settings (⌘,), Language, Appearance, Services, Hide (⌘H), Hide Others,
  Show All, Quit (⌘Q). The tagged items leave their old menus. Without a tagged Quit item the
  layout adds one with `craft_quit::QUIT_MENU_ID`.
- **Window:** Minimize, Zoom, the app's own Window items, Bring All to Front, then the open
  documents. A Window menu is added if the app has none. **File** gets Close Window (⌘W).
- **Clashes:** when the app binds ⌘H, ⌘M, ⌘W, ⌘, or ⌘Q to a command, the command keeps it.
  Hide and Minimize move to ⌃⌘H and ⌃⌘M, the others go without a shortcut, and each case is
  reported as a `Clash`.

The labels the layout writes are English. To translate them, pass
`LayoutOptions { app_name, label }`, where `label` maps an English label such as `"Quit {app}"`
to the app's translation; the crate substitutes `{app}`.

## macOS

Install the menu bar once, on the main thread, in eframe's creator closure (so winit's default
menu doesn't stay up). `None` means AppKit refused the menus: keep the in-window ones.

```rust
#[cfg(target_os = "macos")]
let native_menu = craft_menu::mac::install(&cc.egui_ctx, &craft_menu::mac_layout(&menu_bar, &LayoutOptions::new("PhotoCraft")).bar);
```

The backend owns muda's menu event handler, of which there is one per process, so an app using
craft-menu does not call `craft_quit::install_app_menu`; craft-menu's own Quit item already asks
the window to close. Each menu event wakes egui, so a chosen item runs at once rather than with
the next input event.

While a menu is open, a click or Escape in the window closes it and does not reach egui (no
canvas gesture, no tool action); pointer moves pass through. Menu updates wait until the menu
closes.

## Per frame

Key equivalents stay with egui: the backend reports them as `Event::Key`, and `raw_input` feeds
each one to egui as the key press it was, so the app's own key handling (text fields, dialogs,
tool-specific undo) runs as without a native menu.

Each frame: `raw_input` before egui reads the input, then the ids from `take_clicks`, then
`sync_with`.

```rust
// eframe::App::raw_input_hook, before egui reads the input:
if let Some(menu) = &mut self.native_menu {
    menu.raw_input(raw_input);
}

// In the frame:
if let Some(menu) = &mut self.native_menu {
    for id in menu.take_clicks(ctx) {
        self.run_command(ctx, &id); // Minimize and craft-quit's Quit are already handled
    }
    let state = self.menu_state_hash(craft_menu::had_input(ctx).then_some(ctx.cumulative_frame_nr()));
    for c in menu.sync_with(state, || craft_menu::mac_layout(&self.menu_bar(), &LayoutOptions::new("PhotoCraft"))) {
        log::info!("menu: {} is {} on macOS and {} in the app; {}", c.shortcut, c.system, c.command, c.resolution);
    }
}
```

`sync_with` builds the layout only when `state` changed or a menu event arrived, because
building the rows usually walks every command's enabled state. The backend then updates labels,
enabled and checked in place and rebuilds only when `structure_key` (or, for the open
documents, `window_key`) changes.
