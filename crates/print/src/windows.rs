//! The Windows spooler, in this process: no PowerShell, script or other program is started, so
//! printing works where scripts are locked down (App Control for Business or AppLocker run
//! PowerShell in Constrained Language Mode; storytold/pdfcraft#1107).
//!
//! The printers come from `EnumPrinters` (local queues and the user's connections, such as
//! `\\server\queue`), the default from `GetDefaultPrinter`, and whether each driver takes
//! two-sided settings from its PrintCapabilities. A job is drawn by the in-box
//! `Windows.Data.Pdf` with Direct2D into the XPS print API, as vectors, and its options (copies,
//! collation, two-sided, colour, paper, orientation, no driver scaling) travel as a PrintTicket
//! that the driver validates before anything is sent. `winprint` and `winsafe` wrap the Win32
//! and WinRT calls, so there is no `unsafe` here.

use std::io::Write as _;
use std::sync::{Mutex, MutexGuard, PoisonError};

use winprint::printer::{PrinterDevice, WinPdfPrinter};
use winprint::ticket::document::{NS_PSK, OwnedName, PrintFeature, PrintTicketDocument, reader::ParsableXmlDocument as _};
use winprint::ticket::{FeatureOptionPack as _, PageMediaSize, PrintCapabilities, PrintTicket, PrintTicketBuilder};

use crate::sheets::{self, Fit, SAME};
use crate::ticket::{self, Paper};
use crate::{Error, Job, Printer, Scaling};

fn spool_error(e: impl std::fmt::Display) -> Error {
    Error::Spool(e.to_string())
}

/// The user's default printer; `None` when there is none.
fn default_printer() -> Option<String> {
    winsafe::GetDefaultPrinter().ok().filter(|n| !n.is_empty())
}

/// Keeps two threads of this process out of the Windows PrintTicket provider (`PTOpenProvider`
/// and the calls on its handle) at the same time. When two threads make the process's first calls
/// together, winspool can keep a pointer to a device capabilities cache it has freed, and a later
/// call crashes the process.
static PRINT_TICKET_PROVIDER: Mutex<()> = Mutex::new(());

/// Holds [`PRINT_TICKET_PROVIDER`] until the guard is dropped. A lock that a panicking thread
/// held is taken like any other.
fn print_ticket_provider() -> MutexGuard<'static, ()> {
    PRINT_TICKET_PROVIDER.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn printers() -> Vec<Printer> {
    match PrinterDevice::all() {
        Ok(devices) => crate::printer_list(devices.iter().map(|d| (d.name().to_string(), two_sided(d))), default_printer().as_deref()),
        Err(_) => Vec::new(),
    }
}

/// Whether the printer's driver takes two-sided settings: its PrintCapabilities offer a two-sided
/// option for the feature the job's ticket sets. `None` when the capabilities can't be read. A
/// driver without one cannot confirm an explicit two-sided request, which is refused before submission.
fn two_sided(device: &PrinterDevice) -> Option<bool> {
    let capabilities = {
        let _provider = print_ticket_provider();
        PrintCapabilities::fetch(device)
    }
    .ok()?;
    let duplex = OwnedName::qualified("JobDuplexAllDocumentsContiguously", NS_PSK, Some("psk"));
    Some(capabilities.options_for_feature(duplex).any(|o| o.name.as_ref().is_some_and(|n| n.local_name.starts_with("TwoSided"))))
}

/// What the driver's PrintCapabilities offer for the job options [`ticket::verify_job_options`]
/// checks; all unknown when they can't be read.
fn driver_offers(device: &PrinterDevice) -> ticket::Offered {
    let xml = {
        let _provider = print_ticket_provider();
        PrintCapabilities::fetch_xml_for_ticket(device, None)
    };
    xml.ok().and_then(|xml| String::from_utf8(xml).ok()).map_or_else(ticket::Offered::default, |xml| ticket::offered(&xml))
}

