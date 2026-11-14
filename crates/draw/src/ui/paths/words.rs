//! The words of the Paths panel ([`PathsWords`]).

craft_ui::words! {
    /// The words of [`PathsPanel`](super::PathsPanel), English by default (Photoshop's).
    pub struct PathsWords {
        /// The panel's accessible name.
        panel: "paths.panel" = "Paths";
        /// The default name of the work path.
        work_path: "paths.workPath" = "Work Path";
        /// The footer's Fill Path button.
        fill_path: "paths.fillPath" = "Fill path with foreground color";
        /// The footer's Stroke Path button.
        stroke_path: "paths.strokePath" = "Stroke path with brush";
        /// The footer's Load button.
        load_selection: "paths.loadSelection" = "Load path as a selection";
        /// The footer's Make Work Path button.
        make_work_path: "paths.makeWorkPath" = "Make work path from selection";
        /// The footer's New button.
        new_path: "paths.newPath" = "Create new path";
        /// The footer's Delete button.
        delete_path: "paths.deletePath" = "Delete current path";
    }
}
