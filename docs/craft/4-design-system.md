# 4. Design system

The design system decides how apps look and how components compose. Its readers build components, themes and app looks.

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

## 4.2 Ownership

| Concern | Owner | Responsibility |
| --- | --- | --- |
| Token model, themes, resolution, validation | `craft-theme` | The design language as data, with no egui |
| Theme files, machine-local library, suite selection | `craft-theme-store` | `selection.json`, lock and revision protocol, watcher, client |
| Tokens to egui styles | `craft_ui::theme` | The one place data becomes egui styling, stock widgets included |
| Behaviour | `craft_ui::behavior`, `craft_ui::edit` | Focus, keys, typing, list navigation, the edit lifecycle |
| Generic components and shells | `craft-ui` | Reusable UI |
| Domain components | Family crates | Feature UI on the kernel |
| Gallery, lint, conformance | `craft-ui-gallery`, `craft-design-lint`, `craft-plate-test` | Inspect and enforce the system ([8](8-tests-and-gates.md)) |

## 4.3 Generic or family component

A component goes in `craft-ui` when both hold:

1. Its inputs are general data: values, ranges, points, text, a list of rows, or a small trait.
2. It names no document concept: layer, colour model, gradient, path, clip, page.

Otherwise it goes in the family crate that owns the concept. A component that two families need moves down to `craft-ui` once its inputs are general.

Borderline cases (curve editor, histogram, preset browser, menus, canvas overlays) and where the code places them are in [reference/component-tokens.md](reference/component-tokens.md#generic-or-family-placement).

## 4.4 Patterns and shells

A pattern is a reusable region of a window. It works in any shell and decides nothing about the rest of the window: panel, dialog, inspector, dock area, options bar, status bar, title bar, rails.

A shell composes a whole window. It orders slots (title bar, top bars, rails, body, bottom bars), decides which show, scopes ids and state per window, and draws no bar or panel itself. Slots take patterns.

| Shell | State |
| --- | --- |
| Canvas: fixed document canvas, document tabs, tool rail, panel dock. Photo, vector, design, figure, PDF and CAD apps | Built |
| Workspace: equal panels in named workspaces. Film, effects and sound apps | Planned |
| Office-style: ribbon in the top slot, backstage view, task panes | Planned |

Docking details are in [reference/component-tokens.md](reference/component-tokens.md#docking-details).

## 4.5 Per-app look and feel

An app has one layout in every theme and size. A theme never changes the layout. Different arrangements are workspaces, which are separate from themes. Studio, for example, is Pro's layout with Studio colours and the Comfortable size.

| The app decides | The framework shares |
| --- | --- |
| Layout and placement, following its industry original | Components and their behaviour |
| Conventions: menus, shortcuts, tool behaviour | Family patterns: dock and tool rail, ribbon |
| Default density: Compact or Comfortable | Focus and Tab order, Esc cancels, one undo step per drag |
| Default theme and mode: dark for creative apps, light for office-style apps | Themes, high contrast, translation, screen-reader support |
| Colour overrides for its own look | The token model that carries them |

## 4.6 Customization ladder

Use the first step that works.

1. **Density.** Compact or Comfortable.
2. **Size overrides.** The app ships a `sizes.json` that overrides base sizes (for example `size.control.md`). A base override carries to every size derived from it. To change one component, pin that component's token. Overrides stay within the validator's limits.
3. **Component tokens.** One named value changes a component app-wide.
4. **Builder options.** Options cover structural differences. A new need gets a new option, not a new component.
5. **Per-app colour overrides.** A theme file can carry a section per app. Values are theme roles or literal colours, so themes, dark mode and high contrast keep working.
6. **One-off app components.** A last resort, built from shared pieces. Allowed and not policed.

## 4.7 Themes

Ten built-in themes with 17 mode variants; the suite default is ArtCraft. A two-mode theme is one theme with a variant per mode, and a mode can carry a partial `seeds` override so each mode derives its own colours. Picking a two-variant theme sets both modes. A theme can name an icon pack that replaces the base icons it covers. The gallery is also the base of a theme editor. The themes are listed in [reference/component-tokens.md](reference/component-tokens.md#built-in-themes).