/// The queue the job goes to: the one asked for, else the default. Never another one: a job
/// for a queue that has gone must not come out of a different printer.
fn select_device(devices: Vec<PrinterDevice>, requested: Option<&str>, default: Option<&str>) -> Result<PrinterDevice, Error> {
    let name = requested.or(default).ok_or_else(|| spool_error("there is no default printer; choose a printer"))?;
    devices
        .into_iter()
        .find(|p| p.name() == name)
        .ok_or_else(|| spool_error(format!("the printer is not available: {name} (reopen Print to refresh the list)")))
}

/// The ticket the driver accepted: [`ticket::ticket_xml`] with the driver's own paper option for
/// `sheet` (its exact option, so a vendor's sizes survive; a ticket with dimensions only can come
/// back as Letter) and its own "no scaling" option, merged and validated against the printer's
/// capabilities. For exact size the driver must have that paper; for shrink-to-fit without one,
/// its default paper is used. Returns the ticket and its paper in micrometres. Refused when the
/// driver changed the paper, the orientation or the scaling: then nothing is sent, rather than a
/// job at the wrong size.
fn driver_ticket(device: &PrinterDevice, job: &Job, sheet: (f64, f64), landscape: bool) -> Result<(PrintTicket, (u32, u32)), Error> {
    let capabilities = {
        let _provider = print_ticket_provider();
        PrintCapabilities::fetch(device)
    }
    .map_err(spool_error)?;
    let medias: Vec<PageMediaSize> = capabilities.page_media_sizes().collect();
    let papers: Vec<Paper<'_>> = medias
        .iter()
        .map(|m| Paper {
            width: m.size().width_in_micron(),
            height: m.size().height_in_micron(),
            name: m.display_name(),
            keyword: m.option().name.as_ref().is_some_and(|n| n.namespace_ref() == Some(NS_PSK)),
        })
        .collect();
    let media = ticket::matching_paper(&papers, sheet.0, sheet.1).and_then(|i| medias.get(i));
    let wanted = match (media, job.scaling) {
        (Some(m), _) => Some((m.size().width_in_micron(), m.size().height_in_micron())),
        (None, Scaling::ShrinkToFit) => None,
        (None, Scaling::Exact) => {
            return Err(spool_error(format!(
                "{} has no {:.2} × {:.2} inch paper; choose a paper size it has, or add this one in its driver",
                device.name(),
                sheet.0 / 72.0,
                sheet.1 / 72.0
            )));
        }
    };
    let requested = PrintTicket::from_xml(ticket::ticket_xml(job, landscape));
    let mut document = PrintTicketDocument::parse_from_bytes(requested.get_xml()).map_err(spool_error)?;
    if let Some(media) = media {
        let media_ticket: PrintTicket = media.clone().into();
        let media_document = PrintTicketDocument::parse_from_bytes(media_ticket.get_xml()).map_err(spool_error)?;
        document.features.extend(media_document.features);
        document.parameter_inits.extend(media_document.parameter_inits);
    }
    let scaling_name = OwnedName::qualified("PageScaling", NS_PSK, Some("psk"));
    if let Some(no_scaling) = capabilities.options_for_feature(scaling_name.clone()).find(|o| o.name.as_ref().is_some_and(|n| n.local_name == "None"))
    {
        document.features.retain(|f| !(f.name.local_name == "PageScaling" && f.name.namespace_ref() == Some(NS_PSK)));
        document.features.push(PrintFeature { name: scaling_name, properties: Vec::new(), options: vec![no_scaling.clone()], features: Vec::new() });
    }
    let built = {
        let _provider = print_ticket_provider();
        // The builder closes its provider when it is dropped, before the guard.
        let mut builder = PrintTicketBuilder::new(device).map_err(spool_error)?;
        builder.merge(PrintTicket::from(document)).map_err(spool_error)?;
        builder.build().map_err(spool_error)?
    };
    let validated = in_drivers_spelling(device, built);
    let xml = std::str::from_utf8(validated.get_xml()).map_err(spool_error)?;
    let paper = ticket::verify_ticket(xml, wanted, landscape)?;
    Ok((validated, paper))
}

