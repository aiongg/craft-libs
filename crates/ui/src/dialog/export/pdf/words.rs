//! The words of the Save PDF dialog, and the wording of craft-pdf's checks and warnings.

use craft_pdf::settings::{PdfSettings, SettingsError, SettingsWarning};

crate::words! {
    /// The words of Save PDF, English by default.
    #[non_exhaustive]
    pub struct PdfExportLabels {
        title: "pdfExport.title" = "Save PDF";
        save: "pdfExport.save" = "Save PDF";
        preset: "pdfExport.preset" = "Preset";
        save_preset: "pdfExport.savePreset" = "Save Preset…";
        save_preset_hint: "pdfExport.savePresetHint" = "Save these settings as a preset";
        save_as_preset: "pdfExport.saveAsPreset" = "Save as preset";
        name_save: "pdfExport.nameSave" = "Save";
        name_cancel: "pdfExport.nameCancel" = "Cancel";
        /// The name suggested for a new preset.
        new_preset_name: "pdfExport.newPresetName" = "New PDF Preset";
        standard: "pdfExport.standard" = "Standard";
        compatibility: "pdfExport.compatibility" = "Compatibility";
        sections: "pdfExport.sections" = "Sections";
        section_general: "pdfExport.sectionGeneral" = "General";
        section_compression: "pdfExport.sectionCompression" = "Compression";
        section_marks: "pdfExport.sectionMarks" = "Marks and Bleeds";
        section_output: "pdfExport.sectionOutput" = "Output";
        section_advanced: "pdfExport.sectionAdvanced" = "Advanced";
        section_security: "pdfExport.sectionSecurity" = "Security";
        section_summary: "pdfExport.sectionSummary" = "Summary";
        options: "pdfExport.options" = "Options";
        preserve_editing: "pdfExport.preserveEditing" = "Preserve editing capabilities";
        thumbnails: "pdfExport.thumbnails" = "Embed page thumbnails";
        fast_web_view: "pdfExport.fastWebView" = "Optimize for fast web view";
        view_after_saving: "pdfExport.viewAfterSaving" = "View PDF after saving";
        create_layers: "pdfExport.createLayers" = "Create PDF layers from layers and sublayers";
        include_non_printing: "pdfExport.includeNonPrinting" = "Include non-printing layers";
        artboards: "pdfExport.artboards" = "Artboards";
        all: "pdfExport.all" = "All";
        range: "pdfExport.range" = "Range";
        range_hint: "pdfExport.rangeHint" = "1-3, 5";
        color_images: "pdfExport.colorImages" = "Color images";
        gray_images: "pdfExport.grayImages" = "Grayscale images";
        mono_images: "pdfExport.monoImages" = "Monochrome images";
        downsampling: "pdfExport.downsampling" = "Downsampling";
        to: "pdfExport.to" = "to";
        above: "pdfExport.above" = "above";
        compression: "pdfExport.compression" = "Compression";
        quality: "pdfExport.quality" = "Quality";
        compress_text: "pdfExport.compressText" = "Compress text and line art";
        /// `{n}`: a resolution.
        ppi: "pdfExport.ppi" = "{n} ppi";
        /// `{n}`: a length in points.
        points: "pdfExport.points" = "{n} pt";
        percent: "pdfExport.percent" = "{n}%";
        marks: "pdfExport.marks" = "Marks";
        all_marks: "pdfExport.allMarks" = "All printer's marks";
        trim_marks: "pdfExport.trimMarks" = "Trim marks";
        registration_marks: "pdfExport.registrationMarks" = "Registration marks";
        color_bars: "pdfExport.colorBars" = "Color bars";
        page_information: "pdfExport.pageInformation" = "Page information";
        mark_type: "pdfExport.markType" = "Printer mark type";
        mark_weight: "pdfExport.markWeight" = "Trim mark weight";
        mark_offset: "pdfExport.markOffset" = "Offset";
        bleeds: "pdfExport.bleeds" = "Bleeds";
        use_document_bleed: "pdfExport.useDocumentBleed" = "Use document bleed settings";
        top: "pdfExport.top" = "Top";
        bottom: "pdfExport.bottom" = "Bottom";
        left: "pdfExport.left" = "Left";
        right: "pdfExport.right" = "Right";
        color: "pdfExport.color" = "Color";
        color_conversion: "pdfExport.colorConversion" = "Color conversion";
        destination: "pdfExport.destination" = "Destination";
        document_profile: "pdfExport.documentProfile" = "Document profile";
        profile_inclusion: "pdfExport.profileInclusion" = "Profile inclusion";
        note_cmyk_only: "pdfExport.noteCmykOnly" = "PDF/X-1a files are CMYK: every colour is converted to the destination (blank: the output intent's CMYK profile), untagged.";
        note_tagged: "pdfExport.noteTagged" = "Files of this standard tag their colours with ICC profiles.";
        output_intent: "pdfExport.outputIntent" = "Output Intent";
        intent_profile: "pdfExport.intentProfile" = "Output intent profile";
        no_profile: "pdfExport.noProfile" = "None";
        cmyk_in_effect: "pdfExport.cmykInEffect" = "CMYK profile in effect";
        output_condition: "pdfExport.outputCondition" = "Output condition";
        condition_identifier: "pdfExport.conditionIdentifier" = "Condition identifier";
        registry_name: "pdfExport.registryName" = "Registry name";
        trapped: "pdfExport.trapped" = "Mark as trapped";
        note_pdfa_intent: "pdfExport.notePdfaIntent" = "PDF/A files carry their own output intent.";
        fonts: "pdfExport.fonts" = "Fonts";
        subset_below: "pdfExport.subsetBelow" = "Subset fonts below";
        of_characters: "pdfExport.ofCharacters" = "of characters used";
        outline_text: "pdfExport.outlineText" = "Convert text to outlines";
        overprint_flattener: "pdfExport.overprintFlattener" = "Overprint and Transparency Flattener";
        overprint: "pdfExport.overprint" = "Overprint";
        flattener_preset: "pdfExport.flattenerPreset" = "Flattener preset";
        note_flatten: "pdfExport.noteFlatten" = "PDF 1.3 files have no transparency: the preset flattens it.";
        flatten_high: "pdfExport.flattenHigh" = "High Resolution";
        flatten_medium: "pdfExport.flattenMedium" = "Medium Resolution";
        flatten_low: "pdfExport.flattenLow" = "Low Resolution";
        open_password_heading: "pdfExport.openPasswordHeading" = "Document open password";
        require_open_password: "pdfExport.requireOpenPassword" = "Require a password to open the document";
        open_password: "pdfExport.openPassword" = "Open password";
        permissions: "pdfExport.permissions" = "Permissions";
        restrict_permissions: "pdfExport.restrictPermissions" = "Restrict printing, editing and other tasks";
        permissions_password: "pdfExport.permissionsPassword" = "Permissions password";
        printing_allowed: "pdfExport.printingAllowed" = "Printing allowed";
        changes_allowed: "pdfExport.changesAllowed" = "Changes allowed";
        allow_copy: "pdfExport.allowCopy" = "Enable copying of text, images and other content";
        allow_reader: "pdfExport.allowReader" = "Enable text access for screen readers";
        plaintext_metadata: "pdfExport.plaintextMetadata" = "Enable plaintext metadata";
        note_standard_password: "pdfExport.noteStandardPassword" = "Files of a PDF standard can't be password-protected.";
        /// `{name}`: the encryption algorithm.
        encryption_note: "pdfExport.encryptionNote" = "Encryption: {name} (set by Compatibility)";
        encryption: "pdfExport.encryption" = "Encryption";
        summary_warnings: "pdfExport.summaryWarnings" = "Warnings";
        no_warnings: "pdfExport.noWarnings" = "None.";
        no_encryption: "pdfExport.noEncryption" = "None";
        /// `{app}`: the application's name.
        preset_default: "pdfExport.presetDefault" = "{app} Default";
        preset_high_quality: "pdfExport.presetHighQuality" = "High Quality Print";
        preset_press: "pdfExport.presetPress" = "Press Quality";
        preset_smallest: "pdfExport.presetSmallest" = "Smallest File Size";
        no_downsampling: "pdfExport.noDownsampling" = "No downsampling";
        average: "pdfExport.average" = "Average";
        subsample: "pdfExport.subsample" = "Subsample";
        bicubic: "pdfExport.bicubic" = "Bicubic";
        none: "pdfExport.none" = "None";
        automatic: "pdfExport.automatic" = "Automatic";
        run_length: "pdfExport.runLength" = "Run Length";
        minimum: "pdfExport.minimum" = "Minimum";
        low: "pdfExport.low" = "Low";
        medium: "pdfExport.medium" = "Medium";
        high: "pdfExport.high" = "High";
        maximum: "pdfExport.maximum" = "Maximum";
        roman: "pdfExport.roman" = "Roman";
        japanese: "pdfExport.japanese" = "Japanese";
        no_conversion: "pdfExport.noConversion" = "No conversion";
        convert_destination: "pdfExport.convertDestination" = "Convert to destination";
        convert_preserve: "pdfExport.convertPreserve" = "Convert to destination (preserve numbers)";
        no_profiles: "pdfExport.noProfiles" = "Don't include profiles";
        all_profiles: "pdfExport.allProfiles" = "Include all profiles";
        destination_profile: "pdfExport.destinationProfile" = "Include destination profile";
        tagged_profiles: "pdfExport.taggedProfiles" = "Include tagged source profiles";
        preserve: "pdfExport.preserve" = "Preserve";
        discard: "pdfExport.discard" = "Discard";
        printing_low: "pdfExport.printingLow" = "Low resolution (150 ppi)";
        printing_high: "pdfExport.printingHigh" = "High resolution";
        changes_pages: "pdfExport.changesPages" = "Inserting, deleting and rotating pages";
        changes_forms: "pdfExport.changesForms" = "Filling in form fields and signing";
        changes_comments: "pdfExport.changesComments" = "Commenting, filling in form fields and signing";
        changes_any: "pdfExport.changesAny" = "Any except extracting pages";
        /// `{standard}`, `{version}`.
        check_standard_version: "pdfExport.checkStandardVersion" = "{standard} files can't be {version}";
        /// `{standard}`.
        check_standard_editing: "pdfExport.checkStandardEditing" = "{standard} files can't carry editing data: turn off Preserve editing capabilities";
        /// `{standard}`.
        check_standard_layers: "pdfExport.checkStandardLayers" = "{standard} files can't have PDF layers: turn off Create PDF layers";
        /// `{standard}`.
        check_standard_password: "pdfExport.checkStandardPassword" = "{standard} files can't be password-protected: remove the passwords or choose no standard";
        check_passwords_equal: "pdfExport.checkPasswordsEqual" = "The open and permissions passwords must differ";
        /// `{version}`.
        check_low_printing: "pdfExport.checkLowPrinting" = "{version} can't limit printing to low resolution: choose PDF 1.4 or later, or allow high-resolution printing";
        warn_layers_version: "pdfExport.warnLayersVersion" = "PDF layers need PDF 1.5 or later: every layer is plain page content";
        warn_font_subset: "pdfExport.warnFontSubset" = "Fonts are embedded as subsets of the characters used: a subset threshold below 100% is not applied";
        warn_permissions_password: "pdfExport.warnPermissionsPassword" = "Permissions need a password: without one they are not applied";
        /// `{version}`.
        warn_plaintext_version: "pdfExport.warnPlaintextVersion" = "{version} encryption covers the metadata too: plaintext metadata needs PDF 1.5 or later";
        /// `{version}`.
        warn_weak_key: "pdfExport.warnWeakKey" = "{version} encrypts with a 40-bit key, which can be searched in minutes: choose PDF 1.6 or later for stronger protection";
        warn_pdfa_intent: "pdfExport.warnPdfaIntent" = "PDF/A files carry their own output intent: the output intent and Trapped entries are left out";
        /// `{name}`.
        warn_intent_not_embedded: "pdfExport.warnIntentNotEmbedded" = "No profile is called “{name}”: the output intent names it without embedding it";
        warn_condition_needs_intent: "pdfExport.warnConditionNeedsIntent" = "An output intent needs a profile or a condition identifier: the output condition and registry are left out";
    }
}

