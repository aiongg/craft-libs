//! Sample text that stories share with registered words. A story that shows a label equal to a
//! built-in word takes it from here, so it is looked up (and marked in the pseudo locale) like
//! the text of a host app's own labels.

crate::words! {
    /// The sample labels of stories whose English equals a registered word.
    #[non_exhaustive]
    pub struct StoryLabels {
        percent: "story.percent" = "{n}%";
        all: "story.all" = "All";
        amount: "story.amount" = "Amount";
        app_name: "story.appName" = "Studio";
        applies_at_next_launch: "story.appliesAtNextLaunch" = "Applies at next launch.";
        color_table: "story.colorTable" = "Color Table";
        default: "story.default" = "Default";
        export: "story.export" = "Export…";
        height: "story.height" = "Height";
        image: "story.image" = "Image";
        maximum: "story.maximum" = "Maximum";
        name: "story.name" = "Name";
        none: "story.none" = "None";
        opacity: "story.opacity" = "Opacity";
        photo: "story.photo" = "Photo";
        poster: "story.poster" = "Poster";
        play: "story.play" = "Play";
        preset: "story.preset" = "Preset";
        save: "story.save" = "Save";
        shadow: "story.shadow" = "Shadow";
        size: "story.size" = "Size";
        text: "story.text" = "Text";
        tools: "story.tools" = "Tools";
        width: "story.width" = "Width";
        working: "story.working" = "Working";
    }
}

impl StoryLabels {
    /// A whole percentage, as `{n}%` fills it.
    pub fn percent_text(&self, percent: f64) -> String {
        craft_i18n::fmt(&self.percent, &[("n", &format!("{percent:.0}"))])
    }
}
