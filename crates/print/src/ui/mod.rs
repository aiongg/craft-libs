//! Printer selection and asynchronous job feedback, independent of any spooler or document.
//! Hosts provide workers; applications own PDF generation, commands and translated wording.
#![allow(deprecated, reason = "the printer and duplex choices use the deprecated combo box until they move to controls::Select")]

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A printer the spooler offers.
pub struct Printer {
    /// The spooler's destination: what [`Settings::printer`] and jobs name.
    pub name: String,
    /// What the dialog shows; empty shows `name`. CUPS, for one, describes the queue
    /// `_192_0_2_15` as `192.0.2.15`, the name the user gave the printer.
    pub label: String,
    /// Whether it is the system's default printer.
    pub default: bool,
}

/// The text the dialog shows for each of `printers`, in order: its label, or its name when it has
/// none. Text two printers would share is followed by the name in brackets, so neither hides the
/// other; when even that text is shared (another printer's label is `Office (Office_1)`), those
/// printers show their names, which the spooler keeps distinct.
pub fn shown_names(printers: &[Printer]) -> Vec<String> {
    let base: Vec<&str> = printers.iter().map(|p| if p.label.trim().is_empty() { p.name.as_str() } else { p.label.as_str() }).collect();
    let mut base_count: HashMap<&str, usize> = HashMap::with_capacity(base.len());
    for text in &base {
        *base_count.entry(text).or_default() += 1;
    }
    let mut shown: Vec<String> = printers
        .iter()
        .zip(&base)
        .map(|(printer, text)| {
            let shared = base_count.get(text).is_some_and(|n| *n > 1);
            if shared && *text != printer.name { format!("{text} ({})", printer.name) } else { (*text).to_owned() }
        })
        .collect();
    // Which printers show each text. A shared text turns every printer showing it into its name;
    // a printer that shows its name never changes again, so each moves at most once and the
    // whole cascade stays linear in the number of printers.
    let mut by_text: HashMap<String, Vec<usize>> = HashMap::with_capacity(shown.len());
    for (i, text) in shown.iter().enumerate() {
        by_text.entry(text.clone()).or_default().push(i);
    }
    let mut pending: Vec<String> = by_text.iter().filter(|(_, ids)| ids.len() > 1).map(|(text, _)| text.clone()).collect();
    while let Some(text) = pending.pop() {
        let Some(ids) = by_text.get_mut(&text) else { continue };
        if ids.len() < 2 {
            continue;
        }
        let (stay, moving): (Vec<usize>, Vec<usize>) = ids.iter().partition(|&&i| printers.get(i).is_some_and(|p| p.name == text));
        *ids = stay;
        for i in moving {
            let (Some(printer), Some(slot)) = (printers.get(i), shown.get_mut(i)) else { continue };
            slot.clone_from(&printer.name);
            let ids = by_text.entry(printer.name.clone()).or_default();
            ids.push(i);
            if ids.len() == 2 {
                pending.push(printer.name.clone());
            }
        }
    }
    shown
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Two-sided printing mode.
pub enum Duplex {
    /// One-sided printing.
    Off,
    /// Two-sided, flipping on the long edge.
    LongEdge,
    /// Two-sided, flipping on the short edge.
    ShortEdge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// What the user chose for one print job.
pub struct Settings {
    /// The chosen printer's [`Printer::name`].
    pub printer: String,
    /// Number of copies.
    pub copies: u32,
    /// None preserves the printer's default instead of forcing simplex.
    pub duplex: Option<Duplex>,
    /// Print in grayscale.
    pub grayscale: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { printer: String::new(), copies: 1, duplex: None, grayscale: false }
    }
}

/// Starts discovery of printers; the returned receiver delivers the list once.
pub type Discover = Box<dyn Fn() -> Receiver<Vec<Printer>>>;
/// The receiver of one print job's outcome: the backend's receipt on success, its error text on failure.
pub type Submission = Receiver<Result<String, String>>;
/// Starts a print job from the document's PDF bytes with the chosen [`Settings`].
pub type Submit = Box<dyn Fn(Vec<u8>, Settings) -> Submission>;

/// Apps format these states in their own language. Receipts and errors are backend text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feedback {
    /// Discovery is running.
    FindingPrinters,
    /// Discovery finished and found no printers.
    NoPrinters,
    /// Discovery's worker stopped without a list.
    DiscoveryFailed,
    /// A job was handed to the worker.
    Submitting,
    /// The job succeeded; carries the backend's receipt text.
    Submitted(String),
    /// The job failed; carries the backend's error text.
    Failed(String),
    /// The job's worker stopped without an outcome.
    WorkerStopped,
}

