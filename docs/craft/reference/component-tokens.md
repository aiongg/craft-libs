# Component tokens

Maintainer reference. The rule is in [5.4 Dimensions](../5-foundations.md#54-dimensions): components name semantic tokens, and semantic tokens pick from the scale. This page lists which role each component takes, and which components still hold raw values.

## Radius roles

| Role | Components |
| --- | --- |
| `radius.control` | Fields, small controls, buttons that are not pills |
| `radius.popup` | Popups, menus |
| `radius.window` | Windows, dialogs |
| `radius.panel` | Docked panels (0 in every built-in size set) |
| `radius.button` | Push buttons. `full` in the built-in size sets |
| `radius.pill` | Switch and slider tracks and knobs, progress bars, badges. Clamps to half the part's height |

A theme with its own shape replaces the radii or adds an offset to control, popup and window. Panel, button and pill are not offset.

## Size roles

| Role | Components |
| --- | --- |
| `size.control.sm`, `size.control.md` | Controls, fields, buttons |
| `size.icon.sm`, `size.icon.md`, `size.icon.lg` | Icons |
| `size.bar.title`, `size.bar.tab`, `size.bar.status`, `size.bar.options`, `size.bar.options_narrow` | Title bar, document tab bar, status bar, options bar |
| `size.panel_header` | Panel headers |
| `size.rail.width`, `size.rail.button`, `size.rail.panel_width`, `size.rail.panel_button` | Tool rail and panel rail |
| `size.list.*`, `size.tab.*`, `size.preset.*` | Lists, tab strips, preset browsers |
| `space.gap.*`, `space.pad.*` | Gaps between and padding inside components |

Component values that differ by size set are in the `components` section of `sizes/compact.json` and `sizes/comfortable.json`: `options_bar.button`, `progress.width`, `button.height.sm`, `button.height.md`, `button.pad_x`, `dock.body_gap`, `status_bar.field_width`, `command_palette.row_height`, `inspector.header_height`, `inspector.icon`, `toolbar.*`, `toast.pad`, `toast.inset`, `channels.thumb`, `tile.*`. NumberField defaults to a width of three control heights. Apps set other widths per field.

## Raw values to convert

`crates/theme/src/values.rs` holds 943 component constants that are raw values: sizes, widths, gaps, paddings and radii written as pixel values, plus some fixed colours and opacities; many set for the Compact size and scaled by control height. Each one becomes a semantic token or a component token that names one. Stroke widths, chevron geometry and dash lengths stay constants.

The table groups the constants by the component prefix of their names.

| Component | Constants | Of which radii |
| --- | --- | --- |
| Check | 9 | 2 |
| Panel | 4 |  |
| Button | 3 |  |
| Segmented | 1 |  |
| Segment | 1 |  |
| Field | 1 |  |
| Number | 1 |  |
| Property | 6 |  |
| Inspector | 1 |  |
| Toolbar | 1 |  |
| Options | 11 |  |
| Preferences | 8 |  |
| Preset | 5 |  |
| Form | 7 |  |
| Histogram | 7 |  |
| Listbox | 2 |  |
| Menu | 7 |  |
| Section | 5 |  |
| Dock | 19 | 1 |
| Float | 9 | 1 |
| Color | 50 | 2 |
| Swatch | 6 | 1 |
| Gradient | 29 |  |
| Curve | 11 |  |
| Levels | 9 |  |
| List | 15 |  |
| Angle | 7 |  |
| Tab | 9 |  |
| Title | 11 |  |
| Slider | 12 | 1 |
| Range | 3 | 1 |
| Handle | 4 |  |
| Cursor | 17 |  |
| Guide | 15 | 1 |
| Transform | 23 | 2 |
| Bezier | 14 |  |
| Canvas | 2 |  |
| Font | 5 |  |
| Text | 6 |  |
| Glyphs | 9 |  |
| Split | 4 |  |
| Pasteboard | 2 |  |
| Ref | 3 |  |
| Marquee | 7 |  |
| Scroll | 6 |  |
| Ruler | 7 |  |
| Status | 4 |  |
| Command | 5 |  |
| Readout | 5 |  |
| Radio | 1 |  |
| Icon | 6 |  |
| Switch | 2 |  |
| Badge | 1 |  |
| Kbd | 2 | 1 |
| Progress | 1 |  |
| Tooltip | 2 |  |
| Toast | 3 |  |
| Empty | 8 |  |
| Rail | 9 | 1 |
| Dialog | 25 | 1 |
| Popover | 3 |  |
| Crop | 25 |  |
| Grid | 5 |  |
| App | 2 |  |
| Eyedropper | 3 | 1 |
| Sampler | 3 | 1 |
| Screen | 5 |  |
| Paint | 4 |  |
| Harmony | 4 |  |
| Start | 2 |  |
| Tour | 4 |  |
| Selection | 16 |  |
| New | 12 |  |
| Export | 9 |  |
| Theme | 10 | 4 |
| Findings | 1 |  |
| File | 2 |  |
| Table | 6 |  |
| Chip | 6 |  |
| Breadcrumb | 3 |  |
| Date | 4 |  |
| Reset | 2 | 1 |
| Drag | 4 |  |
| Drop | 9 | 2 |
| Event | 2 |  |
| Autocomplete | 2 |  |
| Metadata | 4 |  |
| Rating | 4 |  |
| Motion | 4 |  |
| About | 5 |  |
| Password | 1 |  |
| Align | 1 |  |
| Tree | 2 |  |
| Alt | 2 |  |
| Thumbnail | 1 |  |
| Channels | 1 |  |
| History | 2 |  |
| Layers | 16 |  |
| Tile | 19 | 1 |
| Link | 7 |  |
| Navigator | 9 | 1 |
| Scopes | 12 |  |
| Actions | 6 | 1 |
| Select | 10 | 1 |
| Hover | 5 |  |
| Outline | 2 |  |
| Grip | 5 |  |
| Constrain | 2 |  |
| Line | 2 |  |
| Before | 6 |  |
| Brush | 54 | 3 |
| Full | 10 |  |
| Corner | 7 |  |
| Stroke | 16 |  |
| Measure | 3 |  |
| Symmetry | 3 |  |
| Adjust | 14 | 1 |
| Layer | 8 |  |
| Blend | 7 |  |
| Add | 5 | 1 |
| Shape | 3 |  |
| Slice | 12 | 1 |
| Artboard | 2 |  |
| Connector | 17 |  |
| Vanishing | 12 |  |
| Wide | 5 |  |
| Blur | 16 |  |
| Liquify | 2 |  |
| Filter | 3 |  |
