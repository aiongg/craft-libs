//! [`FullWindowLabels`]: the words the full-window regions write themselves.

crate::words! {
    /// The words of the full-window regions (`fullWindow.*`), English by default. The workspace's
    /// own names (its title, tool names, panel headers) are the workspace's words.
    #[non_exhaustive]
    pub struct FullWindowLabels {
        /// The title strip with a subject and the zoom: "Liquify (Layer 1, 66%)".
        title_zoom: "fullWindow.titleZoom" = "{name} ({subject}, {zoom})";
        /// The title strip with a subject and no zoom: "Adaptive Wide Angle (Layer 1)".
        title_subject: "fullWindow.titleSubject" = "{name} ({subject})";
        /// The title strip with the zoom and no subject: "Liquify (66%)".
        title_zoom_only: "fullWindow.titleZoomOnly" = "{name} ({zoom})";
        /// The canvas region's accessible name.
        canvas: "fullWindow.canvas" = "Image";
        /// A side column's accessible name when it has no header.
        side: "fullWindow.side" = "Settings";
        /// The zoom bar's Zoom Out button.
        zoom_out: "fullWindow.zoomOut" = "Zoom Out";
        /// The zoom bar's Zoom In button.
        zoom_in: "fullWindow.zoomIn" = "Zoom In";
        /// The zoom bar's select, for a screen reader.
        zoom: "fullWindow.zoom" = "Zoom";
        /// The zoom select's fit entry.
        fit_in_view: "fullWindow.fitInView" = "Fit in View";
        /// The polite live label while a preview is being computed.
        updating: "fullWindow.updatingPreview" = "Updating preview";
        /// The error line under a preview that failed.
        failed: "fullWindow.previewFailed" = "Preview failed: {error}";
    }
}
