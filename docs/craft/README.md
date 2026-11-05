# Craft framework

The Craft framework is the shared layer under every Craft app: the design system, shells, canvas core, host convention and feature families, supplied by `craft-libs` as one dependency. This set describes the design, how to install it, how the crates fit together, and how apps migrate. Engineers who build an app read sections 1 to 3. Engineers who build components or themes read sections 4 to 8.

## Contents

1. [Overview](1-overview.md)
   1. [What the Craft framework is](1-overview.md#11-what-the-craft-framework-is)
   2. [Framework and design system](1-overview.md#12-framework-and-design-system)
   3. [Principles](1-overview.md#13-principles)
2. [Installation](2-installation.md)
   1. [Run the component gallery](2-installation.md#21-run-the-component-gallery)
   2. [Depend on `craft` from an app](2-installation.md#22-depend-on-craft-from-an-app)
3. [Architecture](3-architecture.md)
   1. [Crate tiers](3-architecture.md#31-crate-tiers)
   2. [The `craft` facade](3-architecture.md#32-the-craft-facade)
   3. [Crate taxonomy](3-architecture.md#33-crate-taxonomy)
   4. [The host convention: services in, edits out](3-architecture.md#34-the-host-convention-services-in-edits-out)
   5. [One egui revision](3-architecture.md#35-one-egui-revision)
4. [Design system](4-design-system.md)
   1. [Component layers](4-design-system.md#41-component-layers)
   2. [Ownership](4-design-system.md#42-ownership)
   3. [Generic or family component](4-design-system.md#43-generic-or-family-component)
   4. [Patterns and shells](4-design-system.md#44-patterns-and-shells)
   5. [Per-app look and feel](4-design-system.md#45-per-app-look-and-feel)
   6. [Customization ladder](4-design-system.md#46-customization-ladder)
   7. [Themes](4-design-system.md#47-themes)
5. [Foundations](5-foundations.md)
   1. [Colour](5-foundations.md#51-colour)
   2. [Typography](5-foundations.md#52-typography)
   3. [Iconography](5-foundations.md#53-iconography)
   4. [Dimensions](5-foundations.md#54-dimensions)
   5. [Elevation](5-foundations.md#55-elevation)
   6. [Motion](5-foundations.md#56-motion)
   7. [Appearance and contrast](5-foundations.md#57-appearance-and-contrast)
6. [Components](6-components.md)
   1. [What every component provides](6-components.md#61-what-every-component-provides)
   2. [Index by layer](6-components.md#62-index-by-layer)
   3. [Family components](6-components.md#63-family-components)
7. [Accessibility and translation](7-accessibility-and-translation.md)
   1. [Required for every component](7-accessibility-and-translation.md#71-required-for-every-component)
   2. [Translation rules](7-accessibility-and-translation.md#72-translation-rules)
   3. [Goals beyond the required list](7-accessibility-and-translation.md#73-goals-beyond-the-required-list)
8. [Tests and gates](8-tests-and-gates.md)
   1. [Design lint](8-tests-and-gates.md#81-design-lint)

Procedures and references:

- [Migration and landing](migration-and-landing.md): migration strategy, acceptance checklist, steps, critical path, landing strategy, breaking changes, status
- [Crate and module layout](reference/crate-layout.md): module tables for every crate
- [Component tokens](reference/component-tokens.md): which components use which token, and the raw values still to convert
