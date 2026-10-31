#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Behaviour tests of `security`.

use super::*;

const TOKEN: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn names() -> (crate::args::FlagNames, crate::args::EnvNames) {
    let flags = crate::args::FlagNames { port: "--control".into(), token: "--control-token".into(), token_file: "--control-token-file".into() };
    let env = crate::args::EnvNames {
        port: "PHOTOCRAFT_CONTROL_PORT".into(),
        token: "PHOTOCRAFT_CONTROL_TOKEN".into(),
        token_file: "PHOTOCRAFT_CONTROL_TOKEN_FILE".into(),
    };
    (flags, env)
}

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("craft-automation-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// A connection's first line must be a matching `auth`; anything else is refused and closes.
#[test]
fn first_line_must_authenticate() {
    let auth = |token: &str| format!(r#"{{"id": 7, "method": "auth", "params": {{"token": "{token}"}}}}"#);
    let refused = |id: Value| serde_json::json!({"id": id, "ok": false, "error": "authentication required"});

    let (reply, ok) = authentication_reply(&auth(TOKEN), TOKEN);
    assert!(ok);
    assert_eq!(reply, serde_json::json!({"id": 7, "ok": true, "result": {"authenticated": true}}));

    // The server's copy is lower-cased; a client sends the token as configured.
    assert!(authentication_reply(&auth(&TOKEN.to_uppercase()), TOKEN).1);

    let wrong = TOKEN.replace('0', "1");
    assert_eq!(authentication_reply(&auth(&wrong), TOKEN), (refused(7.into()), false));
    assert_eq!(authentication_reply(&auth("short"), TOKEN), (refused(7.into()), false));
    let other_method = format!(r#"{{"id": 8, "method": "ui.click", "params": {{"token": "{TOKEN}"}}}}"#);
    assert_eq!(authentication_reply(&other_method, TOKEN), (refused(8.into()), false));
    assert_eq!(authentication_reply("not json", TOKEN), (refused(Value::Null), false));

    assert_eq!(repeat_auth_reply(&auth(TOKEN)).unwrap(), serde_json::json!({"id": 7, "ok": false, "error": "already authenticated"}));
    assert_eq!(repeat_auth_reply(r#"{"id": 1, "method": "ui.click"}"#), None);
    assert_eq!(repeat_auth_reply("not json"), None);
}

// Tokens are 64 lower-case hex; a file token is trimmed; both supplied is a conflict.
#[test]
fn server_token_sources_and_validation() {
    let generated = generate_token().unwrap();
    validate_token(&generated).unwrap();
    assert_ne!(generated, generate_token().unwrap());
    assert!(generated.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));

    let bad = validate_token(&TOKEN[1..]).unwrap_err();
    assert_eq!(bad.to_string(), "control token must contain exactly 64 hexadecimal characters");
    assert!(validate_token(&format!("{}g", &TOKEN[1..])).is_err());

    assert_eq!(server_token(Some(&TOKEN.to_uppercase()), None).unwrap(), TOKEN);
    let dir = scratch_dir("token");
    let file = dir.join("nested").join("token");
    let both = server_token(Some(TOKEN), Some(&file)).unwrap_err();
    assert_eq!(both.to_string(), "use either a control token or a control token file, not both");

    // A missing file is created with the generated token, readable by its owner only.
    let created = server_token(None, Some(&file)).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), format!("{created}\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
    }
    assert_eq!(server_token(None, Some(&file)).unwrap(), created);

    // An existing file's content is trimmed and lower-cased.
    std::fs::write(&file, format!("  {}\n\n", TOKEN.to_uppercase())).unwrap();
    assert_eq!(server_token(None, Some(&file)).unwrap(), TOKEN);
    let (flags, env) = names();
    assert_eq!(client_token(None, Some(&file), &flags, &env).unwrap(), TOKEN);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        let err = server_token(None, Some(&file)).unwrap_err().to_string();
        assert!(err.starts_with("I/O: ") && err.contains("readable by group or others (mode 640)"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        client_token(None, None, &flags, &env).unwrap_err().to_string(),
        "bridge mode needs --control-token, --control-token-file, PHOTOCRAFT_CONTROL_TOKEN, or PHOTOCRAFT_CONTROL_TOKEN_FILE"
    );
}

// A line over MAX_REQUEST_BYTES is TooLong and the rest is discarded.
#[test]
fn bounded_line_read_stops_at_the_limit() {
    let mut input = vec![b'a'; MAX_REQUEST_BYTES + 100];
    input.extend_from_slice(b"\nnext\n");
    let mut reader = std::io::BufReader::new(&input[..]);
    let mut line = String::new();

    assert_eq!(read_bounded_line(&mut reader, &mut line).unwrap(), LineRead::TooLong);
    assert_eq!(line.len(), MAX_REQUEST_BYTES + 1);
    discard_rest_of_line(&mut reader).unwrap();
    assert_eq!(read_bounded_line(&mut reader, &mut line).unwrap(), LineRead::Line);
    assert_eq!(line, "next\n");
    assert_eq!(read_bounded_line(&mut reader, &mut line).unwrap(), LineRead::Eof);

    let mut invalid = std::io::BufReader::new(&b"\xff\xfe\nok\n"[..]);
    assert_eq!(read_bounded_line(&mut invalid, &mut line).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(read_bounded_line(&mut invalid, &mut line).unwrap(), LineRead::Line);
}