/// The ticket with the driver's vendor namespaces under the prefixes the driver declares them with
/// ([`ticket::respell`]). Windows' merge renames them (`brpsk` to `ns0000`) when the merged ticket
/// uses one of the driver's names, such as its own paper option, and a Brother driver then fails
/// every page of the job with 0x80040003. The driver's prefixes are those of its own default ticket
/// (an empty one, validated). The ticket as it is when they can't be read.
fn in_drivers_spelling(device: &PrinterDevice, ticket: PrintTicket) -> PrintTicket {
    let spelled = (|| {
        let _provider = print_ticket_provider();
        let mut builder = PrintTicketBuilder::new(device).ok()?;
        builder.merge(PrintTicket::from_xml(ticket::empty_ticket())).ok()?;
        let own = builder.build().ok()?;
        ticket::respell(std::str::from_utf8(ticket.get_xml()).ok()?, std::str::from_utf8(own.get_xml()).ok()?)
    })();
    spelled.map_or(ticket, PrintTicket::from_xml)
}

/// The widest unprintable margin of the ticket's paper, in points; 0 when the driver doesn't say.
fn imageable_margin(device: &PrinterDevice, ticket: &PrintTicket) -> f64 {
    let xml = {
        let _provider = print_ticket_provider();
        PrintCapabilities::fetch_xml_for_ticket(device, Some(ticket))
    };
    xml.ok().and_then(|xml| String::from_utf8(xml).ok()).and_then(|xml| ticket::imageable_margin(&xml)).map_or(0.0, ticket::points)
}

