# craft-imaging

Raster image editing for Craft apps.

`craft_imaging::scopes` counts without egui and builds on every target: the clipping rule and clipping masks, the vectorscope grid, hue and saturation, and histogram statistics. The app does the colour management and the histogram count.

With the `ui` feature, `craft_imaging::ui` has:

- the `WarpGrid` (a warp's mesh on a canvas, with `WarpMesh`, `WarpEdit` and the press and release helpers);
- the `LevelsEditor` (a channel's `Levels` over its histogram);
- the `ChannelsPanel` (thumbnails, eyes, selection, rename in place with `start_rename`, drag, the footer (`ChannelsEvent::Footer` and `FooterDrop`) and the menus over plain `ChannelRow`s; `ChannelsPanel::style(&UiTokens)` gives the look);
- the `ScopePanel` (histogram, statistics, tone zones and vectorscope) and the `ClippingOverlay` for a preview;
- their stories and words (`craft_imaging::ui::stories()`, `craft_imaging::ui::WORDS`).

The panels report what the user did as events and hold no document. Thumbnails load as jobs on `craft_host::queue::JobQueue`.

The curves editor and the histogram stay in `craft-ui`.