impl PdfExportLabels {
    /// The label of choice `id` of the setting `kind` (`"downsample"`, `"standard"`, ...), or
    /// `fallback`, craft-pdf's English label, for names no translator changes (`PDF 1.7`, `ZIP`).
    pub(super) fn choice(&self, kind: &str, id: &str, fallback: &str) -> String {
        let word = match (kind, id) {
            ("standard", "none") | ("monoCodec", "none") | ("imageCodec", "none") | ("printing", "none") | ("changes", "none") => &self.none,
            ("downsample", "none") => &self.no_downsampling,
            ("downsample", "average") => &self.average,
            ("downsample", "subsample") => &self.subsample,
            ("downsample", "bicubic") => &self.bicubic,
            ("imageCodec", "auto") => &self.automatic,
            ("monoCodec", "runLength") => &self.run_length,
            ("jpegQuality", "minimum") => &self.minimum,
            ("jpegQuality", "low") => &self.low,
            ("jpegQuality", "medium") => &self.medium,
            ("jpegQuality", "high") => &self.high,
            ("jpegQuality", "maximum") => &self.maximum,
            ("markKind", "roman") => &self.roman,
            ("markKind", "japanese") => &self.japanese,
            ("colorConversion", "none") => &self.no_conversion,
            ("colorConversion", "destination") => &self.convert_destination,
            ("colorConversion", "preserveNumbers") => &self.convert_preserve,
            ("profileInclusion", "none") => &self.no_profiles,
            ("profileInclusion", "all") => &self.all_profiles,
            ("profileInclusion", "destination") => &self.destination_profile,
            ("profileInclusion", "taggedSource") => &self.tagged_profiles,
            ("overprint", "preserve") => &self.preserve,
            ("overprint", "discard") => &self.discard,
            ("printing", "low") => &self.printing_low,
            ("printing", "high") => &self.printing_high,
            ("changes", "pages") => &self.changes_pages,
            ("changes", "forms") => &self.changes_forms,
            ("changes", "comments") => &self.changes_comments,
            ("changes", "any") => &self.changes_any,
            _ => return fallback.to_owned(),
        };
        word.clone()
    }

