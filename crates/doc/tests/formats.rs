//! Each format's documents, built from scratch: what `sniff`, `info`, `renditions` and `outline`
//! return for them.

use super::common;

use common::*;
use craft_doc::{Limits, RenditionKind, info, outline, renditions};

fn limits() -> Limits {
    Limits::default()
}

#[test]
fn vectorcraft_members_after_the_json() {
    // Version 4: the PDF after the marker, behind an image the document uses; the preview inline.
    let image = vec![0xAB; 9000];
    let preview = png(3, 3, 1);
    let members = format!(r#""preview":{},"pdf":{}"#, inline("image/png", &preview), blob("application/pdf", image.len(), PDF.len()));
    let mut file = vectorcraft_json(4, &members).into_bytes();
    file.extend(BLOB_MAGIC);
    file.extend(&image);
    file.extend(PDF);
    for bytes in [file.clone(), gzip(&file)] {
        let r = renditions(&bytes, &limits()).unwrap();
        assert_eq!(r.iter().map(|r| r.kind).collect::<Vec<_>>(), [RenditionKind::Thumbnail, RenditionKind::Pdf]);
        assert_eq!(r[0].bytes, preview);
        assert_eq!(r[1].bytes, PDF);
        // The JSON alone answers info and outline.
        assert_eq!(info(&bytes, &limits()).unwrap().version, 4);
        assert_eq!(outline(&bytes, &limits()).unwrap().pages.len(), 2);
    }
}

#[test]
fn a_compressed_marker_across_the_reader_s_64_kib_chunks() {
    // The marker starts at every offset around the first 64 KiB boundary of the unpacked stream.
    let base = vectorcraft_json(4, &format!(r#""pdf":{},"pad":"""#, blob("application/pdf", 0, PDF.len())));
    for json_len in (64 << 10) - 12..(64 << 10) + 2 {
        let json = base.replacen(r#""pad":"""#, &format!(r#""pad":"{}""#, "x".repeat(json_len - base.len())), 1);
        assert_eq!(json.len(), json_len);
        let mut file = json.into_bytes();
        file.extend(BLOB_MAGIC);
        file.extend(PDF);
        let gz = gzip(&file);
        assert_eq!(info(&gz, &limits()).unwrap().version, 4, "{json_len}");
        assert_eq!(outline(&gz, &limits()).unwrap().pages.len(), 2, "{json_len}");
        assert_eq!(renditions(&gz, &limits()).unwrap()[0].bytes, PDF, "{json_len}");
    }
}
