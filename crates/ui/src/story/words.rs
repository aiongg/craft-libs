//! Sample text that stories share with registered words. A story that shows a label equal to a
//! built-in word takes it from here, so it is looked up (and marked in the pseudo locale) like
//! the text of a host app's own labels.

crate::words! {
    /// The sample labels of stories whose English equals a registered word.
    #[non_exhaustive]
    pub struct StoryLabels {
        percent: "story.percent" = "{n}%";
        all: "story.all" = "All";
        about_blurb: "story.aboutBlurb" = "Studio, a native image editor.";
        about_community: "story.aboutCommunity" = "Join the community";
        about_source: "story.aboutSource" = "Source code";
        about_website: "story.aboutWebsite" = "Website";
        document: "story.document" = "Document";
        macro_brighten: "story.macroBrighten" = "Brighten";
        macro_brighten_info: "story.macroBrightenInfo" = "Raises the exposure by one stop.";
        macro_name: "story.macroName" = "Macro name:";
        macro_resize: "story.macroResize" = "Resize for Web";
        macro_resize_info: "story.macroResizeInfo" = "Scales to 1600 px wide and exports a JPEG.";
        macro_sharpen: "story.macroSharpen" = "Sharpen";
        macro_sharpen_info: "story.macroSharpenInfo" = "Unsharp mask, 80 percent, radius 1.2.";
        macros: "story.macros" = "Macros";
        macros_empty: "story.macrosEmpty" = "No macros recorded yet.";
        macros_note: "story.macrosNote" = "Recorded macros run on the active document.";
        picked: "story.picked" = "Picked: {action}";
        result_ok: "story.resultOk" = "OK, {value}";
        last_link: "story.lastLink" = "Link opened: {url}";
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
        end: "story.end" = "End";
        middle: "story.middle" = "Middle";
        start: "story.start" = "Start";
        elide_path: "story.elidePath" = "Path";
        elide_rtl: "story.elideRtl" = "RTL";
    }
}

impl StoryLabels {
    /// A whole percentage, as `{n}%` fills it.
    pub fn percent_text(&self, percent: f64) -> String {
        craft_i18n::fmt(&self.percent, &[("n", &format!("{percent:.0}"))])
    }
}
