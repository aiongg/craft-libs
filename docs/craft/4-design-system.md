# 4. Design system

The design system is the part of the framework that decides how apps look and how components compose. Its readers build components, themes and app looks. Crate tiers ([3.1](3-architecture.md#31-crate-tiers)) describe dependencies; the layers below describe composition. Six layers do not mean six crate tiers.

## 4.1 Component layers

Each layer reads only the layer below it.

```text
Shells       title bar, rails, body and bars as slots; presets per kind of app
  ^
Patterns     panel, dialog, inspector, dock area, options bar, status bar, title bar
  ^
Composites   field rows, property rows, headers, lists, tabs, toasts, menus
  ^
Controls     buttons, fields, sliders, checkboxes, selects, swatches, tooltips
  ^
Primitives   surface, state layer, focus ring, divider, stack, interactive area, text, icon, handle
  ^
Tokens       scales, ramps, seeds, contrast, size presets, component tokens, themes  (craft-theme, no egui)
```

The component index is in [6](6-components.md).

## 4.2 Ownership

| Concern | Owner | Responsibility |
| --- | --- | --- |
| Token model, themes, resolution, validation | `craft-theme` | The design language as data. No egui, so themes parse, validate and test without a UI toolkit |
| Theme files, machine-local library, suite selection | `craft-theme-store` | `selection.json`, lock and revision protocol, watcher, client |
| Tokens to egui styles | `craft_ui::theme` | The one place data becomes toolkit styling. Themes stock egui widgets too |
| Behaviour | `craft_ui::behavior`, `craft_ui::edit` | Focus, keys, typing and IME, list navigation, the edit lifecycle |
| Generic components and shells | `craft-ui` | Reusable UI infrastructure |
| Domain components | Family crates | Feature-specific UI on top of the kernel |
| Gallery, lint, conformance | `craft-ui-gallery`, `craft-design-lint`, `craft-plate-test` | Make the system inspectable and enforceable ([8](8-tests-and-gates.md)) |

## 4.3 Generic or family component

A component goes in `craft-ui` when both hold:

1. Its inputs are general data: values, ranges, points, text, a list of rows, or a small trait.
2. It names no document concept: layer, colour model, gradient, path, clip, page.

Otherwise it goes in the family crate that owns the concept. A component that two families need moves down to `craft-ui` once its inputs are general.

| Component | Placed in | Reason |
| --- | --- | --- |
| Button, Slider, List, Tab strip | `craft-ui` | General inputs |
| Menu | `craft-ui` (`menu`) | Takes rows and shortcuts; the app supplies the commands |
| Curve editor, histogram | `craft-ui` (composites) | Take points and bin counts; channel names are app words |
| Preset browser | `craft-ui` (`presets`) | Works on a generic preset library model |
| Canvas overlays: grid, guides, marquee, rulers, readout, reference point | `craft-ui` (`canvas`) | Draw over any canvas; no document type |
| Crop frame, gradient overlay, transform box, bezier paths | `craft-draw` | Need shape and gradient types |
| Colour picker, wheel, swatch palette | `craft-color` | Need colour models |
| Levels editor, warp grid | `craft-imaging` | Need image-adjustment types |
| Layers panel | `craft-layers` | Needs the `LayerTree` trait |

## 4.4 Patterns and shells

A pattern is a reusable region of a window. It works in any shell and decides nothing about the rest of the window: panel, dialog, inspector, dock area, options bar, status bar, title bar, rails.

A shell composes a whole window. It orders slots (title bar, top bars, rails, body, bottom bars), decides which show, scopes ids and state per window, and draws no bar or panel itself. Slots take patterns.

| Shell | State |
| --- | --- |
| Canvas: fixed document canvas, document tabs, tool rail, panel dock. Photo, vector, design, figure, PDF and CAD apps | Built |
| Workspace: equal panels in named workspaces. Film, effects and sound apps | Planned |
| Office-style: ribbon in the top slot, backstage view, task panes | Planned |

Panels live as tabs in an expanded dock column or as icons in a collapsed column, never both. A click on an icon opens its group in a flyout beside the column. Dialogs move by their title only. A floating panel group has square corners and the floating shadow.

## 4.5 Per-app look and feel

An app has one layout in every theme and size. A theme never changes the layout. Different arrangements are workspaces, which are separate from themes. Studio, for example, is Pro's layout with Studio colours and the Comfortable size.

Each app decides its layout and placement (following its industry original), conventions (menus, shortcuts, tool behaviour), default density (Compact for dense editors, Comfortable for roomier apps), default theme and mode (dark for creative apps, light for office-style apps), and colour overrides for its own look. The framework shares the components, family patterns (dock and tool rail, ribbon), behaviour (focus and Tab order, Esc cancels, one undo step per drag), themes, high contrast, translation and screen-reader support.

## 4.6 Customization ladder

Use the first step that works.

1. **Density.** Compact or Comfortable.
2. **Size overrides.** The app ships a `sizes.json` that overrides base sizes (for example `size.control.md`). A base override carries to every size derived from it. To change one component, pin that component's token. Overrides stay within the validator's limits.
3. **Component tokens.** One named value changes a component app-wide.
4. **Builder options.** Options cover structural differences. A new need gets a new option, not a new component.
5. **Per-app colour overrides.** A theme file can carry a section per app. Values are theme roles or literal colours, so themes, dark mode and high contrast keep working.
6. **One-off app components.** A last resort, built from shared pieces. Allowed and not policed.

Apps may set corner radii. A theme with its own shape (Classic, Breeze, Adwaita, Soft) wins over the app's radii.

## 4.7 Themes

Ten built-in themes with 17 mode variants. The suite default is ArtCraft.

- Two modes: ArtCraft (mid-grey panels), Pro (neutral greys), Graphite (near-black dark fields), Studio (PhotoCraft Studio colours).
- One mode: Pro Light Gray (light), Solarized Dark (dark), Classic (light).
- Own shape, two modes: Adwaita (rounded, radii 6 / 9 / 12), Breeze (flat, 5 px corners), Soft (low contrast, corners rounder by +1 / +2 / +2). Classic also has its own shape: square, 1 pt bevel, all radii 0.

A two-mode theme is one theme with a variant per mode. A mode can carry a partial `seeds` override, so each mode derives its own colours instead of freezing pins. Picking a two-variant theme sets both modes. A theme can name an icon pack that replaces the base icons it covers. The gallery is also the base of a theme editor.
