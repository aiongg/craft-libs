# craft-browse

Browsing for Craft apps: folders and volumes, linked files, file thumbnails, and the widgets that show them. The app supplies its items through small read-only traits and gets events back; every listing, status check, search and decode runs as a job on `craft_host::queue::JobQueue`.

| Module | What it gives the app |
| --- | --- |
| `places` | The roots of a folder tree (standard folders, volumes, favourites, recents, the folder being browsed), the hidden and kept lists as one settings section (`PlacesSettings`), and `FolderCache`, lazy folder listings refreshed when `FileWatch` reports a change. Folder access goes through `Directories` (`SystemDirectories` on desktop, a fake in stories). |
| `links` | `LinkMonitor`: link statuses kept current as files change, each change reported once. The link rules themselves are `craft_docpath::links`. |
| `thumbs` | File thumbnails: the app implements `ThumbDecoder` (and optionally `ThumbStore`); with `ui`, `FileThumbSource` is a `ThumbSource` over files for `craft_ui::canvas::textures::Thumbnails`. |
| `ui::grid` | `TileGrid`: a virtualised thumbnail grid with groups, badges, rating, selection, check mode, reorder, marquee, hover scrub, keys and drag, over the app's `TileSource`. |
| `ui::bin` | `MediaBin`: the project or media panel with List, Icon and Freeform views, search, sort, rename, bin drops and a footer, over the app's `BinModel`. |
| `ui::places` | `PlacesTree`: the folder tree with lazy children, reveal of the browsed folder and the keep, hide and reveal menu. |
| `ui::compare` | `CompareTile`: one fitted image with zoom and pan shared across tiles and a caption strip; the two-pane and survey arrangements. |
| `ui::relink` | `LinkList` (the Links panel) and `RelinkDialog` (find missing files, match options, folder search, candidates). |

`places`, `links` and `thumbs` work without egui; the `ui` feature adds the widgets, their words (`craft_browse::ui::WORDS`, `craft_browse::ui::COUNTS`) and root re-exports of each widget, its labels and its style.
