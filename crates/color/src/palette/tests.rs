//! Tests for the swatch file codecs.

use super::*;

fn values(c: &SwatchColor) -> Vec<f32> {
    match *c {
        SwatchColor::Rgb(v) | SwatchColor::Hsb(v) | SwatchColor::Lab(v) => v.to_vec(),
        SwatchColor::Cmyk(v) => v.to_vec(),
        SwatchColor::Gray(k) => vec![k],
        SwatchColor::Book { .. } => Vec::new(),
    }
}

fn assert_same(a: &SwatchColor, b: &SwatchColor) {
    assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b), "{a:?} vs {b:?}");
    if let (SwatchColor::Book { .. }, SwatchColor::Book { .. }) = (a, b) {
        assert_eq!(a, b);
    }
    for (x, y) in values(a).into_iter().zip(values(b)) {
        assert!((x - y).abs() < 1e-3, "{a:?} vs {b:?}");
    }
}

fn group(name: &str, entries: Vec<SwatchEntry>) -> SwatchGroup {
    SwatchGroup { name: name.into(), entries }
}

/// An `.aco` file with RGB, CMYK and Lab swatches round-trips with each colour in its model.
#[test]
fn aco_round_trips_colours_in_their_own_model() {
    let colors = [
        ("Red", SwatchColor::Rgb([1.0, 0.0, 0.25])),
        ("Hue", SwatchColor::Hsb([120.0, 0.5, 0.75])),
        ("Cyan ink", SwatchColor::Cmyk([1.0, 0.0, 0.25, 0.1])),
        ("Lab", SwatchColor::Lab([53.0, -80.0, 67.5])),
        ("Gray", SwatchColor::Gray(0.4)),
        ("Book 日本", SwatchColor::Book { space: 3, values: [1, 2, 3, 4] }),
    ];
    let entries = colors.iter().map(|(n, c)| SwatchEntry::new(*n, *c)).collect();
    let (bytes, report) = write_swatches(&[group("All", entries)], SwatchFormat::Aco).unwrap();
    assert!(report.notes.is_empty(), "{report:?}");
    let (groups, report) = read_swatches(&bytes, "Brand.aco").unwrap();
    assert_eq!(report.notes, [SwatchNote::BookColors(1)]);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Brand");
    for ((name, color), entry) in colors.iter().zip(&groups[0].entries) {
        assert_eq!(entry.name, *name);
        assert_same(color, &entry.color);
    }
    // A version 1 section comes first, then version 2 with the names.
    assert_eq!(&bytes[..4], &[0, 1, 0, 6]);
    assert!(bytes.windows(4).any(|w| w == [0, 2, 0, 6]));
}

/// `.ase` keeps groups and spot colours; `.aco` flattens them and says so.
#[test]
fn ase_keeps_groups_and_aco_reports_flattening() {
    let mut spot = SwatchEntry::new("Spot Orange", SwatchColor::Cmyk([0.0, 0.5, 1.0, 0.0]));
    spot.kind = SwatchKind::Spot;
    let groups = vec![
        group("Brand", vec![SwatchEntry::new("Blue", SwatchColor::Rgb([0.0, 0.0, 1.0])), spot]),
        group("Greys", vec![SwatchEntry::new("Mid", SwatchColor::Gray(0.5))]),
    ];
    let (ase, report) = write_swatches(&groups, SwatchFormat::Ase).unwrap();
    assert!(report.notes.is_empty());
    let (back, _) = read_swatches(&ase, "x.ase").unwrap();
    assert_eq!(back.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Brand", "Greys"]);
    assert_eq!(back[0].entries[1].kind, SwatchKind::Spot);
    assert_same(&back[0].entries[1].color, &groups[0].entries[1].color);
    assert_same(&back[1].entries[0].color, &SwatchColor::Gray(0.5));

    // A colour in a model this crate does not read is skipped and counted.
    let mut unknown = Vec::new();
    put_ase_name(&mut unknown, "Odd");
    unknown.extend(b"XYZ ");
    unknown.extend([0u8; 12]);
    unknown.extend(2u16.to_be_bytes());
    let mut file = ase.clone();
    put_ase_block(&mut file, ASE_COLOR, &unknown);
    file[8..12].copy_from_slice(&(u32::from_be_bytes(ase[8..12].try_into().unwrap()) + 1).to_be_bytes());
    let (skipped, report) = read_swatches(&file, "x.ase").unwrap();
    assert_eq!(report.notes, [SwatchNote::Skipped(1)]);
    assert_eq!(count_swatches(&skipped), 3);

    let (aco, report) = write_swatches(&groups, SwatchFormat::Aco).unwrap();
    assert_eq!(report.notes, [SwatchNote::GroupsFlattened, SwatchNote::KindsDropped(1)]);
    let (flat, _) = read_swatches(&aco, "x.aco").unwrap();
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0].entries.len(), 3);
}

