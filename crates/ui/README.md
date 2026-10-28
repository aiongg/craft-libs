# Craft UI

`craft-ui` provides small egui components with application-supplied styles. `icons` draws the
suite's icons; `buttons` (deprecated) paints icon buttons in application-supplied colours; `tabs` handles tab fitting and
interaction; `layout` provides toolbar fitting, split geometry and resize handles; `theme` shares widget-state styling.
`docking` supplies transactional panel moves and an optional docking surface.
`printing` supplies printer/job controls and asynchronous feedback used by WordCraft and GridCraft.
`fonts` installs the UI font stack, the same fallback order in every app.
`primitives` holds the design system's building blocks (surfaces, state layer, focus ring, divider, stack and row, interactive element, text, icon, canvas handle), styled from `theme`'s tokens.
`controls` holds the design system's controls built on them (button, icon button, toggle button, segmented control, link, check box, radio group and radio set, switch, slider, range slider, swatch, badge, kbd, spinner, progress bar, tooltip, text field, search field, number field, select, combo box).
`composites` puts controls together with their text into form rows: `Field` (a control with its label beside it in a label column or above it, and help or error text under it; AccessKit `labelled_by`, `described_by` and `error_message`) and `FieldGroup` (one label column for a form, as wide as its widest label), `PropertyRow` (an inspector row: a glyph or word label and a control that fills the width; `pair` puts two in a row with an optional link slot; narrow rows truncate essential labels and hide redundant ones; the label can scrub a number field) `PropertyGroup` (a section whose glyph labels share a centre), `SectionHeader` (a panel section's header row: hairline, chevron and title; collapsible with a persisted or host-owned open state and Enter or Space, or always open, with a trailing controls slot; AccessKit `DisclosureTriangle` with `expanded`) and `PanelHeader` (the title row of a flyout or floating panel: title, actions, panel menu, float and close buttons; the background reports drags).
`list` has the rows of lists: `ListRow` (one row of a layers, history or channels list: optional toggle columns with a drag-down sweep reported as one edit gesture, an icon or framed thumbnail, a label with an optional detail line cut with an ellipsis, trailing items whose optional ones drop out before the label gets too narrow, selected, focused, dimmed, highlighted and drop-marked states, and an in-place rename field; AccessKit `ListBoxOption` with `selected` and `position_in_set`; the row keeps no selection or order, so a list drives it) and `List` (the container: one Tab stop that keeps the focus while rows scroll away, a scroll area that lays out only the rows in view for 10,000 rows of one or mixed heights, `ListSelection`'s clicks and keys with reveal, a click on the empty space to clear, rename coordination, a toggle column's key, and a context-menu request with the row and its anchor; AccessKit `ListBox` with `active_descendant`, `size_of_set` and row actions), and `TreeRow` with `List::tree` (indent by depth that squeezes before the label gets narrow, a disclosure triangle with Alt-click for every group, the tree keys with Alt for all, the lead moving to a group that closes over it, AccessKit `Tree` and `TreeItem` with `level`, `expanded` and Expand and Collapse; `TreeState` keeps the open groups in egui memory).
`behavior` holds the keyboard, focus and selection rules that several controls share, such as a dialog's Tab order (`FieldTab`: Tab and Shift+Tab visit only the text and number fields, Ctrl+Tab every control), a radio group's (`RadioSelection`, used by the radio group, the radio set and the segmented control), a line of typed text's (`Typing`, egui's `TextEdit` reported as one edit gesture per typing run, used by the text, search and number fields and the combo box; a `Codec` writes and reads a typed value) and a popup list's active option (`Listbox`: arrows, Page Up and Down, Home, End and type-ahead while the focus stays on the select or the combo box's field), and a list's selection (`ListSelection` over a `DisplayOrder`, with a host-owned `Selection`: click, Shift-click, Cmd/Ctrl-click, the listbox and tree keys, reported as `SelectionChange`), and a drag-to-reorder's rules (`Reorder`, without drawing: `Move` targets with group, gap and no-op rules, `accepts`, copy, the Cmd/Ctrl+] and [ keyboard move, edge auto-scroll speed, the `ListDrag` payload and the `ReorderGesture` event sequence; vertical and horizontal, left to right and right to left).
`inspect` records the primitives' layout boxes and the AccessKit tree for inspector overlays such as the gallery's; off by default.
`edit` is the edit lifecycle value controls report through: gesture events (begin, update, commit, cancel, one-shot set), the `Gesture` state machine controls embed, and `GestureHost`, which turns the events into undo steps.
`localized` writes and reads the user's region's numbers (`LocaleNumbers`, `Unit`); its number fields are deprecated for `controls::NumberField`.
The current delivery covers nine creative apps (DesignCraft, VectorCraft, PhotoCraft, FilmCraft, EffectCraft, CADCraft, PDFCraft, LightCraft and SoundCraft); prior basic and office adapters are separate. The [suite audit](../../docs/ui-components-audit.md#adoption-and-acceptance)
records their scope and acceptance limits. Assets (font files included), complete themes, localization
and command handling remain in each application. The suite's icons are bundled (`icons/`, with
their licences in `icons/ATTRIBUTION.md`); fonts and application engines are not.

