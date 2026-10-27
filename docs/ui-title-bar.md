# Title bar and caption buttons

`craft_ui::title_bar` (feature `window-frame`) makes the app's own bar the window's title bar.
`craft_window::policy` decides who draws the frame; this module draws the app's part of it.

## What the bar does per frame

| Frame | Leading end | Trailing end | Empty area | Resize zones |
|---|---|---|---|---|
| `Native` | nothing | nothing | nothing (the OS bar moves the window) | none |
| `Inset(MacInset)` | the traffic lights' width | nothing | drags; double-click follows the macOS setting | none (AppKit) |
| `Integrated(Captions)` | the layout's left buttons | the layout's right buttons | drags; double-click maximizes | until a native hit test resizes |

- In full screen there is no inset, no button and no drag.
- The app draws the bar's background and fills `BarSlots::content` (menus, tabs, search).
  `BarSlots::centre_line` is the traffic lights' centre on macOS and the bar's middle elsewhere.
- Close never closes the window. The bar returns `close_requested`, and the app runs its own quit
  path (the unsaved-documents prompt). Minimize and maximize send their `ViewportCommand`s.
- Resize zones: 8 pt edges and 12 pt corners, never over a caption button or a control in the
  bar, on every undecorated window. Once the window procedure answers the hit test itself
  (Windows' `WM_NCHITTEST`), the app turns them off with `TitleBar::native_hit_test(true)` or a
  `CaptionRegions` sink whose `native_hit_test()` returns true. Both are false by default.

## Using it

```rust
let colors = CaptionColors::from_resolved(&resolved); // or the five fields from the app's own tokens
let labels = CaptionLabels::from_i18n(&i18n, lang);   // English where a translation is missing
let bar = TitleBar::new(&decision.frame, 32.0, colors).labels(&labels).show(ui, |ui, slots| {
    menus(ui);
});
if bar.close_requested {
    app.quit(ctx);
}
```

The app also:

- calls `title_bar::release_after_os_resize(ctx, raw_input)` from its `raw_input_hook` while the
  window is undecorated. KWin and other window managers keep the button release of an OS resize
  or move; without this, every other resize or move fails;
- on macOS, calls `craft_window::title_bar::sync_movable` once per frame.

## Caption buttons

`caption_buttons(ui, bar, &captions, &colors)` draws the buttons alone, for apps that lay out
their bar themselves. `CaptionButtons` adds labels and icon-theme glyphs.

| Family | Buttons | Close on hover / press |
|---|---|---|
| Windows | 46 pt wide, full bar height, flush with the window's edge | `#C42B1C` with a white glyph; 90 % while pressed |
| Adwaita | 24 pt discs, 10 % glyph colour at rest | the app's hover and pressed fills |
| Breeze | 18 pt bare discs; under the pointer the disc takes the glyph colour and the glyph the bar's | `#FF98A2` / `#6D2229`, glyph in the bar colour |
| Generic | Adwaita's buttons without the disc at rest | the app's hover and pressed fills |

- `CaptionColors::from_resolved` takes the suite theme's roles: glyph `text`, inactive glyph
  `text_muted`, fills `derived.hover` and `derived.pressed`, bar `surface`.
- Glyphs are painted. `CaptionIcons` swaps in the desktop icon theme's `window-*-symbolic`
  glyphs for the round families; the app supplies the renderer.
- Inactive windows paint the glyphs in `glyph_inactive`.
- Like the OS's own, the buttons are not Tab stops. Screen readers reach and press them through
  AccessKit.
- In a right-to-left UI the Windows captions move to the left, Close outermost. The Linux
  desktops' layouts are the user's settings and stay as set.
- Tooltips and accessibility labels are "Minimize", "Maximize", "Restore" and "Close"
  (`CaptionLabels`, keys `window.minimize`, `window.maximize`, `window.restore`, `window.close`).

## Snap layouts

`TitleBar::regions` takes a `CaptionRegions` sink that receives the bar's rect and each button's
rect every frame. It does nothing by default; it is the hook for the Windows snap-layout hit test
in the window procedure, and its `native_hit_test()` turns off the bar's resize zones.

## Tests

`src/title_bar/tests.rs` drives the bar headlessly (clicks, drags, resize zones, full screen,
frames). `src/title_bar/snapshot_tests.rs` (feature `screen-testing`) compares the renders in
`crates/ui/tests/snapshots/title-bar-*.png`; `UPDATE_SNAPSHOTS=1` rewrites them.