/// Spool a print-ready PDF. The sheets are read first ([`sheets::read`]); everything that talks
/// to the driver runs on a thread of its own, so WinRT and COM get an apartment of their own
/// whoever calls. Windows.Data.Pdf reads from a path, so the job goes to a file in a private
/// temporary folder (an unpredictable name, only this user's) for as long as the spooler takes
/// it, and the folder is removed however the job ended. Windows answers once the job is in the
/// queue: a paused queue or an offline printer keeps the job there, as with any program.
pub(crate) fn submit_with_progress(pdf: &[u8], job: &Job, progress: &mut dyn FnMut(usize, usize)) -> Result<String, Error> {
    let read = sheets::read(pdf)?;
    read.check_print_annotations()?;
    let count = read.sheets.len();
    let first = read.sheets.first().map(sheets::Sheet::size).ok_or_else(|| spool_error("the print-ready PDF has no pages"))?;
    let landscape = first.0 > first.1 + SAME;
    // Exact size: every sheet must be the first one's size, either way round; checked before
    // the driver is asked anything.
    if job.scaling == Scaling::Exact {
        sheets::plan(&read.sheets, first, Fit::Exact)?;
    }
    let device = select_device(PrinterDevice::all().map_err(spool_error)?, job.printer.as_deref(), default_printer().as_deref())?;
    let written = |e: std::io::Error| spool_error(format!("the print job could not be written: {e}"));
    let dir = tempfile::Builder::new().prefix("craft-print-").tempdir().map_err(written)?;
    let path = dir.path().join(ticket::job_filename(&job.title));
    let read = &read;
    // The worker is joined before `dir` is dropped: the spooler reads the file.
    let result = crate::progress::run(progress, move |sender| {
        let (mut ticket, mut paper) = driver_ticket(&device, job, first, landscape)?;
        let xml = std::str::from_utf8(ticket.get_xml()).map_err(spool_error)?;
        let mut repeat = None;
        // Jobs of one copy each, for collated copies that can't be one job.
        let mut jobs = 1;
        // The capabilities are read when the ticket lacks a requested value: they tell a driver
        // that dropped it from one that never had the feature. Unreadable stays strict. A driver
        // can also keep a collation it doesn't offer, so collated copies always read them.
        let checked = ticket::verify_job_options(xml, job, count, ticket::Offered::default());
        if checked.is_err() || ticket::collated_copies(job, count) {
            let offered = driver_offers(&device);
            if ticket::repeats_collated(job, count, offered) {
                let single = Job { copies: 1, ..job.clone() };
                (ticket, paper) = driver_ticket(&device, &single, first, landscape)?;
                let xml = std::str::from_utf8(ticket.get_xml()).map_err(spool_error)?;
                ticket::verify_job_options(xml, &single, count, offered)?;
                // Two-sided as the driver will print it, the queue's default included.
                let copies = job.copies.clamp(1, 999);
                match ticket::one_job_sides(xml) {
                    Some(two_sided) => repeat = Some(sheets::Repeat { copies, two_sided }),
                    None => jobs = copies,
                }
            } else if checked.is_err() {
                ticket::verify_job_options(xml, job, count, offered)?;
            }
        }
        let (fit, paper) = match job.scaling {
            Scaling::Exact => (Fit::Exact, first),
            Scaling::ShrinkToFit => {
                let (short, long) = (ticket::points(paper.0.min(paper.1)), ticket::points(paper.0.max(paper.1)));
                (Fit::Shrink { margin: imageable_margin(&device, &ticket) }, if landscape { (long, short) } else { (short, long) })
            }
        };
        let edits = sheets::plan(&read.sheets, paper, fit)?;
        let mut updated = None;
        if let Some(r) = repeat {
            updated = read.repeated(&edits, r)?;
            if updated.is_none() {
                (repeat, jobs) = (None, r.copies);
            }
        }
        if updated.is_none() && !edits.is_empty() {
            updated = Some(read.update(&edits)?);
        }
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(written)?;
        file.write_all(updated.as_deref().unwrap_or(pdf)).map_err(written)?;
        drop(file);
        let printer = WinPdfPrinter::new(device);
        let jobs = jobs as usize;
        for copy in 0..jobs {
            // winprint converts the ticket to a DEVMODE through the provider before each job's
            // first progress report, which releases the guard.
            let mut provider = Some(print_ticket_provider());
            printer
                .print_with_progress(&path, ticket.clone(), |processed, total| {
                    drop(provider.take());
                    // Page acceptance is distinct from terminal job success. The callback
                    // drains on the submitter's thread, so it can retain non-Send UI state.
                    let (processed, total) = (processed as usize, total as usize);
                    let _ = sender.send((copy * total + processed, jobs * total));
                })
                .map_err(|e| {
                    let mut message = e.to_string();
                    let mut cause = std::error::Error::source(&e);
                    while let Some(source) = cause {
                        message.push_str(&format!(": {source}"));
                        cause = source.source();
                    }
                    if copy > 0 {
                        message = format!("{copy} of {jobs} copies were sent, each as a job of its own, and the next was not: {message}");
                    }
                    spool_error(message)
                })?;
        }
        Ok((repeat, jobs))
    });
    drop(dir);
    Ok(match result? {
        (Some(repeat), _) => format!("spooled {count} sheet(s) {} times, collated", repeat.copies),
        (None, jobs) if jobs > 1 => format!("spooled {count} sheet(s) as {jobs} jobs, one per collated copy"),
        _ => format!("spooled {count} sheet(s)"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_ui::testing::trial;

    #[trial]
    fn one_thread_at_a_time_is_in_the_print_ticket_provider() {
        use std::sync::Barrier;
        use std::sync::atomic::{AtomicUsize, Ordering};
        const THREADS: usize = 12;
        let inside = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);
        let start = Barrier::new(THREADS);
        std::thread::scope(|s| {
            for _ in 0..THREADS {
                s.spawn(|| {
                    start.wait();
                    for _ in 0..20 {
                        let _provider = print_ticket_provider();
                        most.fetch_max(inside.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        inside.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }
        });
        assert_eq!(most.load(Ordering::SeqCst), 1);
    }
}
