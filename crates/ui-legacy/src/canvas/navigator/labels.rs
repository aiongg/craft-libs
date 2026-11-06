//! [`NavigatorLabels`]: every word the Navigator, its panel menu and Panel Options show.

crate::words! {
    /// The Navigator's words, English by default (Photoshop's). Pass a translated copy with
    /// [`Navigator::labels`](super::Navigator::labels); the dialog and the panel menu read the
    /// installed words. OK and Cancel are [`DialogLabels`](crate::dialog::DialogLabels)'.
    #[non_exhaustive]
    pub struct NavigatorLabels {
        /// The preview's accessible name (a `Group`, one Tab stop).
        name: "navigator.name" = "Navigator";
        /// The decorative preview image's name.
        preview: "navigator.preview" = "Document preview";
        /// The horizontal scroll bar node of the preview.
        horizontal: "navigator.horizontal" = "Visible area horizontal position";
        /// The vertical scroll bar node of the preview.
        vertical: "navigator.vertical" = "Visible area vertical position";
        /// The zoom field's and the slider's name.
        zoom: "navigator.zoom" = "Zoom";
        /// The slider's value text, `{n}` the zoom in percent as the field shows it.
        zoom_value: "navigator.zoomValue" = "{n} %";
        zoom_out: "navigator.zoomOut" = "Zoom Out";
        zoom_in: "navigator.zoomIn" = "Zoom In";
        /// The panel menu row that opens [`NavigatorOptionsDialog`](super::NavigatorOptionsDialog).
        panel_options: "navigator.panelOptions" = "Panel Options…";
        /// The dialog's title.
        options_title: "navigator.optionsTitle" = "Navigator Panel Options";
        /// The caption of the proxy colour list.
        proxy_color: "navigator.proxyColor" = "Color";
        /// [`ProxyColor::Default`](super::ProxyColor::Default): the theme's proxy colour.
        default_color: "navigator.defaultColor" = "Default";
        red: "navigator.red" = "Red";
        orange: "navigator.orange" = "Orange";
        yellow: "navigator.yellow" = "Yellow";
        green: "navigator.green" = "Green";
        blue: "navigator.blue" = "Blue";
        violet: "navigator.violet" = "Violet";
        gray: "navigator.gray" = "Gray";
        custom: "navigator.custom" = "Custom…";
        /// The colour picker's title for Custom… ([`ColorRequest::title`](crate::dialog::ColorRequest::title)).
        picker_title: "navigator.pickerTitle" = "Color Picker (Proxy Color)";
    }
}
