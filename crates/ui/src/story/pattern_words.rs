//! Sample text of the pattern, shell and foundation stories whose English equals a registered
//! word. A story takes it from here, so it is looked up (and marked in the pseudo locale) like
//! the text of a host app's own labels.

crate::words! {
    /// The sample labels of the pattern stories, English by default.
    #[non_exhaustive]
    pub struct PatternStoryLabels {
        advanced: "patternStory.advanced" = "Advanced";
        brush: "patternStory.brush" = "Brush";
        copy: "patternStory.copy" = "Copy";
        crop: "patternStory.crop" = "Crop";
        delete: "patternStory.delete" = "Delete";
        description: "patternStory.description" = "Description";
        edit: "patternStory.edit" = "Edit";
        eraser: "patternStory.eraser" = "Eraser";
        general: "patternStory.general" = "General";
        home: "patternStory.home" = "Home";
        more: "patternStory.more" = "More";
        move_tool: "patternStory.moveTool" = "Move";
        new: "patternStory.new" = "New";
        nothing_chosen: "patternStory.nothingChosen" = "Nothing chosen";
        search: "patternStory.search" = "Search";
        show: "patternStory.show" = "Show";
        small: "patternStory.small" = "Small";
        zoom: "patternStory.zoom" = "Zoom";
    }
}
