#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Behaviour tests of `args`.

use super::*;

fn names(prefix: &str) -> EnvNames {
    EnvNames::for_app(prefix)
}

fn flag_port(port: u16) -> ControlOptions {
    ControlOptions { port: Some(port), sources: Sources { port: Some(Source::Flag), ..Sources::default() }, ..ControlOptions::default() }
}

// A flag wins over its variable; a blank variable is unset; a bad port names the variable.
#[test]
fn variables_are_the_flags_fallbacks() {
    let env_names = names("APP");
    let vars = |port: &'static str| {
        let (p, t) = (env_names.port.clone(), env_names.token.clone());
        move |name: &str| match name {
            n if n == p => Some(port.into()),
            n if n == t => Some("   ".into()),
            _ => None,
        }
    };
    let env = ControlOptions::from_lookup(&env_names, vars(" 7000 ")).unwrap();
    assert_eq!(env.port, Some(7000));
    assert_eq!(env.sources.port, Some(Source::Env));
    assert_eq!(env.token, None, "a blank variable is unset");

    let merged = ControlOptions::merged(flag_port(0), env.clone());
    assert_eq!((merged.port, merged.sources.port), (Some(0), Some(Source::Flag)));
    let merged = ControlOptions::merged(ControlOptions::default(), env);
    assert_eq!((merged.port, merged.sources.port), (Some(7000), Some(Source::Env)));

    let error = ControlOptions::from_lookup(&env_names, vars("nope")).unwrap_err();
    assert_eq!(error, OptionsError::NotAPort { value: "nope".into(), shown: env_names.port.clone() });
    assert_eq!(error.to_string(), "invalid value 'nope' for 'APP_CONTROL_PORT': expected a port, 0-65535");
    assert_eq!(control_port("65536"), Err(NotAPort));
    assert_eq!(control_port(" 65535 "), Ok(65535));
}

// token + token file (either from a flag or a variable) is a conflict that never shows the token.
#[test]
fn token_and_token_file_conflict_names_the_sources() {
    let (flags, env_names) = (FlagNames::default(), names("PHOTOCRAFT"));
    let secret = "secret-token-value";
    let both = |token: Source, file: Source| ControlOptions {
        port: Some(0),
        token: Some(secret.into()),
        token_file: Some("t".into()),
        sources: Sources { port: Some(Source::Flag), token: Some(token), token_file: Some(file) },
    };

    let error = both(Source::Flag, Source::Flag).check(&flags, &env_names).unwrap_err();
    assert_eq!(error.to_string(), "the argument '--control-token' cannot be used with '--control-token-file'");
    let error = both(Source::Env, Source::Flag).check(&flags, &env_names).unwrap_err();
    assert_eq!(error.to_string(), "the argument 'PHOTOCRAFT_CONTROL_TOKEN' cannot be used with '--control-token-file'");
    let error = both(Source::Flag, Source::Env).check(&flags, &env_names).unwrap_err();
    assert_eq!(error.to_string(), "the argument '--control-token' cannot be used with 'PHOTOCRAFT_CONTROL_TOKEN_FILE'");
    assert!(!error.to_string().contains(secret));

    let mut no_server = both(Source::Flag, Source::Flag);
    no_server.port = None;
    assert_eq!(no_server.check(&flags, &env_names), Ok(()), "the token pair is ignored without --control");
}

// A bad variable is not read when its flag is given, as with clap.
#[test]
fn a_bad_variable_is_ignored_when_its_flag_is_given() {
    let env_names = names("APP");
    let bad = |name: &str| (name == env_names.port).then(|| "nope".into());
    let resolved = ControlOptions::resolve_with(flag_port(0), &env_names, bad).unwrap();
    assert_eq!((resolved.port, resolved.sources.port), (Some(0), Some(Source::Flag)));
    assert!(ControlOptions::resolve_with(ControlOptions::default(), &env_names, bad).is_err());
}

// A non-Unicode port or token variable is a usage error; a token file path may be any bytes.
#[cfg(unix)]
#[test]
fn non_unicode_variables_are_reported() {
    use std::os::unix::ffi::OsStringExt;
    let env_names = names("APP");
    let bytes = || std::ffi::OsString::from_vec(vec![0x66, 0xff]);
    let only = |var: String| move |name: &str| (name == var).then(bytes);
    let error = ControlOptions::from_lookup(&env_names, only(env_names.token.clone())).unwrap_err();
    assert_eq!(error, OptionsError::NotUnicode { shown: "APP_CONTROL_TOKEN".into() });
    let error = ControlOptions::from_lookup(&env_names, only(env_names.port.clone())).unwrap_err();
    assert_eq!(error.to_string(), "invalid UTF-8 was detected in 'APP_CONTROL_PORT'");
    let file = ControlOptions::from_lookup(&env_names, only(env_names.token_file.clone())).unwrap();
    assert_eq!(file.token_file, Some(PathBuf::from(bytes())));
}
