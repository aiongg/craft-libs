//! Stories of the Brush Preset picker, its cards and chip (stub; task ST-BR1 writes them, RG2
//! registers them and adds `BrushPresetPicker`, `BrushPresetCards` and `BrushPresetChip` to the
//! components). Fake presets in three groups with tips and strokes drawn with the painter, so
//! every state shows without an engine; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/brush-preset-picker/open` | The picker open on 40 fake presets in three groups: part boxes, scale, search |
//! | `composites/brush-preset-picker/sampled-tip` | A sampled tip: no Hardness slider |
//! | `composites/brush-preset-picker/empty` | An empty library |
//! | `composites/brush-cards/parts` | The card in all seven part combinations at scales 0.15, 0.7, 1 and 2 |
//! | `controls/brush-preset-chip/states` | Closed, open, a four-digit size |
