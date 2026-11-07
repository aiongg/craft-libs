//! Built-in words of the tile grid family.

craft_ui::words! {
    /// The words of a tile and of the thumbnail slider.
    pub struct TileWords {
        /// A tile's accessible name: `{name}`, then `{details}`, the parts that apply joined with a comma.
        tile_name: template "tileGrid.name" = "{name}, {details}";
        picked: "tileGrid.picked" = "picked";
        rejected: "tileGrid.rejected" = "rejected";
        /// `{label}` is the app's name for the colour label.
        label: template "tileGrid.label" = "{label} label";
        offline: "tileGrid.offline" = "Offline";
        /// An offline tile's detail: `{offline}` is the word above, `{message}` the app's text.
        offline_with: template "tileGrid.offlineWith" = "{offline}: {message}";
        unreadable: "tileGrid.unreadable" = "Unreadable";
        failed: "tileGrid.failed" = "Preview failed";
        /// The stack badge: `{n}` tiles.
        stack: template "tileGrid.stack" = "×{n}";
        /// The name of a tile's check box: `{name}` is the tile's name.
        select: template "tileGrid.select" = "Select {name}";
        /// The slider's end buttons.
        smaller: "tileGrid.smaller" = "Smaller Thumbnails";
        larger: "tileGrid.larger" = "Larger Thumbnails";
    }
}
