# 6. Components

This section indexes the components by layer. Examples and every state of each component are in the gallery ([2.1](2-installation.md#21-run-the-component-gallery)). Which tokens a component reads is in [reference/component-tokens.md](reference/component-tokens.md).

## 6.1 What every component provides

- A builder API with semantic props only: no colour, pixel size or corner shape of its own. The idiom is `Type::new(required).option(..).show(ui)`. Panels take `&mut Model` in `new`. A dialog's per-pass answer is an `Outcome`; on the host it returns a `DialogResult<T>`.
- Gallery stories for every state.
- Everything in the checklist in [7.1](7-accessibility-and-translation.md#71-required-for-every-component).

## 6.2 Index by layer

| Layer | Components |
| --- | --- |
| Foundations | Colours, edit lifecycle, stock egui |
| Specimens | Icons |
| Primitives | Surface, State layer, Focus ring, Divider, Stack, Row, Interactive, Text, Icon, Handle. Canvas overlays: canvas, grid, guides, marquee, readout, reference point, rulers, smart guides |
| Controls | Button, IconButton, ToggleButton, SplitButton, Segmented, Link. Checkbox, Radio, Switch. TextField, SearchField, NumberField, Autocomplete, Linked and Mixed fields, PointPicker. Slider, RangeSlider, AngleDial. Select, ComboBox, TreePicker, Chips, Date. Swatch, Badge, Kbd, Spinner, Progress, Rating, Breadcrumb, Tooltip, ThumbRow |
| Composites | Field, FormGrid, GroupBox, PropertyRow, SliderRow, SectionHeader, SectionLabel, PanelHeader, PanelFooter. Menu, ContextMenu, PressMenu, MenuBar. List, ListRow, TreeRow, Table. Tab, TabStrip. Toast, Notice, NoticeCentre, EmptyState. CurveEditor, CurvePreview, Histogram. PresetBrowser |
| Patterns | Panel, Popover, Dialog, FullWindowDialog, PreferencesWindow, CommandPalette, Inspector, DockArea, ViewportWindow, PanelRail, ToolRail, Toolbar, ToolbarDrawer, OptionsBar, OptionsCell, StatusBar, StatusReadout, TitleBar, FlyoutButton, AlignBar, Tour |
| Shells | AppShell with the Canvas preset (`CanvasShell`), start screen |

The gallery lists every component.

## 6.3 Family components

| Crate | Components |
| --- | --- |
| `craft-color` | ColorPicker, ColorArea, ColorWheel, ColorDisc, ColorRamp, ColorPopover, HexField, SwatchPair, SwatchPalette |
| `craft-draw` | GradientSwatch, GradientEditor, GradientPalette, gradient line, TransformFields, transform box, bezier paths, crop frame, perspective crop quad, pen draft |
| `craft-imaging` | LevelsEditor, warp grid |
| `craft-layers` | Layers panel, rows, filter bar, thumbnails |
| `craft-browse` | Tile grid, media bin, places, thumbnails, relink |

Other families ([3.3](3-architecture.md#33-crate-taxonomy)) add theirs as they land.