    /// The name a built-in preset is shown by; other names (saved presets, the PDF/X ones) stay.
    pub(super) fn preset_name(&self, name: &str, app: &str) -> String {
        if name == craft_pdf::settings::default_preset_name(app) {
            return craft_i18n::fmt(&self.preset_default, &[("app", app)]);
        }
        match name {
            "High Quality Print" => self.preset_high_quality.clone(),
            "Press Quality" => self.preset_press.clone(),
            "Smallest File Size" => self.preset_smallest.clone(),
            _ => name.to_owned(),
        }
    }

    /// The words of a refused setting, craft-pdf's English message where no word exists.
    pub(super) fn error_text(&self, error: &SettingsError, settings: &PdfSettings) -> String {
        let standard = settings.standard.label();
        let version = settings.compatibility.label();
        let fill = |word: &str| craft_i18n::fmt(word, &[("standard", standard), ("version", version)]);
        match error.id {
            "standardVersion" => fill(&self.check_standard_version),
            "standardEditing" => fill(&self.check_standard_editing),
            "standardLayers" => fill(&self.check_standard_layers),
            "standardPassword" => fill(&self.check_standard_password),
            "lowPrintingVersion" => fill(&self.check_low_printing),
            "passwordsEqual" => self.check_passwords_equal.clone(),
            _ => error.message.clone(),
        }
    }

    /// The words of a warning, craft-pdf's English message where no word exists.
    pub(super) fn warning_text(&self, warning: &SettingsWarning, settings: &PdfSettings) -> String {
        let version = settings.compatibility.label();
        match warning.id {
            "layersVersion" => self.warn_layers_version.clone(),
            "fontSubsetThreshold" => self.warn_font_subset.clone(),
            "permissionsNeedPassword" => self.warn_permissions_password.clone(),
            "plaintextMetadataVersion" => craft_i18n::fmt(&self.warn_plaintext_version, &[("version", version)]),
            "weakKey" => craft_i18n::fmt(&self.warn_weak_key, &[("version", version)]),
            "pdfaOutputIntent" => self.warn_pdfa_intent.clone(),
            "intentNotEmbedded" => craft_i18n::fmt(&self.warn_intent_not_embedded, &[("name", settings.output.output_intent.trim())]),
            "conditionNeedsIntent" => self.warn_condition_needs_intent.clone(),
            _ => warning.message.clone(),
        }
    }
}
