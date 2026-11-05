# 5. Foundations

Foundations are the values every component reads. They are the default group in every theme and size, and apps and components reference them by meaning. This section holds concepts only. Which component uses which token is in [reference/component-tokens.md](reference/component-tokens.md).

## 5.1 Colour

```text
seeds -> ramps -> semantic tokens -> pins -> contrast pass -> component tokens -> resolved Tokens -> theme bridge
```

- **Seeds.** Neutral, accent and four status hues (good, warn, bad, info).
- **Ramps.** 12 steps per seed per mode, in OKLCH, on a fixed lightness curve.
- **Semantic tokens.** Mapped to ramp steps by meaning (groups below).
- **Pins.** Any token can be set by hand.
- **Anchoring.** Unpinned background greys keep their distance from the theme's panel colour, so they follow a pinned panel. Text greys stay on the curve.
- **Contrast pass.** Derived colours move to the minimums in [5.7](#57-appearance-and-contrast).
- **Component tokens.** Derived from semantic tokens and pinnable. About 1,100 resolved tokens per variant.

| Group | Tokens |
| --- | --- |
| Surface | background, canvas, canvas pattern, chrome, panel, raised, sunken, knob, scrim, shadow |
| Text | normal, muted, faint, disabled, on accent, on strong, on selected, on status |
| Line | divider, border, control edge, knob edge, mark, focus |
| Accent | solid, hover, pressed, strong (+ hover, pressed), soft, text |
| Status | good, warn, bad, info, each with fg, bg, border, solid |
| State | hover, pressed, selected, selected hover, disabled fill |
| Canvas overlay | stroke, halo, handle, marquee, guide, grid |
| Data | 8 series; histogram R, G, B, luma |

Accent fills that carry text (menu highlight, primary buttons, solid badges) use the strong pair: the accent moved in lightness only as far as white text needs.

## 5.2 Typography

App text maps to the nearest named role. A real new need gets a new role. No role serves a single use.

| Role | Use | Compact (size / line, pt) | Comfortable |
| --- | --- | --- | --- |
| Caption | Small print, badges | 10.5 / 14 | 11.5 / 16 |
| Label | Control labels | 11.5 / 16 | 12.5 / 18 |
| Body | Body text | 12 / 16 | 13 / 18 |
| Body strong | Emphasised body text (semibold) | 12 / 16 | 13 / 18 |
| Title | Panel and dialog titles (semibold) | 13 / 18 | 14 / 20 |
| Heading | Section headings (semibold) | 15 / 20 | 17 / 24 |
| Body large | Large regular text, such as a prominent field | 15 / 20 | 16 / 22 |
| Mono | Code, numbers in tables | 11.5 / 16 | 12.5 / 18 |

The Text size setting scales the roles only (0.83x, 1x, 1.17x, 1.33x), rounded to 0.5 pt. Control heights, icons and faces do not scale. The interface font setting can use the OS UI face; mono stays the suite's.

## 5.3 Iconography

- **Sets.** Phosphor is the base set. The gallery's active set defaults to Lucide. A theme may supply an icon pack that replaces the base icons it covers; missing icons fall back to the base set.
- **Typed values.** Icons are generated at build time from the SVG folders.
- **Sizes.** Small, medium and large, from the size preset (12 / 16 / 20 pt Compact, 14 / 18 / 22 Comfortable).
- **Strokes.** 1.25 pt at the small size, 1.5 pt at medium and large.

## 5.4 Dimensions

Sizes are runtime tokens. The rule runs from the general to the specific:

1. A **scale** holds the allowed values: spacing 0, 2, 4, 6, 8, 12, 16, 20, 24, 32; radius 0, 2, 3, 4, 6, 8, 12 and full; strokes hairline, 1, 1.5, 2, 3.
2. **Semantic role tokens** pick from the scale by meaning: control height, icon size, gap, padding, radius, bar height.
3. **Component tokens** name a semantic token. A component never names a pixel value or a shape of its own.

Buttons, badges and slider tracks do not each carry their own "pill". Each takes the semantic radius role for its kind, and the size set and theme decide what the role resolves to. Components ship with this rule built in, so app code just uses the component. Only drawing details (stroke widths, chevron geometry, dash lengths) stay constants.

**Semantic role tokens (pt).**

| Role | Compact | Comfortable |
| --- | --- | --- |
| Control height sm / md | 20 / 24 | 24 / 30 |
| Icon sm / md / lg | 12 / 16 / 20 | 14 / 18 / 22 |
| Gap inline / row / stack / section | 4 / 4 / 6 / 12 | 6 / 6 / 8 / 16 |
| Padding control / panel / dialog | 6 / 8 / 16 | 8 / 12 / 20 |
| Radius control / popup / window / panel | 3 / 4 / 6 / 0 | 6 / 8 / 12 / 0 |
| Radius button / pill | full / full | full / full |
| Bar title / tab / status / options | 32 / 26 / 24 / 36 | 38 / 30 / 30 / 42 |
| Panel header | 28 | 34 |
| Rail width / button | 40 / 30 | 50 / 36 |

The pill radius is one token. It shapes switch and slider tracks and knobs, progress bars and badges, and clamps to half the part's height. Classic sets it to 0.

**Who sets sizes.** Themes never set sizes. The Size preset (Compact or Comfortable) sets them, and apps may override them within validator limits ([4.6](4-design-system.md#46-customization-ladder)).

**Theme shape.** Adwaita, Breeze, Soft and Classic define their own corner radii and edge style. These change how a component is drawn, not its geometry, placement or the workspace structure. Control heights, gaps and padding come from the Size preset in every theme.

**Edge.** Flat or bevel. Only the Surface primitive reads it.

## 5.5 Elevation

Shadow levels exist for popups, dialogs, floating panels and knobs. A bevel edge has none. Docked panels are square and flat. Floating panels add a shadow.

## 5.6 Motion

Durations are Fast (80 ms, hover and press feedback), Normal (150 ms, most state changes) and Slow (250 ms, large surfaces opening or moving). Easings are Standard, Enter and Exit. Reduced motion sets every duration to zero.

## 5.7 Appearance and contrast

### Settings

| Setting | Options | Changes | Stored |
| --- | --- | --- | --- |
| Theme | 10 built-ins or a JSON file, per mode (`theme_light`, `theme_dark`) | Colours; shape for four themes | Suite |
| Mode | light, dark, system | Palette | Suite |
| Contrast | standard, high, system (default) | Contrast minimums | Suite |
| Size | Compact, Comfortable | Heights, padding, gaps, type, corners | Suite |
| Text size | 0.83x, 1x, 1.17x, 1.33x | Type only | Suite |
| Reduced motion | full, reduced, system (default) | Animations to zero | Suite |
| Interface font | Suite face, System | UI face | App |
| Direction | Language (default), LTR, RTL | Mirroring; sliders, number fields and key chords never mirror | App |
| UI scaling | Auto, or 75 to 300 % (steps of 5 from 75 to 125, then 150, 175, 200, 250, 300) | Whole-interface zoom | App |

UI scaling omits sizes that would shrink the window below the app's minimum on the current screen (the size in use stays listed). Every app has a "Reset UI Scaling" command. Pointers an app draws grow with the scale.

### Precedence

Each field resolves on its own, in this order:

```text
app override  ->  suite choice (selection.json)  ->  app default
 (preferences)      (set once any app picks it)     (shipped by the app)
```

- **Theme, mode, size.** The app's override, if set, wins. Otherwise the suite's choice. Otherwise the app's default. An override replaces one field, not a whole profile: an app can override only its size and still follow the suite's theme and mode. An override of the theme replaces both slots of the theme pair; a slot may name the app default.
- **Contrast, reduced motion, text size.** Suite choice only, then the built-in default (system, system, 1x). The app has neither default nor override.
- **Interface font, direction, UI scaling.** App settings only.
- **System values.** `system` resolves against what the OS reports. Mode falls back to the app's fallback mode, contrast to standard and motion to full.
- **Unknown theme.** An unknown theme id falls back to the app default's slot, then to ArtCraft.

### Contrast

Standard contrast follows each theme's design. Derived colours reach the minimums. Pinned colours that fall short are kept and reported (`craft-theme check`, the gallery token browser, a "Low contrast" label in the theme picker), never rewritten. High contrast fixes every failing pair in the table, pinned or not.

| Class | Pairs | Standard | High |
| --- | --- | --- | --- |
| Text | Text normal, muted, faint on surfaces and state fills; text on selected fills; accent text; status foregrounds; text on accent, strong accent and status solids | 4.5:1 | 7:1 |
| Non-text cue | Focus line; accent solid, check and radio marks on control surfaces; overlay stroke on its halo; marquee on its dash | 3:1 | 4.5:1 |
| State cue | Accent border of selected controls; selected anchor outline on its fill; curve and channel colours on their plot | none | 4.5:1 |
| Control edge | Field and button borders, slider tracks, the active row of a popup list | none | 3:1 |
| No rule | Dividers, hover and pressed fills, disabled text and fills | none | none |
| Exempt | Guides, smart guides, path outlines and box lines: bare lines in a colour the user chooses | none | none |

"Every colour adjusted" in high contrast means every pair in the first four rows. The last two rows are outside the rule in both modes.
