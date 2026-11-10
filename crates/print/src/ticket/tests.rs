use super::*;

#[test]
fn the_ticket_carries_the_job_options() {
    let job = Job { copies: 3, collate: false, duplex: Some(Duplex::ShortEdge), grayscale: true, title: "<&>".into(), ..Job::default() };
    let xml = ticket_xml(&job, true);
    for value in [">3<", "psk:Uncollated", "psk:TwoSidedShortEdge", "psk:Monochrome", "psk:Landscape", "psk:None"] {
        assert!(xml.contains(value), "missing {value}");
    }
    assert!(!xml.contains("<&>"), "the title never enters the XML");
    assert!(roxmltree::Document::parse(&xml).is_ok());
    let xml = ticket_xml(&Job { copies: 5000, ..Job::default() }, false);
    for value in [">999<", "psk:Collated", "psk:Portrait"] {
        assert!(xml.contains(value), "missing {value}");
    }
    assert!(!xml.contains("JobDuplex"), "no duplex asked: the queue's default applies");
    assert!(!xml.contains("PageOutputColor"), "no grayscale asked: the driver's own colour setting applies");
    assert!(ticket_xml(&Job { duplex: Some(Duplex::Off), ..Job::default() }, false).contains("psk:OneSided"));
    assert!(ticket_xml(&Job { duplex: Some(Duplex::LongEdge), ..Job::default() }, false).contains("psk:TwoSidedLongEdge"));
    assert!(!ticket_xml(&Job { printer: Some("Office \"east\" <1>".into()), ..Job::default() }, false).contains("east"));
}