## Icons

```rust
use craft_ui::icons::{self, Icon, IconSize};

fn close_glyph(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    icons::paint(ui, rect, Icon::Close, IconSize::Md, ui.visuals().text_color());
}
```

`icons/icons.toml` lists the icons by meaning and each candidate set's file for them; `build.rs`
embeds the files and generates `Icon` and `IconSet`. Icons rasterise once per pixel size with `resvg`
and are tinted when drawn. `icons::set_active` picks the set for a context (Lucide by default); the
gallery's `specimens/icons/compare` story shows the four candidates side by side.

## SVG catalogs (deprecated)

`SvgIconSet` serves an app's own SVG files through egui's image loaders. Use `Icon` instead.

```rust
use std::sync::OnceLock;
use craft_ui::icons::{SvgIconSet, SvgStyle};

// Supply the application's own attributed assets here.
static ICONS: &[(&str, &[u8])] = &[];

fn icons() -> &'static SvgIconSet {
    static ICONS_SET: OnceLock<SvgIconSet> = OnceLock::new();
    ICONS_SET.get_or_init(|| SvgIconSet::new(
        ICONS,
        SvgStyle { current_color: "#ffffff", stroke_width: Some("1.75") },
        "bytes://my-app/icons/",
        "square",
    ))
}

fn icon(name: &str) -> egui::Image<'static> {
    egui::Image::new(icons().source(name))
}
```

The application installs `egui_extras::install_image_loaders` with its `svg` feature before drawing.
It sets tint and size on the returned image.
The library does not install loaders or an egui theme.

The catalog is immutable. SVGs are prepared once and held in `Arc<[u8]>`; lookups share those bytes.
The fallback uses the requested name in its URI, matching PhotoCraft and VectorCraft. Missing both
the requested name and fallback produces empty bytes for the loader to report as an error, without
indexed lookups or a panic. `contains` and `canonical_name` inspect only actual catalog entries. PdfCraft resolves an unknown
name to `square` before calling `source`, preserving its fallback cache key.

Use a different URI prefix for every catalog/style combination used in the same egui context.
egui caches by URI; supplying different bytes under one URI would reuse stale content. Apps with
their own contexts can retain their current prefixes. Unknown names do not add entries to this
catalog, though the egui loader may cache a requested URI.

This is an adapter for trusted, compiled-in monochrome assets, not an SVG sanitizer. It performs
literal `currentColor` and optional `stroke-width="2"` substitutions, keeps other widths and explicit
colors untouched, and uses the existing lossy UTF-8 behavior. General SVG parsing and rasterization
remain the responsibility of the existing loader.

## Icon buttons (deprecated)

`buttons::IconButton` and `IconButtonStyle` are deprecated and stay until the last app moves to
`controls::IconButton`, which takes its colours from the theme. `buttons::IconButton` accepts a label, size and `IconButtonStyle`, and returns `egui::Response`.
`ButtonVisuals` describes fill, border and icon tint for normal, hovered, pressed and selected
states. Selection wins over press and hover. `pressed: None` keeps ordinary hover/normal styling;
`Some` supplies a separate press treatment. egui owns pointer capture and click cancellation.

```rust
use craft_ui::buttons::{ButtonVisuals, IconButton, IconButtonStyle};
use egui::{Color32, CornerRadius, Stroke, Vec2};

fn layer_button(ui: &mut egui::Ui, selected: bool) -> egui::Response {
    let normal = ButtonVisuals {
        fill: Color32::TRANSPARENT, stroke: Stroke::NONE, icon: Color32::WHITE,
    };
    let style = IconButtonStyle {
        normal,
        hovered: ButtonVisuals { fill: Color32::DARK_GRAY, ..normal },
        pressed: None,
        selected: ButtonVisuals { fill: Color32::BLUE, ..normal },
        corner_radius: CornerRadius::same(4),
        focus_stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
    };
    IconButton::new("Layers", Vec2::splat(28.0), &style).selected(selected)
        .show(ui, |ui, rect, tint| {
            // Use the application's SVG catalog or its existing painter icon here.
            ui.painter().circle_filled(rect.center(), 5.0, tint);
        })
}
```

Use actual theme colors in the style adapter and localized text for the label. The painter receives
the full button rectangle and icon tint; the app chooses icon padding/size. Parent disabled state
applies to painting, interaction and accessibility. The library reports label and selection to
AccessKit and draws the supplied inset focus outline. Mouse, Enter and Space activation come from
egui. The caller reads `clicked()`, runs its command and updates selection; the component neither
dispatches nor toggles state. Attach tooltips to the returned response.

