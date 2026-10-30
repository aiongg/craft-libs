# Icon sources

Each folder holds only the files `icons.toml` lists, unchanged from the pinned source (exception: the drawings named below the table), and the set's licence as `LICENSE`. To add an icon, copy its file from the same commit.

| Folder | Set | Licence | Source (tag, commit) | Files taken from |
|---|---|---|---|---|
| `lucide/` | [Lucide](https://lucide.dev) | ISC | [lucide-icons/lucide](https://github.com/lucide-icons/lucide) `1.63.0`, `2ca268e74888d3506b3319fb8f94aaaaf70ceca9` | `icons/` |
| `phosphor/` | [Phosphor](https://phosphoricons.com), regular weight | MIT | [phosphor-icons/core](https://github.com/phosphor-icons/core) `v2.0.8`, `d42782b2abe747d904b971ccab48b182a1455f86` | `assets/regular/` |
| `carbon/` | [IBM Carbon](https://carbondesignsystem.com/elements/icons/library/) | Apache-2.0 | [carbon-design-system/carbon](https://github.com/carbon-design-system/carbon) `v11.119.0` (`@carbon/icons` 11.91.0), `571cd549545dd156e11f06fd046eb14efa6a61c3` | `packages/icons/src/svg/32/` |
| `tabler/` | [Tabler Icons](https://tabler.io/icons), outline | MIT | [tabler/tabler-icons](https://github.com/tabler/tabler-icons) `v3.50.0`, `6904efd245391ad396df17654738f52f6da8c40e` | `icons/outline/` |

Carbon's licence file is the repository's root `LICENSE` (the icons package has none of its own).

Every set folder's files come from the npm package of the version in the table (`@phosphor-icons/core` 2.0.8, `lucide-static` 1.63.0, `@carbon/icons` 11.91.0, `@tabler/icons` 3.50.0), the same releases as the tags above. Phosphor files are all `regular` weight.

Phosphor has no glyph for `bandage-sparkle`, `brush-replace`, `clip-below`, `clip-release`, `convert-point`, `direct-select`, `eraser-background`, `eraser-magic` and `font-similar`. Their files in `phosphor/` are our own drawings, in Phosphor's regular weight (16-unit stroke on the 256 view box, round caps and joins), after PhotoCraft's glyphs.
`phosphor/` also holds drawings made for this library in Phosphor's style (256 view box, 16-unit round-capped strokes): `distribute-*`, `symmetry-*` and `painting-symmetry`. The symmetry glyphs follow PhotoCraft's own axis icons.
`phosphor/` also holds drawings made for the suite in Phosphor's regular style (256 view box, 16-unit round-capped stroke), which have no counterpart in the pinned release: `lasso`, `lasso-magnetic`, `lasso-select`, `layer-mask`, `lollipop`, `marquee-column`, `marquee-row`, `pen-add-anchor`, `pen-curvature`, `pen-delete-anchor`, `pen-freeform`, `quick-selection` (the pen nib and the quick-selection brush follow Lucide's `pen-tool` and `paintbrush`, ISC). `panel-right` is Phosphor's `sidebar-simple` mirrored.