/// A `.gpl` palette with `# Group:` headers reads as groups; a nameless line is "Swatch N".
#[test]
fn gpl_reads_group_headers_and_fills_missing_names() {
    let text = "GIMP Palette\nName: Test\nColumns: 0\n#\n255   0   0\tRed\n# Group: Cool\n  0   0 255\n  0 255 255\tAqua\n";
    let (groups, _) = read_swatches(text.as_bytes(), "Test.gpl").unwrap();
    assert_eq!(groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Test", "Cool"]);
    assert_eq!(groups[0].entries[0].name, "Red");
    assert_eq!(groups[1].entries[0].name, "Swatch 2");
    assert_eq!(groups[1].entries[1].name, "Aqua");
    assert_same(&groups[1].entries[0].color, &SwatchColor::Rgb([0.0, 0.0, 1.0]));

    let (written, report) = write_swatches(&groups, SwatchFormat::Gpl).unwrap();
    assert!(report.notes.is_empty());
    let (again, _) = read_swatches(&written, "Test.gpl").unwrap();
    assert_eq!(again, groups);
}

/// The format comes from the content, whatever the extension; `.act` only from the name.
#[test]
fn the_format_is_sniffed_from_content_and_act_from_the_name() {
    let entries = vec![SwatchEntry::new("A", SwatchColor::Rgb([1.0, 0.0, 0.0]))];
    let groups = [group("G", entries)];
    for format in [SwatchFormat::Aco, SwatchFormat::Ase, SwatchFormat::Gpl] {
        let (bytes, _) = write_swatches(&groups, format).unwrap();
        assert_eq!(SwatchFormat::sniff(&bytes, "mystery.bin"), Some(format));
        assert_eq!(SwatchFormat::sniff(&bytes, "wrong.css"), Some(format));
    }
    // 768 bytes starting `0, 1, x`: only the name makes it an `.act`.
    let mut act = vec![0u8; ACT_PALETTE_LEN];
    act[1] = 1;
    assert_eq!(SwatchFormat::sniff(&act, "table.ACT"), Some(SwatchFormat::Act));
    assert_eq!(SwatchFormat::sniff(&act, "table.bin"), None);
    let (groups, _) = read_swatches(&act, "table.act").unwrap();
    assert_eq!(groups[0].entries.len(), 256);
    let (written, _) = write_swatches(&groups, SwatchFormat::Act).unwrap();
    assert_eq!(written.len(), 772);
    assert_eq!(SwatchFormat::from_name("a/b.Gpl"), Some(SwatchFormat::Gpl));
    assert_eq!(SwatchFormat::from_name("noext"), None);
    assert_eq!(read_swatches(b"hello", "x.txt"), Err(SwatchError::UnknownFormat));
}

/// CSS is written with escaped property names and cannot be read.
#[test]
fn css_is_written_with_escaped_names_and_not_read() {
    let mut spot = SwatchEntry::new("Brand Blue #2", SwatchColor::Rgb([0.0, 0.0, 1.0]));
    spot.kind = SwatchKind::Spot;
    let groups = [group("Brand", vec![spot, SwatchEntry::new("brand blue #2", SwatchColor::Rgb([1.0, 0.0, 0.0]))])];
    let (bytes, report) = write_swatches(&groups, SwatchFormat::Css).unwrap();
    assert!(report.notes.is_empty());
    let css = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(css, ":root {\n  /* Brand */\n  --brand-blue-\\#2: #0000ff; /* spot */\n  --brand-blue-\\#2-2: #ff0000;\n}\n");
    assert_eq!(read_swatches(&bytes, "x.css"), Err(SwatchError::CannotRead(SwatchFormat::Css)));
    assert!(!SwatchFormat::Css.can_read());
    assert_eq!(write_swatches(&[], SwatchFormat::Css), Err(SwatchError::Empty));
}

/// Each model draws as sRGB; CMYK goes through the converter.
#[test]
fn swatch_colours_draw_in_srgb() {
    let cmyk = CmykConverter::coated();
    let near = |c: Srgba, rgb: [f32; 3], tol: f32| (c.r - rgb[0]).abs() <= tol && (c.g - rgb[1]).abs() <= tol && (c.b - rgb[2]).abs() <= tol;
    assert!(near(SwatchColor::Hsb([120.0, 1.0, 1.0]).to_srgb(cmyk), [0.0, 1.0, 0.0], 1e-6));
    assert!(near(SwatchColor::Lab([100.0, 0.0, 0.0]).to_srgb(cmyk), [1.0; 3], 0.01));
    assert!(near(SwatchColor::Gray(1.0).to_srgb(cmyk), [0.0; 3], 1e-6));
    assert!(near(SwatchColor::Book { space: 3, values: [0; 4] }.to_srgb(cmyk), [0.0; 3], 0.0));
    assert!(near(SwatchColor::Cmyk([0.0; 4]).to_srgb(cmyk), [1.0; 3], 0.03));
    let cyan = SwatchColor::Cmyk([1.0, 0.0, 0.0, 0.0]).to_srgb(cmyk);
    assert!(cyan.r < 0.3 && cyan.g > cyan.r && cyan.b > cyan.r, "{cyan:?}");
    assert_eq!(default_swatches().iter().map(|g| g.entries.len()).sum::<usize>(), 46);
}
