# 6. Components

This section indexes the components by layer. Examples and every state of each component are in the gallery ([2.1](2-installation.md#21-run-the-component-gallery)). Which tokens a component reads is in [reference/component-tokens.md](reference/component-tokens.md).

## 6.1 What every component provides

- A builder API with semantic props only: no colour, pixel size or corner shape of its own.
- Gallery stories for every state.
- Keyboard behaviour, a keyboard-only focus ring, AccessKit roles, RTL and reduced-motion support, and translated library words.
- Edit events for value changes ([3.4](3-architecture.md#34-the-host-convention-services-in-edits-out)).

The checklist that makes a component complete is in [7.1](7-accessibility-and-translation.md#71-required-for-every-component).

## 6.2 Index by layer

| Layer | Components |
| --- | --- |
| Foundations | Colours, edit lifecycle, stock egui |
| Specimens | Icons |
| Primitives | Surface, State layer, Focus ring, Divider, Stack, Row, Interactive, Text, Icon, Handle. Canvas overlays: canvas, grid, guides, marquee, readout, reference point, rulers, smart guides |
| Controls | Button, IconButton, ToggleButton, Segmented, Link. Checkbox, Radio, Switch. TextField, SearchField, NumberField. Slider, RangeSlider, AngleDial. Select, ComboBox. Swatch, Badge, Kbd, Spinner, Progress, Tooltip, ThumbRow |
| Composites | Field, FormGrid, GroupBox, PropertyRow, SliderRow, SectionHeader, SectionLabel, PanelHeader, PanelFooter. Menu, ContextMenu, PressMenu, MenuBar. List, ListRow, TreeRow. Tab, TabStrip. Toast, Notice, EmptyState. CurveEditor, CurvePreview, Histogram. PresetBrowser |
| Patterns | Panel, Popover, Dialog, FullWindowDialog, PreferencesWindow, CommandPalette, Inspector, DockArea, ViewportWindow, PanelRail, ToolRail, Toolbar, OptionsBar, StatusBar, TitleBar, FlyoutButton |
| Shells | AppShell with the Canvas preset (`CanvasShell`) |

Components added by later migration steps (tables, chips, date pickers, notice centre and others) are listed in the gallery.

## 6.3 Family components

| Crate | Components |
| --- | --- |
| `craft-color` | ColorPicker, ColorArea, ColorWheel, ColorDisc, ColorRamp, ColorPopover, HexField, SwatchPair, SwatchPalette |
| `craft-draw` | GradientSwatch, GradientEditor, GradientPalette, gradient line, TransformFields, transform box, bezier paths, crop frame, perspective crop quad, pen draft |
| `craft-imaging` | LevelsEditor, warp grid |
| `craft-layers` | Layers panel, rows, filter bar, thumbnails |
| `craft-browse` | Tile grid, media bin, places, thumbnails, relink |

Other families ([3.3](3-architecture.md#33-crate-taxonomy)) add theirs as they land.
