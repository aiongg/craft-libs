//! Which link targets from a document may become clickable links in a written PDF.

/// Whether a link target from a (possibly untrusted) document may be opened by whoever opens
/// the file: `http`/`https` URLs with a host, and `mailto` with an address.
///
/// Everything else is refused: `file:`, `javascript:`, `data:`, relative paths, network
/// shares (`\\host\share`, `//host/share`, `smb:`) and other schemes, which an operating system
/// may hand to a protocol handler. Leading and trailing whitespace is ignored; schemes compare
/// case-insensitively.
pub fn safe_url(url: &str) -> bool {
    let u = url.trim();
    let Some((scheme, rest)) = u.split_once(':') else { return false };
    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => rest.starts_with("//") && rest.len() > 2,
        "mailto" => !rest.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::safe_url;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn only_web_and_mail_links_are_safe_to_open() {
        for url in ["https://example.com", " HTTP://example.com/x", "mailto:someone@example.com"] {
            assert!(safe_url(url), "{url}");
        }
        let refused = [
            "https://",
            "mailto:",
            "file:///C:/notes.pdf",
            "notes.pdf",
            "javascript:alert(1)",
            "\t JavaScript:alert(1)",
            "java\nscript:alert(1)",
            "vbscript:MsgBox(1)",
            "data:text/html,<script>alert(1)</script>",
            "ms-msdt:/id PCWDiagnostic",
            "ms-word:ofe|u|\\\\attacker.example\\share\\x.docx",
            "ldap://attacker.example/x",
            "//attacker.example/share",
            "\\\\attacker.example\\share",
            "file://attacker.example/share/x",
            "file:////attacker.example/share/x",
            "smb://attacker.example/share",
            "search-ms:query=x&crumb=location:\\\\attacker.example\\share",
            "search:query=x&crumb=location:\\\\attacker.example\\share",
        ];
        for url in refused {
            assert!(!safe_url(url), "{url}");
        }
    }
}
