//! Built-in words of the list family.

craft_ui::words! {
    /// The words of the list family's built-in states and parts.
    pub struct ListWords {
        no_findings: "findingsList.noFindings" = "No findings";
        clipped: "listRow.clipped" = "Clipped to the layer below";
        thumb_none: "thumbSize.none" = "None";
        thumb_small: "thumbSize.small" = "Small";
        thumb_medium: "thumbSize.medium" = "Medium";
        thumb_large: "thumbSize.large" = "Large";
        toggle_visibility: "toggleCell.visibility" = "Visibility {row}";
        toggle_lock: "toggleCell.lock" = "Lock {row}";
        expand: "listRow.expand" = "Expand {row}";
        collapse: "listRow.collapse" = "Collapse {row}";
    }
}
