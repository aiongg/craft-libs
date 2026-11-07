# Writing a component

A component is a Rust type on the contract in `craft_ui::contract`. It supplies its own parts and look; the library runs one show pipeline for it: id, style, sizing, focus, pointer sensing, state, value binding, commands, popups, accessibility, mirroring and the shared test cases. Nothing in a component calls egui's behavioural APIs.

## File layout

One file per component in its family module: `<family>/<component>.rs`, for example `controls/radio.rs`. The file holds the type, its style, its words (`words!`) and a `stories` submodule:

```rust
#[cfg(feature = "testing")]
mod stories {
    use super::*;
    use craft_ui::testing::{StoryData, StoryOutput, story};

    #[story(Block, covers = [Default])]
    fn block(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
        Block::new(CELL).show(ui).into()
    }
}
```

Tests that need more than the shared cases go in `crates/ui/tests/it/<family>/` as `#[craft_ui::testing::trial]` functions.

## Shapes

The shape is a fact of the type, set by what the caller supplies:

| Shape | Declare | Caller supplies | Shown with |
|---|---|---|---|
| Leaf | `#[derive(Component)]` | nothing | `Show::show` |
| Container | `#[component(container)]` and a `#[container] ContainerProps` field, then `impl Container` | a body closure | `ShowContainer::show` |
| Collection | `#[component(collection)]` and a `CollectionProps` field, then `impl Collection` | an item source and an item closure | `ShowCollection::show` |

A type that only assembles components has no pipeline. It is a pattern and implements the `Pattern` marker trait.

The show traits are sealed. A component never has an inherent `show`; `contract/shadow` rejects one.

## The type

```rust
#[derive(Component, Props)]
pub struct Badge {
    #[common]
    common: Common,
    /// The count shown beside the label.
    #[prop]
    count: Option<u32>,
    /// Working: shown dimmed.
    #[prop(flag)]
    busy: bool,
}
```

- `Common` holds what every component has: label, id, variant, size, enabled, read-only, selected, tooltip. Its setters come from `Props`. A component adds no `enabled`, `id_salt` or `tooltip` field of its own (`props/*`).
- `#[prop]` fields become setters. A `#[prop]` field is not an `f32` or a `Vec2`: sizes come from the style (`props/literal-size`).
- The derive writes `<Type>StyleKey`, the setters and the registration that lets the suite find the type. A type that implements `Component` by hand registers with `register_contract!`.

## The trait

`impl Component for Badge` states the type's facts and implements only the hooks it needs:

| Item | Purpose |
|---|---|
| `const ROLE: Role` | The AccessKit role. |
| `type Style`, `fn style(cx, key)` | The look, built from style parts. |
| `type Output` | What `show` returns: `Response`, `EditResponse<T>`, a collection response. |
| `fn sense(&self) -> Sense` | The default follows the shape; override to add or remove click, drag and focus. |
| `fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2` | The content size. Padding and minimum size are added by the pipeline. |
| `fn parts(&mut self, rect, cx: &mut PartsCx<'_, Self>)` | Named sub-rects with `PartKey`, `Sense` and `HitShape`. |
| `fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>)` | Content paint through `PaintUi` and `Draw`. |
| `fn content_env(&self, cx, values: &mut AppValues)` | App values set for the body or items. |
| `fn finish(self, cx: Cx<'_, Self>) -> Self::Output` | The output. |

A component is shown through a parent's `show`, never by allocating a rect itself: `contract/allocate` rejects `allocate_*`, `ui.interact` and `ui.put`.

### Parts and faces

`cx.part(KEY, rect)` registers a part. `ui.face(KEY, Other::new())` paints another component as the look of a part, so a composite reuses the look of the component it wears. A part that only a pointer reaches states why keyboard users do not need it: `.pointer_only("Enter activates the component")`.

### Values

A value component's `Output` is `EditResponse<T>`. It binds with `Bind<T>` and reports through `cx.propose`, `cx.commit` and `cx.adopt`; each gesture ends in one `EditEvent::Commit`. Components with `Input::Editor` own their caret through `ImeSurface`.

## Looks

A style is a plain struct deriving `ComponentStyle`:

```rust
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub struct BadgeStyle {
    #[style(metrics)]
    metrics: Metrics,
}
```

`#[style(metrics)]` marks the `Metrics` field (padding, minimum size, gap); `#[style(surface)]` and `#[style(focus)]` mark the surface and focus-ring parts. The style function reads semantic tokens and `BySize` tables through `StyleCx`. It does not match on state: state colours come from `States`, and `style/*` rejects colour picks from response state, size arms and per-instance setters.

An app changes a look with `Looks::style::<Badge>(f)` before `install`; the replacement is frozen at install. A variant is a `const CustomVariant` with a crate-prefixed id and a look function over semantic tokens; a variant describes itself and needs no registration.

## Tokens

A component reads only semantic tokens. It has no tokens of its own and no unnamed size literals. A crate's own concepts go in a `#[derive(TokenGroup)]` struct whose `derive(tokens)` function gives defaults from the resolved semantic tokens; a theme overrides them under the group's `PATH`.

## Stories

A story shows one registered type in one state. `#[story(Type, covers = [..])]` registers it. The probe of the type lists the states it must show:

| Type has | Required stories |
|---|---|
| any | `Default` |
| a value output | `Mixed`, `Invalid` |
| `Input::Text` or `Input::Editor` | `LongText` |
| `#[component(overlay)]` | `Open` |
| items | `Empty`, `One`, `Many` |

A value a story edits comes from `StoryData::value(name, init)`. The suite fails when a required state has no story. `Drive::Auto` picks how the suite changes the value; a story names another drive (`Drive::Drag(part)`, `Drive::Pick { row }`, `Drive::Custom(f)`) only when the default does not fit. The suite makes the disabled, right-to-left and narrow-room variants itself.

A type that cannot meet a shared case lists it, with a reason:

```rust
#[component(exempt(case = "stories", reason = "A face of the title row, shown only inside it."))]
```

Exemptions are counted in `design/exceptions/<crate>/` and the count cannot rise.

## Seeing and checking the result

- `cargo run -p craft-ui-gallery` shows every registered story in both directions and both control sizes, in light or dark, with a filter.
- `cargo test -p craft-ui --features testing --test it` runs the shared cases and the trials.
- `cargo run -p craft-design-lint -- structure` checks the layer map, imports, exceptions and clones; `check` reports the source rules.
- `cargo run -p craft-ui-gallery --bin components-index` rewrites `docs/components-index.md`, the list of what exists. Read it before writing a type, to avoid a second copy of one that exists.