Controls previously painted with one `egui::Painter::rect` can opt into `.single_shape_frame()`.
This keeps their fill/border edge tessellation, especially for translucent disabled borders.
The default paints fill and border separately, preserving existing consumers' rendering.

IDs come from the surrounding UI scope/position, independent of the translated label. Use
`ui.push_id(stable_id, ...)` when items reorder. Dimensions must be finite, positive and no larger
than 4096 points; other sizes produce an inert zero-size response. Invalid stroke widths are
ignored and large strokes are bounded to half the smaller button dimension.

## Tabs

`tabs::fit(natural_widths, selected, available, minimum, overflow_width)` returns visible tab
indices and widths, overflow indices, and the actual reserved overflow width. It shrinks wide
tabs before hiding them, keeps input order and retains the selected tab. Tiny strips may reduce
its width to zero. Negative/non-finite dimensions become zero and large dimensions are bounded.
The work is linear in tab count with at most 32 fitting passes; the caller caps its own tab count.
Docking tab strips use this policy, with `DockStyle::min_tab_width` as the minimum.

`tabs::fit_whole(natural_widths, selected, available, overflow_width)` keeps every shown tab at its
natural width and sends the rest to overflow. Tabs are shown in order up to the first that does not
fit; the selected tab is always shown in its logical place, taking the place of the last tabs that
fit when it lies past them. Only a selected tab wider than the strip minus the overflow button is
narrowed.

`tabs::Tab` registers a tab at a supplied rectangle and invokes the application's painter:

```rust
fn tab(ui: &mut egui::Ui, rect: egui::Rect, selected: bool) -> egui::Response {
    craft_ui::tabs::Tab::new(ui.scope_id().with("layers-tab"), "Layers", selected)
        .sense(egui::Sense::click_and_drag())
        .show_at(ui, rect, |ui, response| {
            let fill = if response.hovered() { egui::Color32::GRAY } else { egui::Color32::DARK_GRAY };
            ui.painter().rect_filled(response.rect, 0.0, fill);
        })
}
```

Supply a stable ID independent of the translated label. The component reports the accessible
Tab role, label and selection, handles egui keyboard activation, and paints a focus outline from
the caller's theme. The caller keeps selection, labels, overflow menus, painting and drag actions;
close buttons and tab context menus stay separate. Dock and floating-panel adapters use the same
component without sharing window or workspace state.

## Splits and resize handles

`layout::split_rect` divides a rectangle along `SplitAxis::Horizontal` (left/right children) or
`Vertical` (top/bottom). `SplitSize` accepts a ratio, fixed first pane, or fixed second pane in
logical points. The result contains `first`, `divider` and `second` rectangles. Gap and minimum
are supplied by the application. Tiny areas relax the minimum to half the space available;
non-finite or inverted input returns `None` instead of panicking.

```rust
use craft_ui::layout::{split_rect, SplitAxis, SplitSize};

fn two_panes(rect: egui::Rect) -> Option<(egui::Rect, egui::Rect)> {
    let split = split_rect(rect, SplitAxis::Horizontal, SplitSize::Ratio(0.5), 4.0, 20.0)?;
    Some((split.first, split.second))
}
```

`layout::ResizeHandle` receives a stable ID, axis, divider rectangle, localized label and
`ResizeHandleStyle` colors. `show` returns an egui response and a movement delta for that frame.
Pointer drags, focused axis-arrow keys and AccessKit increment/decrement actions all produce
that delta. `value(current, range)` adds accessible numeric bounds and SetValue support. Tab
continues normal focus traversal. Invalid geometry or numeric parameters make the handle inert.

Apply the delta through `SplitLayout::resized(size, delta, minimum)` to retain fixed/ratio sizing,
or use an application-specific adapter. The library does not persist geometry, dispatch commands
or create panes. Apps keep their existing dock trees, stacked panels, layout files, pane contents,
cameras, document focus and native windows. Shared controls do not constitute a dock-tree migration.

## Panel docking

`docking::Layout<P>` owns panel placement independently of application content: a tree of split,
tab and accordion nodes plus floating tab groups. `apply(Action)` validates the complete layout,
applies a proposed change to a copy and commits only a valid result. Moves, tab insertion/reorder,
side splits, detach, activate, close, reveal and resizing use this one transaction path. Panel IDs
must be unique, stable values; labels and document state are supplied by the app.

`Layout::location` captures a panel's placement before hiding it; `restore` returns it there if
the remembered destination still exists. Apps choose an explicit fallback when it does not.
Enable the optional `serde` feature to persist the shared layout, or convert to/from an existing
workspace schema. Validate loaded input before rendering or recursively converting it.

`DockArea` is an optional renderer. It takes a layout snapshot, styles, labels, panel permissions
and body callbacks, then returns actions for the app's command handler. Its floating groups stay
inside the current egui viewport. Protected document canvases have no tab header. Per-panel size
limits constrain geometry without overwriting saved preferred widths when the window shrinks.
Applications with established chrome can use the shared transaction model and `resolve_drop`
without changing their renderer. See [the docking guide](../../docs/ui-docking.md) for details.