/// Translated labels supplied by the app; identity never depends on their text. [`PrintTexts`]
/// holds the same labels as owned, translatable text.
pub struct PrintLabels<'a> {
    #[allow(missing_docs)]
    pub printer: &'a str,
    #[allow(missing_docs)]
    pub select_printer: &'a str,
    #[allow(missing_docs)]
    pub refresh: &'a str,
    #[allow(missing_docs)]
    pub copies: &'a str,
    #[allow(missing_docs)]
    pub printer_default: &'a str,
    #[allow(missing_docs)]
    pub duplex: &'a str,
    #[allow(missing_docs)]
    pub one_sided: &'a str,
    #[allow(missing_docs)]
    pub long_edge: &'a str,
    #[allow(missing_docs)]
    pub short_edge: &'a str,
    #[allow(missing_docs)]
    pub grayscale: &'a str,
}

/// The former name of [`PrintLabels`].
#[deprecated(note = "renamed to PrintLabels")]
pub type Labels<'a> = PrintLabels<'a>;

craft_ui::words! {
    /// The labels of [`PrintLabels`] as owned text. `Default` is English; `PrintTexts::of(ui)` reads
    /// the words installed in the context, and [`PrintTexts::labels`] lends them to
    /// [`State::show`].
    pub struct PrintTexts {
        printer: "print.printer" = "Printer";
        select_printer: "print.selectPrinter" = "Select a printer";
        refresh: "print.refresh" = "Refresh";
        copies: "print.copies" = "Copies";
        printer_default: "print.printerDefault" = "Printer default";
        duplex: "print.duplex" = "Two-sided";
        one_sided: "print.oneSided" = "One-sided";
        long_edge: "print.longEdge" = "Long edge";
        short_edge: "print.shortEdge" = "Short edge";
        grayscale: "print.grayscale" = "Grayscale";
    }
}

impl PrintTexts {
    /// Lends the labels as [`PrintLabels`], for [`State::show`].
    pub fn labels(&self) -> PrintLabels<'_> {
        PrintLabels {
            printer: &self.printer,
            select_printer: &self.select_printer,
            refresh: &self.refresh,
            copies: &self.copies,
            printer_default: &self.printer_default,
            duplex: &self.duplex,
            one_sided: &self.one_sided,
            long_edge: &self.long_edge,
            short_edge: &self.short_edge,
            grayscale: &self.grayscale,
        }
    }
}

#[derive(Default)]
/// The state of the printer controls: settings, the printer list, the feedback and the running workers.
pub struct State {
    /// The current choices.
    pub settings: Settings,
    /// The printers found by the last discovery; empty while none is known.
    pub printers: Vec<Printer>,
    /// The latest feedback for the app to show, if any.
    pub message: Option<Feedback>,
    discovery: Option<Receiver<Vec<Printer>>>,
    submission: Option<Submission>,
    loaded: bool,
}

impl State {
    /// Whether a print job is running.
    pub fn busy(&self) -> bool {
        self.submission.is_some()
    }

    /// Whether printer discovery is running.
    pub fn discovering(&self) -> bool {
        self.discovery.is_some()
    }

    /// A refresh invalidates the old list until discovery completes: a removed queue must
    /// not remain selectable while the host is checking it.
    pub fn ready(&self) -> bool {
        !self.busy() && !self.discovering() && self.printers.iter().any(|p| p.name == self.settings.printer)
    }

    /// Refuse replacing an active job's receiver. The caller still owns the refused receiver;
    /// dropping a receiver cannot cancel a job already handed to a native worker.
    pub fn begin(&mut self, receiver: Submission) -> Result<(), Submission> {
        if self.busy() {
            return Err(receiver);
        }
        self.submission = Some(receiver);
        self.message = Some(Feedback::Submitting);
        Ok(())
    }

