# 1. Overview

## 1.1 What the Craft framework is

The Craft framework is the shared layer under every Craft app, supplied by `craft-libs` as one library:

- **Design system.** Tokens, themes and component layers ([4](4-design-system.md) to [6](6-components.md)).
- **Shells and canvas core.** Window skeleton, docking, title bar, rails, options bar, zoom and navigation.
- **Host convention.** How a feature gets services from the app and returns edits ([3.4](3-architecture.md#34-the-host-convention-services-in-edits-out)).
- **Feature families.** Complete features an app adds with little code: colour picker, layers panel, file dialogs, preferences, shortcut editing, font selection.

The apps differ in what they edit, but the parts around the editing are the same. Separate copies drift apart in look, behaviour, accessibility and translation, and a fix in one does not reach the others. The framework builds each feature once and uses it everywhere, even when only one app needs it today.

## 1.2 Framework and design system

| Term | Meaning |
| --- | --- |
| Craft framework | The whole shared layer |
| Design system | Tokens, themes and component layers. One part of the framework |
| `craft-ui` | The kernel crate: the design-system layers every family uses, with no document concept |
| Family | A set of features a developer looks for together (colour, layers, timeline). One crate each |

App engineers need the framework: sections 2 and 3. Component and theme authors need the design system: sections 4 to 7.

## 1.3 Principles

- **Apps ask for meaning.** A raised surface, a primary button, muted text. Never a colour or pixel size. A lint checks app code ([8](8-tests-and-gates.md)).
- **Apps keep their layout and conventions.** The code and behaviour of the building blocks are shared ([4.5](4-design-system.md#45-per-app-look-and-feel)).
- **Behaviour follows the industry original.** Without one, it follows the app's existing behaviour. Where an app draws one kind of component several ways, the library has one.
- **Apps own the document.** Features hold no document state or undo history.
- **Accessibility and translation are required** ([7](7-accessibility-and-translation.md)). A missing translation never blocks.
- **Colour management goes through `craft-cms`.** A missing capability is added there.
- **Builder idiom.** `Type::new(required).option(..).show(ui)`. Panels take `&mut Model`. A dialog's per-pass answer is an `Outcome`; on the host it returns a `DialogResult<T>`.