## Toolbar overflow

`layout::fit_toolbar(natural_widths, available, spacing, overflow_width)` keeps an ordered
prefix of indivisible groups at their natural widths. When groups do not fit, it reserves the
full overflow-control width and the preceding gap. `ToolbarFit` contains visible `(index, width)`
pairs, overflow indices and the reserved width. It does not skip wide groups or compress controls.

The application paints an enabled, focusable overflow control and makes every overflow index
available in its menu through existing commands. Group identity, labels, selection and command
dispatch remain local. An empty input produces an empty fit. Invalid dimensions, more than 4096
groups, or an available width too small for the overflow control return `None`; the caller must
then provide a scrollable row, wrapping or an existing application menu. Never turn that result
into silently missing commands. See the API documentation for the accepted numeric bounds.

## Theme mechanisms

`theme::WidgetPalette` applies background colors to egui's five widget states, with a supplied
corner radius and foreground stroke. VectorCraft, DesignCraft and LightCraft construct it from their existing
local tokens. It updates each state's strong/weak background fill, radius and foreground; borders,
expansion and all other visual settings are retained.

```rust
use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Stroke};

fn style_widgets(visuals: &mut egui::Visuals, palette: WidgetPalette, text: Color32) {
    palette.apply_to(&mut visuals.widgets, CornerRadius::same(4), Stroke::new(1.0, text));
    // Optional app-specific exceptions belong after the shared operation.
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
}
```

This is a mechanism for coloring widgets, not a shared theme schema. Complete palettes,
selection colors, shadows, spacing, context storage and preference serialization remain in each
application; fonts are installed by `fonts`, below. See the
[eight-app comparison](../../docs/theme-infrastructure.md) for the exact common operation and the
differences preserved by the adapters.

## UI fonts

egui draws a character only if one of its installed faces has it, so an app's labels, layer names
and file names show boxes for any script its faces miss. `fonts::UiFonts` builds every app's UI
font stack in the same order:

1. the app's own faces for the family (UI face, semibold, mono), or egui's default faces for a
   family the app has none for;
2. craft-fonts faces for the UI language;
3. craft-fonts faces for every other script the app ships, so a Japanese interface still draws
   Arabic file names;
4. installed system fonts (see below), set to regular weight so variable fonts don't draw Thin;
5. emoji faces (system emoji first, then the colour, web and monochrome faces the app embeds) and,
   behind the app's own faces, egui's default faces.