    /// False means another discovery or job is active; no worker was started.
    pub fn refresh(&mut self, discover: &Discover) -> bool {
        if self.discovering() || self.busy() {
            return false;
        }
        self.discovery = Some(discover());
        self.loaded = true;
        self.message = Some(Feedback::FindingPrinters);
        true
    }

    /// Reads finished workers' results into [`State::printers`] and [`State::message`], and asks for a repaint every 100 ms while a worker runs. Call it once per frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.discovery {
            match receiver.try_recv() {
                Ok(printers) => {
                    self.printers = printers;
                    if !self.printers.iter().any(|p| p.name == self.settings.printer) {
                        self.settings.printer =
                            self.printers.iter().find(|p| p.default).or(self.printers.first()).map(|p| p.name.clone()).unwrap_or_default();
                    }
                    self.message = self.printers.is_empty().then_some(Feedback::NoPrinters);
                    self.discovery = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.printers.clear();
                    self.message = Some(Feedback::DiscoveryFailed);
                    self.discovery = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.submission {
            match receiver.try_recv() {
                Ok(result) => {
                    self.message = Some(match result {
                        Ok(receipt) => Feedback::Submitted(receipt),
                        Err(error) => Feedback::Failed(error),
                    });
                    self.submission = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.message = Some(Feedback::WorkerStopped);
                    self.submission = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if self.discovering() || self.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    /// Draw just the controls. Apps render feedback, their preview and the submit/export
    /// buttons around this component, using their own command and document state.
    pub fn show(&mut self, ui: &mut egui::Ui, id: egui::Id, discover: &Discover, labels: &PrintLabels<'_>) {
        if !self.loaded {
            self.refresh(discover);
        }
        self.poll(ui.ctx());
        ui.push_id(id, |ui| {
            ui.add_enabled_ui(!self.busy(), |ui| {
                let width = ui.available_width().clamp(32.0, 230.0);
                let printer_label = ui.label(labels.printer);
                let shown = shown_names(&self.printers);
                let selected = self.printers.iter().zip(&shown).find(|(p, _)| p.name == self.settings.printer).map(|(_, text)| text.as_str());
                craft_ui::ComboBox::from_id_salt("printer")
                    .width(width)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(if self.settings.printer.is_empty() { labels.select_printer } else { selected.unwrap_or(&self.settings.printer) })
                    .show_ui(ui, |ui| {
                        for (printer, text) in self.printers.iter().zip(&shown) {
                            ui.selectable_value(&mut self.settings.printer, printer.name.clone(), text);
                        }
                    })
                    .response
                    .labelled_by(printer_label.id);
                if ui.add_enabled(!self.discovering(), egui::Button::new(labels.refresh)).clicked() {
                    self.refresh(discover);
                }
                ui.horizontal(|ui| {
                    let label = ui.label(labels.copies);
                    ui.add(egui::DragValue::new(&mut self.settings.copies).range(1..=999)).labelled_by(label.id);
                });
                let choices = [
                    (None, labels.printer_default),
                    (Some(Duplex::Off), labels.one_sided),
                    (Some(Duplex::LongEdge), labels.long_edge),
                    (Some(Duplex::ShortEdge), labels.short_edge),
                ];
                let duplex_label = ui.label(labels.duplex);
                craft_ui::ComboBox::from_id_salt("duplex")
                    .width(width)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(
                        choices.iter().find(|(value, _)| *value == self.settings.duplex).map(|(_, text)| *text).unwrap_or(labels.printer_default),
                    )
                    .show_ui(ui, |ui| {
                        for (value, text) in choices {
                            ui.selectable_value(&mut self.settings.duplex, value, text);
                        }
                    })
                    .response
                    .labelled_by(duplex_label.id);
                ui.checkbox(&mut self.settings.grayscale, labels.grayscale);
            });
        });
    }
}

/// The labels tables of the print controls, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[<PrintTexts as craft_ui::localized::Labels>::WORDS];

#[cfg(test)]
mod tests;
