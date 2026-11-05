# Craft framework

The Craft framework is the shared layer under every Craft app: the design system, shells, canvas core, host convention and feature families, supplied by `craft-libs` as one dependency.

## Contents

- [1. Overview](1-overview.md)
  - [1.1. What the Craft framework is](1-overview.md#11-what-the-craft-framework-is)
  - [1.2. Framework and design system](1-overview.md#12-framework-and-design-system)
  - [1.3. Principles](1-overview.md#13-principles)
- [2. Installation](2-installation.md)
  - [2.1. Run the component gallery](2-installation.md#21-run-the-component-gallery)
  - [2.2. Depend on `craft` from an app](2-installation.md#22-depend-on-craft-from-an-app)
- [3. Architecture](3-architecture.md)
  - [3.1. Crate tiers](3-architecture.md#31-crate-tiers)
  - [3.2. The `craft` facade](3-architecture.md#32-the-craft-facade)
  - [3.3. Crate taxonomy](3-architecture.md#33-crate-taxonomy)
  - [3.4. The host convention: services in, edits out](3-architecture.md#34-the-host-convention-services-in-edits-out)
  - [3.5. One egui revision](3-architecture.md#35-one-egui-revision)
- [4. Design system](4-design-system.md)
  - [4.1. Component layers](4-design-system.md#41-component-layers)
  - [4.2. Ownership](4-design-system.md#42-ownership)
  - [4.3. Generic or family component](4-design-system.md#43-generic-or-family-component)
  - [4.4. Patterns and shells](4-design-system.md#44-patterns-and-shells)
  - [4.5. Per-app look and feel](4-design-system.md#45-per-app-look-and-feel)
  - [4.6. Customization ladder](4-design-system.md#46-customization-ladder)
  - [4.7. Themes](4-design-system.md#47-themes)
- [5. Foundations](5-foundations.md)
  - [5.1. Colour](5-foundations.md#51-colour)
  - [5.2. Typography](5-foundations.md#52-typography)
  - [5.3. Iconography](5-foundations.md#53-iconography)
  - [5.4. Dimensions](5-foundations.md#54-dimensions)
  - [5.5. Elevation](5-foundations.md#55-elevation)
  - [5.6. Motion](5-foundations.md#56-motion)
  - [5.7. Appearance and contrast](5-foundations.md#57-appearance-and-contrast)
    - [5.7.1. Settings](5-foundations.md#571-settings)
    - [5.7.2. Precedence](5-foundations.md#572-precedence)
    - [5.7.3. Contrast](5-foundations.md#573-contrast)
- [6. Components](6-components.md)
  - [6.1. What every component provides](6-components.md#61-what-every-component-provides)
  - [6.2. Index by layer](6-components.md#62-index-by-layer)
  - [6.3. Family components](6-components.md#63-family-components)
- [7. Accessibility and translation](7-accessibility-and-translation.md)
  - [7.1. Required for every component](7-accessibility-and-translation.md#71-required-for-every-component)
  - [7.2. Translation rules](7-accessibility-and-translation.md#72-translation-rules)
  - [7.3. Goals beyond the required list](7-accessibility-and-translation.md#73-goals-beyond-the-required-list)
- [8. Tests and gates](8-tests-and-gates.md)
  - [8.1. Gates](8-tests-and-gates.md#81-gates)
  - [8.2. Design lint](8-tests-and-gates.md#82-design-lint)

Procedures and references:

- [Migration and landing](migration-and-landing.md): migration strategy, acceptance checklist, steps, critical path, landing strategy, breaking changes, status
- [Crate and module layout](reference/crate-layout.md): facade features, host service rules, module tables for every crate
- [Component tokens](reference/component-tokens.md): semantic colour tokens, appearance settings, themes, component placement, which components use which token, raw values still to convert