When the faces of step 1 don't cover the UI language's main script (Arabic, Thai, Georgian), that
script's face goes first. egui takes a row's height from a family's first face, so the language's
own face sets the row height and its marks aren't clipped. Han, kana and Hangul sit inside the em
square and fit a Latin row, so Chinese, Japanese and Korean UIs keep the Latin face first: Latin
text draws in the app's face in every language, and CJK characters fall back to the language's
face. Rows and widgets keep the same height in every one of these languages as in a Latin UI:
CJK faces sit on the first face's baseline inside its row, scaled down only if a glyph would
otherwise leave the row (Noto Sans CJK and BIZ UDPGothic at the same size don't need it), with
crates.io egui and the suite's patched egui alike. `UiFonts::cjk_rows(CjkRows::Grow)` opts into
the CJK face's taller line instead. The app's theme sets the minimum widget height for rows of its
own face; when the UI language's script face leads the stack and has taller rows, `install` adds
the difference to the theme's height, and takes it off again when a later install needs less. The
theme's height and padding stay the app's: a theme applied after `install` is fitted at the start
of the next pass, so the result doesn't depend on which comes first. On the suite's patched egui
(`FontTweak::set_baseline_fallback`, which the other craft faces and installed text faces turn
on), other fallback glyphs sit on the row's baseline and rows grow to fit them by themselves; emoji
and symbol faces (colour glyphs, or "Emoji" or "Symbol" in the name) stay centred. With crates.io
egui those fallbacks are centred. Monospace keeps the app's or egui's mono face first. Han characters take the regional face the language needs: `zh-TW` gets the
Traditional (TC) face, then Hong Kong, then Simplified; `zh-HK` the Hong Kong face first; `ja` the
Japanese face; `ko` the Korean one.

The app keeps its font files: its `build.rs` embeds the craft-fonts faces it wants from
`fonts/manifest.txt` with `craft-fonts-build`, and craft-ui never depends on craft-fonts. A crate
below the UI (an app's engine or font crate, without egui) holds the table as
`craft_fonts_build::EmbeddedFace` values (storytold/craft-libs#213), and the UI converts each into
a `fonts::CraftFace` with `From`. Apps embed with `Embed::subset_to(catalogs)`, which takes the
CJK faces as craft-fonts' interface faces (`Noto Sans TC UI`, BIZ UDPGothic Regular) and cuts them
at build time to the characters the translation catalogs use (about 3 MB for all five with
FilmCraft's catalogs). A character a cut face lacks, such as one in a file name, comes from
another region's interface face that has it, then from the installed system fonts; the build
warns when a catalog has characters no embedded face covers. Preferences name the full family
(`Noto Sans TC`) and also apply to its interface face (`Noto Sans TC UI`).

The app configures its own faces, a preferred face per script, extra named families (a semibold
family, a family led by Arabic for the runs it shapes itself), emoji faces, and whether system
fonts come before or after the craft faces for other scripts. `UiFonts::stack` returns a `Stack`
whose chains the app can still edit (`insert`, `replace`, `remove`) before `Stack::install`.
`Stack::after_system` (a `FaceList`) holds the faces asked after the system fonts, readable with
`get` and `faces`; `Stack::flat_definitions` appends them to each chain, for a context that takes
its fonts only through `set_fonts`.

`fonts::I18nScripts` reads each language's scripts and Han forms from craft-i18n; an app with its
own table implements `fonts::LanguageScripts` instead.

```rust
use craft_fonts_build::EmbeddedFace;
use craft_ui::fonts::{CraftFace, Family, I18nScripts, UiFonts};
use egui::{FontData, FontFamily};

pub static CRAFT_FONTS: &[EmbeddedFace] = include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

fn install_fonts(ctx: &egui::Context, ui_language: &str) {
    let craft_faces: Vec<CraftFace> = CRAFT_FONTS.iter().copied().map(CraftFace::from).collect();
    UiFonts::new(&I18nScripts)
        .app_face("ui", FontData::from_static(UI_FONT), &[FontFamily::Proportional])
        .app_face("ui-semibold", FontData::from_static(UI_SEMIBOLD), &[FontFamily::Name("semibold".into())])
        .app_face("mono", FontData::from_static(MONO_FONT), &[FontFamily::Monospace])
        .family(Family::named("semibold").weight(600))
        .craft_faces(&craft_faces)
        .languages(OFFERED_LANGUAGES)
        .ui_language(ui_language)
        .install(ctx);
}
```

Call `install` again when the UI language changes.

### Web builds

A web build serves the craft-fonts faces beside its wasm instead of embedding them, so the wasm
stays small and an English interface never downloads Chinese. `craft-fonts-build`'s
`Embed::serve` copies the faces into a directory and writes a table of `fonts::ServedFace`s (URL,
size, Subresource Integrity hash and the characters the face maps, no bytes);
`UiFonts::served_faces` takes it in place of `craft_faces`. Each face's characters are its
`cmap`'s code point ranges, packed as LEB128 gaps and lengths: about 3 KB for a CJK interface face
cut to an app's catalogs, a few hundred bytes for Noto Sans, the symbol faces or an emoji face. The
stack orders served faces exactly as embedded ones, and on the web `Stack::install` fetches them:

- the UI language's faces at once (`Stack::ui_downloads`: the best face for each of its scripts
  the app's faces lack, per family, so a semibold family brings the bold face);
- for a character no installed face has, the one face that draws it (`Stack::downloads_for`):
  the first face in the family's order whose file maps the character and whose scripts fit it.
  Noto Sans draws Latin, Greek and Cyrillic letters the app's faces lack (Vietnamese ẫ), the
  symbol faces (Noto Sans Symbols, Symbols 2, Math) keyboard keys, arrows and other text symbols
  (⌘ ⌥ ⇧ ⎋ ⏎), the emoji faces emoji. A Han character takes the interface face of the UI
  language's region first (Simplified, Traditional, Hong Kong, Japanese or Korean), then the
  other offered languages' and the rest in manifest order;
- a served emoji face (`Emoji::served`) the first time text needs an emoji.

A character no served face maps, such as a rare Han character none of the cut interface faces
has, fetches nothing: egui's web glyph rasterizer draws it with the browser's fonts, or it shows
as a missing glyph. A file that
fails to download leaves its characters to the next face that draws them and is fetched again
after 2 seconds, then after twice as long with each further failure, up to 10 minutes. Each face is
added when its file arrives, and the context repaints; text falls back to the faces after it until
then. Fetched files stay in memory for later installs, such as a language switch, and the
browser's HTTP cache keeps them across page loads. `fonts::loading(ctx)` is true while the UI
language's files are on their way, for an app that would rather draw nothing than Latin fallbacks
in its first frames.

`build.rs`:

```rust
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    // CJK faces as craft-fonts' interface faces, cut to the app's catalogs (`catalogs` as in
    // craft-fonts-build's README; `subset` feature on the build-dependency).
    let fonts = craft_fonts_build::Embed::new().subset_to(catalogs("myapp"));
    let written = if craft_fonts_build::target_is_wasm() {
        // Served by the page as `fonts/`; see Trunk.toml.
        fonts.serve("web/fonts", "fonts/").write("craft_fonts.rs")
    } else {
        fonts.write("craft_fonts.rs")
    };
    if let Err(e) = written {
        println!("cargo::error={e}");
    }
}
```

The app:

```rust
// The embedded table is written as `EmbeddedFace` literals, the served one as `ServedFace`s.
use craft_ui::fonts::{CraftFace as EmbeddedFace, I18nScripts, ServedFace, UiFonts};

#[cfg(not(target_arch = "wasm32"))]
static CRAFT_FONTS: &[EmbeddedFace] = include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));
#[cfg(target_arch = "wasm32")]
static CRAFT_FONTS: &[ServedFace] = include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

fn install_fonts(ctx: &egui::Context, ui_language: &str) {
    let fonts = UiFonts::new(&I18nScripts).app_face("ui", FontData::from_static(UI_FONT), &[FontFamily::Proportional]);
    #[cfg(not(target_arch = "wasm32"))]
    let fonts = fonts.craft_faces(CRAFT_FONTS);
    #[cfg(target_arch = "wasm32")]
    let fonts = fonts.served_faces(CRAFT_FONTS);
    fonts.languages(OFFERED_LANGUAGES).ui_language(ui_language).install(ctx);
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if craft_ui::fonts::loading(ui.ctx()) {
            return; // The fetch repaints when it arrives.
        }
        // …
    }
}
```

Call `install_fonts` from the `eframe::App` creator, so the fetches start before the first frame.
The served directory has to reach the site next to the wasm; with Trunk, copy it in a hook
(`Trunk.toml`):

```toml
[[hooks]]
stage = "post_build"
command = "sh"
command_arguments = ["-c", "cp -R web/fonts \"$TRUNK_STAGING_DIR/fonts\""]
```

A relative `serve` directory is under the package whose `build.rs` runs it; a crate deeper in the
workspace can pass `concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/fonts")` instead. Keep the
directory out of version control. A build filters faces for the web as for the desktop (`scripts`,
`filter`), e.g. to leave out bold faces.

Two features, both off by default, change what every craft-ui user draws, so the app turns them on:

- `system_fonts` (desktop): the stack asks the installed fonts, at regular weight, and `install`
  replaces the context's font providers, including the one eframe adds. One index of the
  installed fonts serves every UI language, so a language switch doesn't scan them again.
  `install` starts building it on a background thread; until it is ready, text draws with the
  stack's own faces, and the context repaints when it arrives. Without it,
  `install` keeps the providers the context has (eframe's `system_fonts`, for one) and asks the
  stack's own faces after them. `UiFonts::system_provider` supplies a provider either way, and
  `SystemFonts::Off` drops system fonts. It also gives the OS's own interface face, which
  `UiFonts::interface_font(InterfaceFont::System)` puts ahead of the app's UI face (and its
  semibold face, if the app has a `SEMIBOLD` family). The face comes from the same index, so the
  UI thread doesn't wait for it: until it resolves the app's faces draw, and the stack is
  installed again with it at the start of the next pass. `fonts::poll_system_ui_face(ctx)` asks
  without waiting (to offer the setting), `fonts::system_ui_face()` waits (checks, tests). Without
  the feature, and on the web, there is no system face and the app's faces stay first.
- `color_fonts`: colour glyphs (COLR, sbix, CBDT), for colour emoji.

egui 0.37's default faces have only a few monochrome emoji and lack symbols such as ⊞ ☰ 🔍. On
the web there are no system fonts, so emoji draw as boxes unless the app sets an `Emoji` face.
`UiFonts::egui_face` puts one of egui's own faces into a family, e.g. Hack in the proportional
family for ⇧ and ▸.

Each app tests its coverage with `fonts::assert_covers`: it checks every shipped translation and a
sample of content scripts against the stack without system fonts, and fails naming the language,
message key and character that has no glyph. Build the stack with `.system_fonts(SystemFonts::Off)`
in that test, and run it with `CRAFT_FONTS_DIR` set so it sees the faces a release embeds. For
languages the app leaves to installed fonts, `fonts::assert_covers_with_system` asks the stack's
system provider too; `fonts::system_font_provider(locale)` gives egui's provider (with
`system_fonts`), shared per locale, for `UiFonts::system_provider` or such a check. At run time,
`Stack::covers_language(language, with_system)` says whether the stack draws a language's scripts,
for leaving a language out of the app's menu on a machine that can't show it.

## Testing screens in each language

The `screen-testing` feature adds `testing::Screens`, for an app's screenshot tests of its key screens in
Arabic, Japanese, Traditional Chinese (Taiwan), Korean, Thai and Vietnamese (`testing::LANGUAGES`).
Each language gets an egui_kittest harness with the app's font stack installed for that UI language
and system fonts off, so snapshots match on every machine and a character no face draws fails the
test. Right-to-left languages lay the screen out from the right. The app's UI closure receives the
`Locale` (language tag and direction) and picks its strings from it.

```rust,ignore
use craft_ui::testing::{KOREAN, Screens, compose};

let screens = Screens::new("rename").size([320.0, 150.0]).fonts(|fonts| fonts.craft_faces(FACES));
// Compares tests/snapshots/rename-ar.png, rename-ja.png, …; UPDATE_SNAPSHOTS=1 writes them.
screens.snapshots(|ui, locale| rename_dialog(ui, locale));

// IME smoke test: a scripted 2-Set Korean composition into the focused field.
let mut harness = screens.harness_state("ko", |ui, locale, state: &mut Dialog| state.ui(ui, locale), Dialog::default());
harness.get_by_role(egui::accesskit::Role::TextInput).focus();
compose(&mut harness, &KOREAN);
assert_eq!(harness.state().name, "한");
```

`compose` sends the preedit and commit events an input method sends (`KOREAN`, `JAPANESE`,
`PINYIN`, `TELEX`), which egui's `TextEdit` and canvases reading `Event::Ime` both take. The module
documentation has the recipe for a shortcut smoke test on a non-Latin layout through craft-keymap.
`tests/screens.rs` is a complete example. egui's default proportional face has no Vietnamese
letters such as ổ, so an app relying on it fails the `vi` screens until its own face covers them. The per-release manual checks are in
[language-testing.md](../../docs/language-testing.md).

## Print settings and feedback

`printing::State` holds the transient printer selection, copies, duplex and grayscale settings,
plus discovery and submission receivers. Hosts supply `Discover` and `Submit` workers; neither
the state nor the controls invokes an OS spooler or generates PDF content. Apps map `Settings`
to `craft-print::Job` and own the Print action, page selection, preview and export behavior.

`State::show` takes a stable `egui::Id` and app-localized `Labels`. It draws the settings only;
apps render typed `Feedback` using their own translations. `poll` requests occasional repaints
while work is pending. Refresh disables submission until the new printer list arrives, preserves
a still-present selection, and otherwise chooses the system default or first available printer.
An empty or failed discovery cannot leave a stale printer ready to submit.

`ready` is false while a job is pending. `begin` refuses replacing an active job receiver;
dropping a receiver does not cancel a native job. Apps must test readiness before invoking the
host submit callback and keep active work alive when closing a window. Queue-default duplex is
represented by `None`, independently of explicit one-sided, long-edge or short-edge choices.
Receipts and backend errors remain unmodified data for the app to display. These controls work
on WASM too; browser PDF printing remains an app-owned service.

## Localized numbers

`localized::LocaleNumbers` writes numbers in the region's digits, decimal mark and grouping and
reads back any digits. Build it from the app's `I18n` with `.with_catalog(&i18n, lang)`, so the
error tooltips come from the catalog rows keyed by `localized::MESSAGES` (English where a row is
missing). `localized::Unit` spells a unit around a number (`{n} px`, Turkish `%{n}`).

`controls::NumberField` is the panel and dialog field. It reports through the edit lifecycle
(`edit`): a typing run is one gesture read on commit. Text that can't become a number is outlined
as it is typed, with the reason in a tooltip; text the commit can't read stays, outlined. Its
default width is `field.number_width`.

```rust
let label = ui.add(Text::new(i18n.t(lang, "Width")));
let edit = NumberField::new("width", i18n.t(lang, "Width"), width, &numbers)
    .range(0.0..=1e4)
    .unit(Unit::translated(&i18n, lang, "{n} px"))
    .scrub_by(&label)
    .mixed(widths_differ)
    .show(ui);
```

- `.scrub_label(text)`, `.scrub_icon(icon)` or `.scrub_by(&label)` scrub the value with a drag: a
  step a point, Shift ten, Alt a tenth as fast; a drag is one gesture and Esc puts the value back.
- Arrow keys, the wheel over the focused field and screen-reader increments step to the next
  multiple of `step` (5.1 up to 6, down to 5); Shift steps ten, Ctrl/Cmd a tenth where the
  decimals show it. Text that isn't a number takes no step and stays.
- `.or_parse(reader)` reads what the locale can't (arithmetic, timecodes); `.popup_slider(name)`
  adds a ▾ that opens a slider over the range, which must be finite.
- `.mixed(true)` shows a blank field for values that differ between objects; a typed number is a
  change even when it equals the host's value, which is where scrubs and steps start.
- `localized::focus_first_field(ctx, true)` when a dialog opens: the first enabled field drawn
  takes the focus with its text selected.

`localized::NumberField` and `LocalizedDragValue` are deprecated, kept until the last app moves to
`controls::NumberField`.

## Screenshot completion

`capture::CaptureSlot` is a one-result slot for egui 0.37's one-shot screenshot callback. It is not a
capture scheduler: request IDs, output paths, deadlines, settling frames, geometry epochs and the
aggregate admission reservation stay in the application. The slot owns strong `Context` and
`ViewportId` handles only in the live request; the callback holds a weak reference, so closing the
request cannot leave a callback-owned context cycle. Keep the slot in the application's request
record, outside egui-owned storage: a slot stored in `egui::Memory::data` or a registered plugin
would form a `Context` → slot → `Context` cycle.

```rust
use craft_ui::capture::{CaptureError, CaptureSlot};
use egui::{Context, ViewportCommand, ViewportId};

fn issue_capture(ctx: &Context, viewport: ViewportId) -> CaptureSlot {
    // A per-image bound, not the application's aggregate admission budget.
    let (slot, callback) = CaptureSlot::new(ctx, viewport, 128 * 1024 * 1024);
    // A pending screenshot command requests the paint that hidden windows need.
    ctx.send_viewport_cmd_to(viewport, ViewportCommand::Screenshot(callback));
    slot
}

fn report_capture(slot: &CaptureSlot) {
    if let Some(result) = slot.take() {
        match result {
            Ok(image) => {
                // Validate the app-owned request/geometry epoch, then encode and reply.
                let _ = image;
            }
            Err(error) => {
                // Report the completion error through the app's request record.
                let _: CaptureError = error;
            }
        }
    }
}
```

`take` returns the image or completion error at most once; `None` means pending, cancelled, already
taken or dropped at that moment. The slot cannot distinguish those closed states, so the
application's request record owns the pending/closed distinction and the associated timeout or
error reply.

Completion checks a non-zero width and height, `width * height * size_of::<Color32>()` with checked
arithmetic, a pixel buffer that matches the reported dimensions, and the per-image byte bound. A
zero width or height is `MalformedImage`; an overflowing pixel or byte count is
`DimensionsOverflow`. `CaptureError` is `#[non_exhaustive]`, so match the known variants with a
wildcard arm. The slot releases its mutex before requesting a repaint, and `cancel`/`Drop` replace
the state and drop any retained image only after unlocking, so a large image is not freed while
another thread is completing.
Both accepted images and rejection errors wake the owning viewport so the application can take
the result and reply promptly.

Every retry needs a fresh `CaptureSlot` and a fresh callback from `CaptureSlot::new`. A callback is
one-shot and a slot never accepts a second completion, even after `take` removed the first result.
Issue an explicit screenshot command only once per slot, after the application has settled the
frame it wants. The first delivered frame wins; later frames delivered to the same callback are
discarded. A completion is published before the wake: a completion racing `cancel` or `Drop`
may already have published its result, so it can still deliver one harmless extra repaint after
cancellation. If the cancellation won, the image is released and a later `take` returns `None`; if
`take` won first, the caller already owns the result. A completion that observes the cancelled or
taken state stores no image and does not wake. Cancellation and wakeup are therefore not one atomic
step, and the helper never blocks a completing thread under its lock.

`complete` calls `Context::request_repaint_of`, which takes egui's context lock. Completing from an
ordinary `Ui` closure, an egui 0.37 plugin hook or a renderer/worker thread is fine: these run outside
the context lock. Repaint observers and request-repaint callbacks run under that lock, so completing
there would deadlock. An observer can take or cancel a slot because those operations only use the
slot lock, but further `Context` access must happen outside the observer.

Correlation is based on callback ownership. The helper cannot detect a callback sent to the wrong
viewport if the returned image otherwise fits its bound. The application must route each callback
to its recorded viewport and validate its request and geometry; native multi-window tests must
verify the actual pixels as well as callback routing.

## Text reading areas

`text::align_galley_rows` positions already shaped rows inside a fixed-width reading area without changing glyphs or logical text. Supply a per-row alignment, for example the paragraph direction from the application shaper. The helper uses stock egui APIs and keeps cached glyph meshes shared. Its logical rectangle is the requested width; ink may overflow, so clip the painter when drawing cards. Use it for read-only painting, keeping the original galley for editable hit testing.

## Compatibility and checks

The crate uses egui 0.37 and Rust 1.95 or later. Its production dependency disables egui's default
features, so egui's bundled fonts and rendering features remain the application's choice. It has
no default features; see [UI fonts](#ui-fonts) for `color_fonts` and `system_fonts`, and
[Testing screens in each language](#testing-screens-in-each-language) for `screen-testing`;
`testing` adds the headless docking driver `docking::testing`. See the
[suite audit](../../docs/ui-components-audit.md) for candidates, policies and adoption order.

Validation:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --target wasm32-unknown-unknown -p craft-ui
```

The [visual regression suite](../../docs/ui-visual-regressions.md) compares the actual controls
against independent reference painters, including exact mesh and RGBA checks, and proves that
color, spacing and stroke changes fail those comparisons. To retain reviewable PNG pairs:

```sh
CRAFT_UI_VISUAL_DIR=/absolute/path/to/visuals cargo test -p craft-ui --test visual_regression
```
