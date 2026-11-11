use super::*;
use craft_ui::testing::trial;

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

/// The ticket Windows returns (`ns0000`) gets the driver's own `brpsk` back, in the declaration, the names and the QName values, but not in text.
#[trial]
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

/// When two namespaces trade prefixes both prefixes stay declared, so a name that is only
/// written with a prefix (an element or an attribute, unlike a `name` value) would still parse and
/// resolve, but to the other namespace. Each ticket here is well-formed and has every prefix
/// declared, so `respell` leaves it alone for the name, not because the result didn't parse.
#[trial]
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
