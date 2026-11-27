//! Installing a stack: its families and sources as the [`FontStack`] the text engine sets text in, offered through the
//! [`FontHost`], and the install again once the OS interface face resolves.

use std::sync::Arc;

use craft_ui_rt::{FaceSource, FallbackAsk, FontStack};

use super::{Stack, SystemFonts};
use crate::fonts::faces::{Installed, has_char, on_baseline};
use crate::fonts::served::loader;
use crate::fonts::system::installed_system_font_source;
use crate::fonts::system::ui_face::{self, SYSTEM_UI_REGULAR, SYSTEM_UI_SEMIBOLD, SystemUiFace};
use crate::fonts::{FontHost, FontRef, RegularWeight, system_font_source};

impl Stack {
    /// Offer the stack to the text engine through `host`; the engine sets text in it from the next frame on. Call it
    /// again when the UI language changes.
    ///
    /// The stack's system fonts (the `system_fonts` feature on desktop, or [`super::UiFonts::system_provider`]) are a
    /// source asked after the stack's families, at each family's weight; [`SystemFonts::Off`] asks none. The default
    /// system source doesn't block the UI thread: before the installed fonts' index is built it finds nothing, and the
    /// host offers the stack again once it is.
    ///
    /// A stack with [`super::UiFonts::served_faces`] and a [`super::UiFonts::fetcher`] also fetches them (see there);
    /// the faces already fetched for an earlier stack are added at once.
    ///
    /// A stack built with [`craft_theme::InterfaceFont::System`] before the system face resolved is installed without
    /// it, then again with it once it resolves, unless another stack is installed through `host` first (see
    /// [`super::UiFonts::interface_font`]).
    pub fn install(&self, host: &FontHost) {
        if self.system_ui_pending
            && let Some(face) = ui_face::resolved()
        {
            let mut stack = self.clone();
            stack.provide_system_ui(face);
            return stack.install(host);
        }
        if !self.served.is_empty()
            && let Some(fetcher) = &self.fetcher
        {
            return loader::install(host, self.clone(), Arc::clone(fetcher));
        }
        self.offer_faces(host, None);
    }

    /// [`Self::install`] without fetching; `miss` is asked for characters no face has.
    pub(in crate::fonts) fn offer_faces(&self, host: &FontHost, miss: Option<Arc<dyn FaceSource>>) {
        host.set_waiting(self.system_ui_pending.then(|| self.clone()));
        let (system, start_index) = self.host_lookup();
        let on_demand = self.on_demand.clone();
        host.offer(self.font_stack(on_demand.into_iter().chain(system).collect(), miss));
        if let Some(start) = start_index {
            start(host);
        }
        if self.system_ui_pending {
            ui_face::install_on_resolve(host);
        }
    }

    /// The stack as the engine takes it: the families' faces, then `first` (the on-demand loader, the system fonts) and
    /// the faces asked after them (then `miss`).
    fn font_stack(&self, first: Vec<Arc<dyn FaceSource>>, miss: Option<Arc<dyn FaceSource>>) -> FontStack {
        let installed = Installed::default();
        installed.set(self.after_system.clone(), miss);
        let mut fallback = first;
        fallback.push(Arc::new(installed));
        FontStack { families: self.checked_definitions().stack_families(), fallback }
    }

    /// Add the system face's data to its slots ([`SYSTEM_UI_REGULAR`], [`SYSTEM_UI_SEMIBOLD`]) and
    /// fit the CJK faces into its rows, or with `None` (no face resolves) drop the empty slots.
    pub(in crate::fonts) fn provide_system_ui(&mut self, face: Option<&SystemUiFace>) {
        self.system_ui_pending = false;
        for (name, data) in [(SYSTEM_UI_REGULAR, face.map(|f| &f.regular)), (SYSTEM_UI_SEMIBOLD, face.map(|f| &f.semibold))] {
            match data {
                Some(data) => {
                    if self.definitions.families.values().any(|chain| chain.iter().any(|n| n == name)) {
                        self.definitions.font_data.insert(name.to_owned(), data.clone().named(name));
                    }
                }
                None if !self.definitions.font_data.contains_key(name) => {
                    for chain in self.definitions.families.values_mut() {
                        chain.retain(|n| n != name);
                    }
                }
                None => {}
            }
        }
        if face.is_some() {
            let cjk_faces = self.cjk_faces.clone();
            self.fit_into_rows(&cjk_faces);
        }
    }

    /// A glyph probe over the stack, for coverage checks. `with_system` adds the system fonts, waiting for their index.
    pub(in crate::fonts) fn probe(&self, with_system: bool) -> GlyphProbe {
        GlyphProbe(self.font_stack(self.system_lookup(with_system).into_iter().collect(), None))
    }

    /// The system fonts asked after the stack's families, `None` for none ([`SystemFonts::Off`], or no source).
    fn system_lookup(&self, with_system: bool) -> Option<Arc<dyn FaceSource>> {
        if !with_system || self.system == SystemFonts::Off {
            return None;
        }
        let system = self.system_provider.clone().or_else(|| system_font_source(&self.ui_language))?;
        Some(self.regular_weight(system))
    }

    /// [`Self::system_lookup`] for [`Self::install`]. The default system source doesn't block the UI thread; the
    /// returned function starts its index and has the host offer the stack again when it is built.
    fn host_lookup(&self) -> (Option<Arc<dyn FaceSource>>, Option<fn(&FontHost)>) {
        if self.system_provider.is_none()
            && self.system != SystemFonts::Off
            && let Some((source, start)) = installed_system_font_source(&self.ui_language)
        {
            return (Some(self.regular_weight(source)), Some(start));
        }
        (self.system_lookup(true), None)
    }

    /// `source` at each family's weight, its text faces on the row's baseline.
    fn regular_weight(&self, source: Arc<dyn FaceSource>) -> Arc<dyn FaceSource> {
        let source = on_baseline(source);
        Arc::new(self.weights.iter().fold(RegularWeight::new(source), |p, (family, weight): &(FontRef, u16)| p.with_weight(family.clone(), *weight)))
    }
}

/// Answers whether a stack draws a character, without an engine: for coverage checks.
pub(crate) struct GlyphProbe(FontStack);

impl GlyphProbe {
    /// Whether `family`'s faces, or a face a source finds for it, have a glyph for `c`.
    pub(crate) fn has(&self, family: &FontRef, c: char) -> bool {
        let face = family.face();
        let own = self.0.family(face).or_else(|| self.0.family(FontRef::proportional().face())).unwrap_or_default();
        if own.iter().any(|data| has_char(data, c)) || self.0.source_faces(face).iter().any(|data| has_char(data, c)) {
            return true;
        }
        let mut utf8 = [0; 4];
        self.0.find_fallback(&FallbackAsk { family: face, cluster: c.encode_utf8(&mut utf8) }).is_some_and(|data| has_char(&data, c))
    }
}