/// A validated ticket as a driver returns it.
fn validated(paper: (u32, u32), orientation: &str, scaling: Option<&str>) -> String {
    let scaling = scaling.map_or(String::new(), |s| format!(r#"<psf:Feature name="psk:PageScaling"><psf:Option name="{s}"/></psf:Feature>"#));
    format!(
        r#"<psf:PrintTicket version="1" xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}" xmlns:ns0000="http://example.com/vendor" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
<psf:Feature name="psk:PageMediaSize"><psf:Option name="psk:NorthAmericaLetter">
<psf:ScoredProperty name="psk:MediaSizeWidth"><psf:Value xsi:type="xsd:integer">{}</psf:Value></psf:ScoredProperty>
<psf:ScoredProperty name="psk:MediaSizeHeight"><psf:Value xsi:type="xsd:integer">{}</psf:Value></psf:ScoredProperty>
</psf:Option></psf:Feature>
<psf:Feature name="psk:PageOrientation"><psf:Option name="psk:{orientation}"/></psf:Feature>{scaling}
<psf:ParameterInit name="ns0000:Empty"><psf:Value xsi:type="xsd:string"></psf:Value></psf:ParameterInit>
</psf:PrintTicket>"#,
        paper.0, paper.1
    )
}

#[test]
fn a_driver_that_changes_the_sheet_is_refused() {
    let letter = (215_900, 279_400);
    let ticket = validated(letter, "Portrait", Some("psk:None"));
    assert_eq!(verify_ticket(&ticket, Some(letter), false), Ok(letter));
    assert_eq!(verify_ticket(&ticket, Some((279_400, 215_900)), false), Ok(letter), "either way round");
    assert_eq!(verify_ticket(&ticket, None, false), Ok(letter), "shrink-to-fit takes the driver's paper");
    assert!(verify_ticket(&ticket, Some((210_000, 297_000)), false).is_err(), "A4 asked, Letter returned");
    assert!(verify_ticket(&ticket, Some(letter), true).is_err(), "landscape asked, portrait returned");
    assert!(verify_ticket(&validated(letter, "Portrait", Some("psk:FitApplicationBleedSizeToPageImageableSize")), Some(letter), false).is_err());
    assert!(verify_ticket(&validated(letter, "Portrait", Some("ns0000:None")), Some(letter), false).is_ok(), "the IPP class driver's None");
    assert!(verify_ticket(&validated(letter, "Portrait", None), Some(letter), false).is_ok(), "a driver without scaling");
    assert!(verify_ticket(&validated((0, 279_400), "Portrait", None), None, false).is_err(), "a roll has no length");
    assert!(verify_ticket("<not xml", Some(letter), false).is_err());
}

/// The namespace of the Brother driver's own keywords.
const BROTHER: &str = "http://schemas.brother.info/mfc/printing/2006/11/printschemakeywords";

/// A Brother driver's ticket with its vendor namespace declared as `prefix`. The driver's own
/// spelling is `brpsk`; Windows' ticket merge returned `ns0000` for the same ticket, and the
/// printer then failed every page (`0x80040003`).
fn brother(prefix: &str) -> String {
    format!(
        r#"<psf:PrintTicket xmlns:psf="{NS_PSF}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" version="1" xmlns:psk="{NS_PSK}" xmlns:{prefix}="{BROTHER}"><psf:Feature name="psk:PageMediaSize"><psf:Option name="psk:NorthAmericaLetter"><psf:ScoredProperty name="psk:MediaSizeWidth"><psf:Value xsi:type="xsd:integer">215900</psf:Value></psf:ScoredProperty><psf:ScoredProperty name="psk:MediaSizeHeight"><psf:Value xsi:type="xsd:integer">279400</psf:Value></psf:ScoredProperty></psf:Option></psf:Feature><psf:Feature name="psk:PageOrientation"><psf:Option name="psk:Portrait"/></psf:Feature><psf:Property name="{prefix}:PageMediaSizeData"><psf:Property name="{prefix}:CustomUnitType"><psf:Value xsi:type="xsd:string">mm</psf:Value></psf:Property></psf:Property><psf:Feature name="{prefix}:JobPrintSpeed"><psf:Option name="{prefix}:PrinterDefault"/></psf:Feature><psf:ParameterInit name="{prefix}:Mode"><psf:Value xsi:type="xsd:QName">{prefix}:Fast</psf:Value></psf:ParameterInit><psf:ParameterInit name="{prefix}:Label"><psf:Value xsi:type="xsd:string">ns0000:text, not a name</psf:Value></psf:ParameterInit></psf:PrintTicket>"#
    )
}

/// A QName value written after whitespace (a pretty-printed ticket) is respelled too.
#[test]
fn a_qname_value_after_whitespace_is_respelled() {
    let spaced = |prefix: &str| brother(prefix).replace(&format!(">{prefix}:Fast<"), &format!(">\n  {prefix}:Fast\n<"));
    assert_eq!(respell(&spaced("ns0000"), &brother("brpsk")), Some(spaced("brpsk")));
}

/// The ticket Windows returns (`ns0000`) gets the driver's own `brpsk` back, in the declaration, the names and the QName values, but not in text.
#[test]
fn a_vendor_namespace_gets_the_prefix_the_driver_declares_it_with() {
    let merged = brother("ns0000");
    assert_eq!(respell(&merged, &brother("brpsk")), Some(brother("brpsk")), "declaration, names and QName values; not the text");
    assert_eq!(respell(&brother("brpsk"), &brother("brpsk")), None, "already the driver's");
    assert_eq!(respell(&brother("brpsk"), &merged), Some(merged.clone()), "whatever the driver calls it");
    // What the job's checks read is the same either way.
    let letter = (215_900, 279_400);
    assert_eq!(verify_ticket(&merged, Some(letter), false), Ok(letter));
    assert_eq!(verify_ticket(&respell(&merged, &brother("brpsk")).unwrap(), Some(letter), false), Ok(letter));
}

/// A second vendor namespace, next to [`BROTHER`].
const OTHER: &str = "http://example.com/other";

/// A ticket that names [`BROTHER`] with the prefix `a` and [`OTHER`] with `b` in its root, uses
/// each in a `name`, and ends with `extra`.
fn two_namespaces(a: &str, b: &str, extra: &str) -> String {
    format!(
        r#"<psf:PrintTicket xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}" xmlns:{a}="{BROTHER}" xmlns:{b}="{OTHER}"><psf:Feature name="{a}:One"/><psf:Feature name="{b}:Two"/>{extra}</psf:PrintTicket>"#
    )
}

/// Prefixes are renamed together, so two namespaces that swap prefixes still end up distinct.
#[test]
fn two_vendor_namespaces_can_trade_prefixes() {
    let ticket = |a: &str, b: &str| two_namespaces(a, b, "");
    assert_eq!(respell(&ticket("ns0000", "ns0001"), &ticket("ns0001", "ns0000")), Some(ticket("ns0001", "ns0000")));
}

/// When two namespaces trade prefixes both prefixes stay declared, so a name that is only
/// written with a prefix (an element or an attribute, unlike a `name` value) would still parse and
/// resolve, but to the other namespace. Each ticket here is well-formed and has every prefix
/// declared, so `respell` leaves it alone for the name, not because the result didn't parse.
#[test]
fn a_swap_does_not_move_an_element_or_attribute_name_to_the_other_namespace() {
    let driver = two_namespaces("ns0001", "ns0000", "");
    let refused = |extra: &str| {
        let ticket = two_namespaces("ns0000", "ns0001", extra);
        let document = roxmltree::Document::parse(&ticket).expect("the fixture must stay well-formed");
        assert!(prefixes_resolve(&document));
        respell(&ticket, &driver)
    };
    assert!(refused("").is_some(), "control: the same ticket without such a name is respelled");

    // An element: `ns0000:Feature` is BROTHER's until the declarations swap, then OTHER's.
    assert_eq!(refused("<ns0000:Feature/>"), None, "element in the first namespace");
    assert_eq!(refused("<ns0001:Feature/>"), None, "element in the second namespace");
    assert_eq!(refused("<psf:Property name=\"ns0001:Three\"><ns0000:Child/></psf:Property>"), None, "nested element");

    // An attribute, whichever element carries it.
    assert_eq!(refused("<psf:Feature name=\"ns0000:Three\" ns0000:flag=\"1\"/>"), None, "attribute in the first namespace");
    assert_eq!(refused("<psf:Feature name=\"ns0000:Three\" ns0001:flag=\"1\"/>"), None, "attribute in the second namespace");
    assert_eq!(refused("<ns0001:Feature ns0000:flag=\"1\"/>"), None, "both at once");
}

/// The same when the prefixes form a chain rather than a swap: `brpsk` is OTHER's, is about to
/// become BROTHER's, and OTHER moves on to `ns0005`.
#[test]
fn a_name_in_a_namespace_whose_prefix_is_taken_over_is_not_respelled() {
    let driver = two_namespaces("brpsk", "ns0005", "");
    let refused = |extra: &str| {
        let ticket = two_namespaces("ns0000", "brpsk", extra);
        let document = roxmltree::Document::parse(&ticket).expect("the fixture must stay well-formed");
        assert!(prefixes_resolve(&document));
        respell(&ticket, &driver)
    };
    assert!(refused("").is_some(), "control");
    assert_eq!(refused("<brpsk:Feature/>"), None, "element: OTHER's now, BROTHER's after");
    assert_eq!(refused("<psf:Feature brpsk:flag=\"1\"/>"), None, "attribute");
}

/// What the live driver test checks. A ticket `respell` refused keeps the prefix the merge gave
/// it, but `respell` says `None` for it, as it does for a ticket that is already right: only the
/// prefixes tell the two apart.
#[test]
fn the_prefixes_tell_a_refused_ticket_from_one_in_the_drivers_spelling() {
    let driver = brother("brpsk");
    let unlike = |ticket: &str| prefixes_unlike_the_drivers(ticket, &driver).expect("both tickets parse");
    let merged = brother("ns0000");
    let mismatch = (BROTHER.to_string(), "ns0000".to_string(), "brpsk".to_string());

    assert_eq!(unlike(&merged), std::slice::from_ref(&mismatch), "as the merge returns it");
    assert_eq!(unlike(&respell(&merged, &driver).unwrap()), [], "respelled");
    assert_eq!(unlike(&driver), [], "already the driver's");

    // `respell` refuses this one (`ns0000` is declared again below the root), so it stays as the
    // merge left it, and `respell` says `None` for it just as it does for the driver's own.
    let refused = merged.replace(
        "<psf:Feature name=\"ns0000:JobPrintSpeed\">",
        "<psf:Feature name=\"ns0000:JobPrintSpeed\" xmlns:ns0000=\"http://example.com/nested\">",
    );
    assert_ne!(refused, merged);
    assert_eq!(respell(&refused, &driver), None);
    assert_eq!(respell(&driver, &driver), None);
    assert_eq!(unlike(&refused), [mismatch], "refused, and still wrong");

    // Two namespaces that trade prefixes are both reported.
    let traded = prefixes_unlike_the_drivers(&two_namespaces("ns0000", "ns0001", ""), &two_namespaces("ns0001", "ns0000", "")).unwrap();
    assert_eq!(traded.len(), 2, "{traded:?}");
    // A namespace the driver doesn't declare has no prefix of the driver's to differ from.
    assert_eq!(unlike(&empty_ticket()), [], "no vendor namespace to compare");
    assert_eq!(unlike(&two_namespaces("ns0000", "ns0001", "")).len(), 1, "only BROTHER is the driver's");
    assert_eq!(prefixes_unlike_the_drivers("<not xml", &driver), None);
    assert_eq!(prefixes_unlike_the_drivers(&merged, "<not xml"), None);
}

/// Only names in a namespace whose prefix changes block the respelling.
#[test]
fn names_in_a_namespace_that_keeps_its_prefix_do_not_block_the_respelling() {
    let keep = "http://example.com/keep";
    let ticket = |prefix: &str| {
        format!(
            r#"<psf:PrintTicket xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}" xmlns:{prefix}="{BROTHER}" xmlns:keep="{keep}"><psf:Feature name="{prefix}:One" keep:flag="1"><keep:Child keep:flag="2"/></psf:Feature><psf:Feature xmlns:own="http://example.com/own" own:flag="3"/></psf:PrintTicket>"#
        )
    };
    assert_eq!(respell(&ticket("ns0000"), &ticket("brpsk")), Some(ticket("brpsk")));
}

/// Each ticket here is well-formed XML, so `respell` leaves it alone for the reason named, not
/// because it didn't parse.
#[test]
fn a_ticket_that_cannot_be_respelled_safely_is_left_alone() {
    let merged = brother("ns0000");
    let driver = brother("brpsk");
    let refused = |ticket: &str| {
        assert!(roxmltree::Document::parse(ticket).is_ok(), "the fixture must stay well-formed");
        respell(ticket, &driver)
    };
    assert_eq!(respell(&merged, &empty_ticket()), None, "the driver has no such namespace");
    assert_eq!(respell("<not xml", &driver), None);
    assert_eq!(respell(&merged, "<not xml"), None);
    // `brpsk` is already another namespace's prefix in the ticket.
    assert_eq!(refused(&merged.replace("xmlns:psk=", "xmlns:brpsk=\"http://example.com/other\" xmlns:psk=")), None);
    // A vendor element can't be renamed by changing attributes and declarations.
    let feature = "<psf:Feature name=\"ns0000:JobPrintSpeed\"><psf:Option name=\"ns0000:PrinterDefault\"/></psf:Feature>";
    assert!(merged.contains(feature));
    assert_eq!(refused(&merged.replace(feature, "<ns0000:Feature/>")), None);
    // A name whose prefix is not declared stays refused after the change.
    assert_eq!(refused(&merged.replace("name=\"ns0000:Mode\"", "name=\"ns0001:Mode\"")), None);
    // A nested element that declares either prefix again would change what a name resolves to.
    for redeclared in ["ns0000", "brpsk"] {
        let nested = merged.replace(
            "<psf:Feature name=\"ns0000:JobPrintSpeed\">",
            &format!("<psf:Feature name=\"ns0000:JobPrintSpeed\" xmlns:{redeclared}=\"http://example.com/nested\">"),
        );
        assert_ne!(nested, merged);
        assert_eq!(refused(&nested), None, "{redeclared} declared again below the root");
    }
    // Control: the same ticket without the nested declaration is respelled.
    assert!(refused(&merged).is_some());
}

#[test]
fn a_driver_that_drops_explicit_job_options_is_refused() {
    let job = Job { copies: 2, duplex: Some(Duplex::LongEdge), grayscale: true, ..Job::default() };
    let xml = ticket_xml(&job, false);
    assert!(verify_job_options(&xml, &job, 3, Offered::default()).is_ok());
    for (from, to) in [(">2<", ">1<"), ("psk:Collated", "psk:Uncollated"), ("psk:TwoSidedLongEdge", "psk:OneSided"), ("psk:Monochrome", "psk:Color")]
    {
        assert!(verify_job_options(&xml.replace(from, to), &job, 3, Offered::default()).is_err(), "{from}");
    }
    assert!(verify_job_options(&xml.replace("psk:Collated", "vendor:Collated"), &job, 3, Offered::default()).is_err(), "namespace matters");
    assert!(verify_job_options(&xml.replace("psk:Monochrome", "psk:Grayscale"), &job, 3, Offered::default()).is_ok());
    for feature in ["DocumentCollate", "JobDuplexAllDocumentsContiguously", "PageOutputColor"] {
        let start = xml.find(&format!("<psf:Feature name=\"psk:{feature}\"")).unwrap();
        let end = start + xml[start..].find("</psf:Feature>").unwrap() + "</psf:Feature>".len();
        let mut dropped = xml.clone();
        dropped.replace_range(start..end, "");
        assert!(verify_job_options(&dropped, &job, 3, Offered::default()).is_err(), "{feature}");
        if feature == "DocumentCollate" {
            assert!(verify_job_options(&dropped, &job, 1, Offered::default()).is_ok(), "one page has no collating order");
        }
    }
    let default = Job::default();
    let xml = ticket_xml(&default, false);
    assert!(verify_job_options(&xml, &default, 3, Offered::default()).is_ok());
    let simplex = Job { duplex: Some(Duplex::Off), ..Job::default() };
    assert!(verify_job_options(&ticket_xml(&simplex, false), &simplex, 3, Offered::default()).is_ok());
    assert!(verify_job_options(&xml, &simplex, 3, Offered::default()).is_err(), "explicit simplex must be confirmed");
    assert!(verify_job_options("<not xml", &default, 1, Offered::default()).is_err());
}

/// PrintCapabilities as a driver might write them: with or without a duplex feature, and the
/// colour and collation options given (none: no such feature).
fn capabilities_xml(duplex: bool, colors: &[&str], collations: &[&str]) -> String {
    let feature = |name: &str, options: &[&str]| {
        if options.is_empty() {
            return String::new();
        }
        let options: String = options.iter().map(|o| format!(r#"<psf:Option name="{o}"/>"#)).collect();
        format!(r#"<psf:Feature name="psk:{name}">{options}</psf:Feature>"#)
    };
    let duplex = feature("JobDuplexAllDocumentsContiguously", if duplex { &["psk:OneSided", "psk:TwoSidedLongEdge"] } else { &[] });
    format!(
        r#"<psf:PrintCapabilities version="1" xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}" xmlns:ns0000="http://example.com/vendor">
{}{duplex}{}
</psf:PrintCapabilities>"#,
        feature("DocumentCollate", collations),
        feature("PageOutputColor", colors)
    )
}

const COLOR: &[&str] = &["psk:Color", "psk:Monochrome"];
const COLLATE: &[&str] = &["psk:Collated", "psk:Uncollated"];

/// The ticket without one of its features, as a driver that doesn't know it returns it.
fn without(xml: &str, feature: &str) -> String {
    let start = xml.find(&format!("<psf:Feature name=\"psk:{feature}\"")).unwrap();
    let end = start + xml[start..].find("</psf:Feature>").unwrap() + "</psf:Feature>".len();
    let mut dropped = xml.to_owned();
    dropped.replace_range(start..end, "");
    dropped
}

#[test]
fn a_driver_without_a_duplex_feature_prints_one_side_and_is_not_refused_for_it() {
    let offered = |duplex| offered(&capabilities_xml(duplex, COLOR, COLLATE));
    assert_eq!(offered(true), Offered { duplex: Some(true), color: Some(true), collate: Some(true) });
    assert_eq!(offered(false).duplex, Some(false));
    assert_eq!(super::offered("<psf:PrintCapabilities xmlns:psf=\"x\"/>"), Offered::default(), "another namespace is not capabilities");
    assert_eq!(super::offered("<not xml"), Offered::default());
    let empty =
        format!(r#"<psf:PrintCapabilities xmlns:psf="{NS_PSF}"><psf:Feature name="psk:JobDuplexAllDocumentsContiguously"/></psf:PrintCapabilities>"#);
    assert_eq!(super::offered(&empty).duplex, Some(false), "a feature with no option offers nothing");
    let document_duplex = format!(
        r#"<psf:PrintCapabilities xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}"><psf:Feature name="psk:DocumentDuplex"><psf:Option name="psk:OneSided"/><psf:Option name="psk:TwoSidedLongEdge"/></psf:Feature></psf:PrintCapabilities>"#
    );
    assert_eq!(super::offered(&document_duplex).duplex, Some(true), "DocumentDuplex is a duplex feature too");

    let one_sided = Job { duplex: Some(Duplex::Off), ..Job::default() };
    let two_sided = Job { duplex: Some(Duplex::LongEdge), ..Job::default() };
    let kept = ticket_xml(&one_sided, false);
    // The ticket a driver with no duplex feature returns: the setting is gone.
    let none_kept = ticket_xml(&Job::default(), false);

    // The Print dialog's defaults (duplex Off) on a simplex-only queue.
    assert!(verify_job_options(&none_kept, &one_sided, 3, offered(false)).is_ok());
    // Offered and kept.
    assert!(verify_job_options(&kept, &one_sided, 3, offered(true)).is_ok());
    // Offered but the ticket lost it: the driver dropped the choice.
    assert!(verify_job_options(&none_kept, &one_sided, 3, offered(true)).is_err());
    // Two-sided never rests on the missing feature.
    assert!(verify_job_options(&none_kept, &two_sided, 3, offered(false)).is_err());
    // Unknown capabilities stay strict.
    assert!(verify_job_options(&none_kept, &one_sided, 3, Offered::default()).is_err());
    // The other options are still checked on a simplex-only queue.
    let copies = Job { copies: 2, duplex: Some(Duplex::Off), ..Job::default() };
    assert!(verify_job_options(&none_kept, &copies, 3, offered(false)).is_err(), "copy count");
    let gray = Job { grayscale: true, duplex: Some(Duplex::Off), ..Job::default() };
    assert!(verify_job_options(&none_kept, &gray, 3, offered(false)).is_err(), "grayscale");
}

#[test]
fn a_driver_without_colour_prints_grayscale_and_is_not_refused_for_it() {
    let colors = |colors: &[&str]| offered(&capabilities_xml(true, colors, COLLATE)).color;
    assert_eq!(colors(COLOR), Some(true));
    assert_eq!(colors(&["psk:Monochrome"]), Some(false), "monochrome-only");
    assert_eq!(colors(&["psk:Grayscale", "psk:Monochrome"]), Some(false));
    assert_eq!(colors(&[]), Some(false), "no colour feature");
    assert_eq!(colors(&["psk:Monochrome", "ns0000:Vivid"]), Some(true), "a vendor's option may be colour");

    let gray = Job { grayscale: true, ..Job::default() };
    let kept = ticket_xml(&gray, false);
    // A driver without the feature returns a ticket without it.
    let dropped = without(&kept, "PageOutputColor");
    let color = kept.replace("psk:Monochrome", "psk:Color");
    let caps = |colors: &[&str]| offered(&capabilities_xml(true, colors, COLLATE));

    assert!(verify_job_options(&dropped, &gray, 3, caps(&["psk:Monochrome"])).is_ok());
    assert!(verify_job_options(&dropped, &gray, 3, caps(&[])).is_ok());
    assert!(verify_job_options(&kept, &gray, 3, caps(COLOR)).is_ok());
    // A colour driver that dropped or changed the request.
    assert!(verify_job_options(&color, &gray, 3, caps(COLOR)).is_err());
    assert!(verify_job_options(&dropped, &gray, 3, caps(COLOR)).is_err());
    assert!(verify_job_options(&dropped, &gray, 3, Offered::default()).is_err(), "unknown stays strict");
}

#[test]
fn copies_on_a_driver_that_cannot_collate_are_accepted_only_uncollated() {
    let caps = |collations: &[&str]| offered(&capabilities_xml(true, COLOR, collations));
    assert_eq!(caps(COLLATE).collate, Some(true));
    assert_eq!(caps(&["psk:Uncollated"]).collate, Some(false));
    assert_eq!(caps(&[]).collate, Some(false));

    let collated = Job { copies: 2, collate: true, ..Job::default() };
    let uncollated = Job { copies: 2, collate: false, ..Job::default() };
    let dropped = |job: &Job| without(&ticket_xml(job, false), "DocumentCollate");

    // Uncollated copies are what a driver without collation prints.
    assert!(verify_job_options(&dropped(&uncollated), &uncollated, 3, caps(&[])).is_ok());
    assert!(verify_job_options(&ticket_xml(&uncollated, false), &uncollated, 3, caps(&["psk:Uncollated"])).is_ok());
    // Collated copies are refused, saying why.
    let refused = verify_job_options(&dropped(&collated), &collated, 3, caps(&[])).unwrap_err();
    assert!(refused.to_string().contains("can't collate"), "{refused}");
    // A driver that collates and dropped or changed the request.
    assert!(verify_job_options(&dropped(&collated), &collated, 3, caps(COLLATE)).is_err());
    assert!(verify_job_options(&dropped(&uncollated), &uncollated, 3, caps(COLLATE)).is_err());
    let changed = ticket_xml(&collated, false).replace("psk:Collated", "psk:Uncollated");
    assert!(verify_job_options(&changed, &collated, 3, caps(COLLATE)).is_err());
    assert!(verify_job_options(&dropped(&uncollated), &uncollated, 3, Offered::default()).is_err(), "unknown stays strict");
    // The job repeats the document instead of asking for collated copies.
    assert!(repeats_collated(&collated, 3, caps(&[])));
    assert!(!repeats_collated(&collated, 3, caps(COLLATE)) && !repeats_collated(&collated, 3, Offered::default()));
    assert!(!repeats_collated(&uncollated, 3, caps(&[])) && !repeats_collated(&collated, 1, caps(&[])));
    assert!(!repeats_collated(&Job { copies: 1, ..collated.clone() }, 3, caps(&[])));
    // One page or one copy has no collating order.
    assert!(verify_job_options(&dropped(&collated), &collated, 1, caps(&[])).is_ok());
}

#[test]
fn the_driver_paper_named_like_the_standard_size_is_preferred() {
    let paper = |width, height, name, keyword| Paper { width, height, name, keyword };
    // A plotter listing 24 × 36 in three times: a custom entry first, "ARCH D", and one a hair off.
    let papers = [
        paper(609_600, 914_400, Some("Custom 24x36"), false),
        paper(609_700, 914_400, Some("ARCH D 24 x 36 in"), false),
        paper(609_600, 914_400, Some("24 x 36"), true),
    ];
    assert_eq!(matching_paper(&papers, 2592.0, 1728.0), Some(1), "named, either way round");
    assert_eq!(matching_paper(&papers[..1], 1728.0, 2592.0), Some(0));
    assert_eq!(matching_paper(&[papers[0].clone(), papers[2].clone()], 1728.0, 2592.0), Some(1), "then a schema keyword");
    assert_eq!(matching_paper(&papers, 1728.0, 2600.0), None, "3 mm off is another paper");
    assert_eq!(matching_paper(&papers, f64::NAN, 2592.0), None);
    assert_eq!(paper_names(595.0, 842.0), ["A4"], "rounded metric points");
    assert_eq!(paper_names(792.0, 612.0), ["Letter", "US Letter"]);
    assert!(paper_names(360.0, 504.0).is_empty());
    assert!(names_paper(Some("ARCH D 24 x 36 in"), "ARCH D"));
    assert!(names_paper(Some("arch d"), "ARCH D"));
    assert!(!names_paper(Some("ANSI D"), "ARCH D"));
    assert!(!names_paper(Some("ARCH E1"), "ARCH E"));
    assert!(!names_paper(Some("A4Extra"), "A4"));
    assert!(!names_paper(Some("Ä"), "A4"), "never slices inside a character");
    assert!(!names_paper(None, "A4"));
}

#[test]
fn the_unprintable_margin_comes_from_the_capabilities() {
    let caps = |x: u32, y: u32, w: u32, h: u32| {
        format!(
            r#"<psf:PrintCapabilities version="1" xmlns:psf="{NS_PSF}" xmlns:psk="{NS_PSK}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
<psf:Property name="psk:PageImageableSize">
 <psf:Property name="psk:ImageableSizeWidth"><psf:Value xsi:type="xsd:integer">215900</psf:Value></psf:Property>
 <psf:Property name="psk:ImageableSizeHeight"><psf:Value xsi:type="xsd:integer">279400</psf:Value></psf:Property>
 <psf:Property name="psk:ImageableArea">
  <psf:Property name="psk:OriginWidth"><psf:Value xsi:type="xsd:integer">{x}</psf:Value></psf:Property>
  <psf:Property name="psk:OriginHeight"><psf:Value xsi:type="xsd:integer">{y}</psf:Value></psf:Property>
  <psf:Property name="psk:ExtentWidth"><psf:Value xsi:type="xsd:integer">{w}</psf:Value></psf:Property>
  <psf:Property name="psk:ExtentHeight"><psf:Value xsi:type="xsd:integer">{h}</psf:Value></psf:Property>
 </psf:Property>
</psf:Property>
</psf:PrintCapabilities>"#
        )
    };
    assert_eq!(imageable_margin(&caps(4_233, 4_233, 207_434, 270_934)), Some(4_233));
    assert_eq!(imageable_margin(&caps(3_000, 5_000, 209_900, 270_000)), Some(5_000), "the widest of the four");
    assert_eq!(imageable_margin(&caps(0, 0, 215_900, 279_400)), Some(0), "borderless");
    assert_eq!(imageable_margin(&caps(10_000, 0, 215_900, 279_400)), None, "an area past the paper is not believed");
    assert_eq!(imageable_margin("<psf:PrintCapabilities/>"), None);
}

#[test]
fn points_and_micrometres_convert_both_ways() {
    assert_eq!(micrometres(612.0), Some(215_900));
    assert_eq!(micrometres(841.89), Some(297_000));
    assert!((points(215_900) - 612.0).abs() < 1e-9);
    assert!((points(4_064) - 11.52).abs() < 1e-9, "a 4 mm hardware margin");
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, 14_401.0] {
        assert_eq!(micrometres(bad), None, "{bad}");
    }
}

#[test]
fn the_job_file_stays_in_its_folder() {
    let name = job_filename(r"..\..\CON:/unsafe?name");
    assert_eq!(std::path::Path::new(&name).components().count(), 1, "{name}");
    assert!(name.ends_with(".pdf"));
    assert_eq!(job_filename("Invoice 42.pdf"), "Invoice 42.pdf");
    assert_eq!(job_filename("Report (draft) v1.2"), "Report (draft) v1.2.pdf");
    assert_eq!(job_filename("con"), "_con.pdf");
    assert_eq!(job_filename("LPT1.txt"), "_LPT1.txt.pdf");
    assert_eq!(job_filename("COMPLETE"), "COMPLETE.pdf");
    assert_eq!(job_filename("..."), "Print.pdf");
    assert_eq!(job_filename(""), "Print.pdf");
    assert_eq!(job_filename("Отчёт"), "Отчёт.pdf");
}

#[test]
fn a_repeated_job_reads_its_sides_and_leaves_layouts_that_mix_copies_to_separate_jobs() {
    let xml = ticket_xml(&Job { duplex: Some(Duplex::LongEdge), ..Job::default() }, false);
    assert_eq!(one_job_sides(&xml), Some(true));
    assert_eq!(one_job_sides(&xml.replace("JobDuplexAllDocumentsContiguously", "DocumentDuplex")), Some(true));
    assert_eq!(one_job_sides(&ticket_xml(&Job { duplex: Some(Duplex::Off), ..Job::default() }, false)), Some(false));
    assert_eq!(one_job_sides(&ticket_xml(&Job::default(), false)), Some(false), "no duplex feature");
    assert_eq!(one_job_sides(&xml.replace("psk:TwoSidedLongEdge", "vendor:Duplex")), None, "an option it can't read");
    let with = |feature: &str| xml.replace("</psf:PrintTicket>", &format!("{feature}</psf:PrintTicket>"));
    let n_up = |n: u32| {
        with(&format!(
            r#"<psf:Feature name="psk:JobNUpAllDocumentsContiguously"><psf:Option><psf:ScoredProperty name="psk:PagesPerSheet"><psf:Value xsi:type="xsd:integer">{n}</psf:Value></psf:ScoredProperty></psf:Option></psf:Feature>"#
        ))
    };
    assert_eq!(one_job_sides(&n_up(1)), Some(true));
    assert_eq!(one_job_sides(&n_up(2)), None);
    for feature in ["JobBindAllDocuments", "JobStapleAllDocuments", "DocumentStaple"] {
        let finishing = |option: &str| with(&format!(r#"<psf:Feature name="psk:{feature}"><psf:Option name="psk:{option}"/></psf:Feature>"#));
        assert_eq!(one_job_sides(&finishing("None")), Some(true), "{feature}");
        assert_eq!(one_job_sides(&finishing("StapleTopLeft")), None, "{feature}");
    }
}
