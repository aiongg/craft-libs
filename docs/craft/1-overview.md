# 1. Overview

## 1.1 What the Craft framework is

The Craft framework is the shared layer under every Craft app, supplied by `craft-libs` as one library. It holds the design system, shells and canvas core, the host convention and the feature families. The framework builds each feature once and uses it everywhere, so copies cannot drift apart in look, behaviour, accessibility and translation.

## 1.2 Framework and design system

| Term | Meaning |
| --- | --- |
| Craft framework | The whole shared layer |
| Design system | Tokens, themes and component layers. One part of the framework |
| `craft-ui` | The kernel crate: the design-system layers every family uses, with no document concept |
| Family | A set of features a developer looks for together (colour, layers, timeline). One crate each |

App engineers need the framework: sections 2 and 3. Component and theme authors need the design system: sections 4 to 8.

## 1.3 Principles

- **Apps ask for meaning.** A raised surface, a primary button, muted text. Never a colour or pixel size. A lint checks app code ([8](8-tests-and-gates.md)).
- **Apps keep their layout and conventions.** The code and behaviour of the building blocks are shared ([4.5](4-design-system.md#45-per-app-look-and-feel)).
- **Behaviour follows the industry original.** Without one, it follows the app's existing behaviour. Where an app draws one kind of component several ways, the library has one.
- **Apps own the document.** Features hold no document state or undo history.
- **Accessibility and translation are required** ([7](7-accessibility-and-translation.md)). A missing translation never blocks.
- **Colour management goes through `craft-cms`.** A missing capability is added there.
