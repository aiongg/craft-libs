//! Unit tests for the affix dictionary.

use super::*;

const AFF: &str = "SET UTF-8\nTRY aeoiułkr\n\nPFX b Y 1\nPFX b   0   nie   .\n\nSFX a Y 3\nSFX a   0   y     [^y]\nSFX a   ek  ka    ek\nSFX a   0   em    .\n\nSFX c N 1\nSFX c   a   ą     a\n\nMAP 2\nMAP aą\nMAP oóu\nREP 2\nREP ż rz\nREP u ó\n";
const DIC: &str = "4\nkot/ab\npiesek/a\nkrowa/c\nWarszawa/c\n";

fn dict() -> AffixDict {
    AffixDict::from_hunspell(AFF, DIC, &["test data"]).unwrap()
}

/// #495 review: the stems' flags are counted as they are read, so a `.dic` whose flags pass
/// the budget is refused while reading (its flag text) or while its flags are read (the
/// keys), before they are all held; one within it loads.
#[test]
fn stem_flags_are_budgeted_while_reading() {
    let aff = "SET UTF-8\nSFX A Y 1\nSFX A 0 s .\nSFX B Y 1\nSFX B 0 x .\n";
    let many: String = (0..100).map(|i| format!("w{i}/AB\n")).collect();
    let dic = format!("100\n{many}");
    // 200 flags: over a budget of 50 keys (whose flag text, 300 bytes, still fits).
    let e = AffixDict::from_hunspell_capped(aff, &dic, &[], false, 50).unwrap_err();
    assert!(e.contains("more than 50 flags"), "{e}");
    let d = AffixDict::from_hunspell_capped(aff, &dic, &[], false, 200).unwrap();
    assert!(d.check("w7s") && d.check("w99x"));
    // Flag text past the budget (entries over the per-entry cap hold no keys, but their text
    // still counts): refused while reading the `.dic`.
    let long = "A".repeat(MAX_FLAGS_PER_ENTRY + 1);
    let dic = format!("3\na/{long}\nb/{long}\nc/{long}\n");
    let e = AffixDict::from_hunspell_capped(aff, &dic, &[], false, 2000).unwrap_err();
    assert!(e.contains("more than 12000 bytes"), "{e}");
    assert!(AffixDict::from_hunspell(aff, &dic, &[]).is_ok(), "within the real budget: skipped entries, no error");
    let mut text = FlagText::new(1);
    assert!(text.add("ABCDEF").is_ok() && text.add("G").is_err());
}

#[test]
fn affix_headers_allow_comments_but_rule_conditions_stay_literal() {
    let aff = "SET UTF-8\nPFX X Y 1 # prefix comment\nPFX X 0 ne . # rule comment\nSFX y Y 1 # suffix comment\nSFX y 0 y . # rule comment\n";
    let d = AffixDict::from_hunspell(aff, "1\nstrom/Xy\n", &[]).unwrap();
    assert_eq!(d.rule_count(), 2, "headers must not become affix rules");
    for w in ["strom", "stromy", "nestrom", "nestromy"] {
        assert!(d.contains(w), "{w}");
    }
    for w in ["stromY1", "stromN1", "X", "y"] {
        assert!(!d.contains(w), "{w}");
    }
    let hash = AffixDict::from_hunspell("SFX H Y 1\nSFX H 0 # .\nTRY a#b\nKEY a#b\n", "1\nstrom/H\n", &[]).unwrap();
    assert!(hash.contains("strom#"), "a hash in an affix field is literal");
    let literal =
        AffixDict::from_hunspell("PFX P Y 1 # header\nPFX P 0 pre #\nSFX S Y 1 # header\nSFX S 0 post #\n", "3\n#stem/P\nstem#/S\nstem/PS\n", &[])
            .unwrap();
    assert!(literal.contains("pre#stem") && literal.contains("stem#post"));
    assert!(!literal.contains("prestem") && !literal.contains("stempost"), "# must remain a literal condition");

    assert_eq!(hash.try_chars(), "a#b");
    assert_eq!(hash.key(), "a#b");
    assert_eq!(AffixDict::from_bytes(&hash.to_bytes().unwrap()).unwrap().key(), "a#b");
}

#[test]
fn try_alphabet_is_capped_and_deduplicated() {
    let aff = format!("TRY {}\n", "abcdefghijklmnopqrstuvwxyz".repeat(10_000));
    let d = AffixDict::from_hunspell(&aff, "1\nkot\n", &[]).unwrap();
    assert_eq!(d.try_chars(), "abcdefghijklmnopqrstuvwxyz");
    let compact = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(compact.try_chars(), "abcdefghijklmnopqrstuvwxyz");

    // More distinct characters than the cap, each repeated: keep the first 128 in order.
    let many: String = (0x100u32..0x100 + 300).filter_map(char::from_u32).collect();
    let aff = format!("TRY {many}{many}\n");
    let want: String = many.chars().take(128).collect();
    let d = AffixDict::from_hunspell(&aff, "1\nkot\n", &[]).unwrap();
    assert_eq!(d.try_chars(), want);
    let compact = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(compact.try_chars(), want);

    // An oversized TRY line fed straight to the compact reader is capped as well.
    let raw = format!("{MAGIC}\nTRY {many}{many}\nWORDS\n\0kot\n");
    let bytes = miniz_oxide::deflate::compress_to_vec(raw.as_bytes(), 6);
    assert_eq!(AffixDict::from_bytes(&bytes).unwrap().try_chars(), want);
}

/// Shared hand-written fixture: collectors must reject every path to NOSUGGEST forms.
pub(crate) fn assert_nosuggest_is_honoured(suggest: impl Fn(&AffixDict, &str, usize) -> Vec<String>) {
    let aff = "SET UTF-8\nNOSUGGEST N\nFORBIDDENWORD F\nTRY abeklmnostxy\nMAP 1\nMAP ae\nKEY ak\nREP 2\nREP xy toka\nREP zz normal\nPFX X Y 1\nPFX X 0 pre .\nSFX A Y 1\nSFX A 0 ble/B .\nSFX B Y 1\nSFX B 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "4\ntoka/NXA\nnormal/XA\nTkname/N\nbanned/F\n", &[]).unwrap();
    for word in d.forms("toka").into_iter().chain(["TOKA".into(), "Toka".into(), "Tkname".into(), "TKNAME".into()]) {
        assert!(d.check(&word), "NOSUGGEST is still correct: {word}");
        assert!(!d.can_suggest(&word), "NOSUGGEST form: {word}");
    }
    for input in ["tokax", "tokablesx", "pretokax", "pretokablesx", "toka", "TOKA", "tkname", "teka", "xy", "normaltoka", "toak", "taak"] {
        let results = suggest(&d, input, 32);
        assert!(results.iter().all(|word| word.split(' ').all(|part| d.can_suggest(part))), "{input}: {results:?}");
        assert!(results.iter().all(|word| !word.to_lowercase().contains("toka") && word.to_lowercase() != "tkname"), "{input}: {results:?}");
    }
    assert!(suggest(&d, "norrmal", 8).contains(&"normal".into()));
    assert!(suggest(&d, "zz", 8).contains(&"normal".into()));
    // Filtering happens before display casing: ordinary mixed-case stems retain the
    // existing all-caps suggestion behaviour, with and without a NOSUGGEST declaration.
    for aff in ["", "NOSUGGEST N\n"] {
        let ordinary = AffixDict::from_hunspell(aff, "2\nmixCase\nhidden/N\n", &[]).unwrap();
        assert!(suggest(&ordinary, "MIXCASX", 8).contains(&"MIXCASE".into()));
    }
}

#[test]
fn nosuggest_is_correct_but_not_suggestible_and_survives_compact_format() {
    let d = AffixDict::from_hunspell("NOSUGGEST N\nSFX S Y 1\nSFX S 0 s .\n", "2\nsecret/NS\nnormal/S\n", &[]).unwrap();
    for dict in [&d, &AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap()] {
        for word in ["secret", "secrets", "Secret", "SECRETS"] {
            assert!(dict.check(word), "{word}");
            assert!(!dict.can_suggest(word), "{word}");
        }
        assert!(dict.contains_suggestible("normal") && dict.can_suggest("NORMALS"));
    }
    assert!(AffixDict::from_hunspell("NOSUGGEST NN\n", "1\nword\n", &[]).is_err());
}

#[test]
fn stems_suffixes_prefixes_and_cross_products() {
    let d = dict();
    assert_eq!(d.len(), 4);
    for w in ["kot", "koty", "kotem", "piesek", "pieska", "piesekem", "krowa", "krową", "niekot", "niekoty", "niekotem", "Warszawa", "Warszawą"] {
        assert!(d.contains(w), "{w}");
    }
    for w in ["kotą", "pieseka", "pieskem", "niepiesek", "niekrową", "krowy", "warszawa", "", "nie", "y", "ka"] {
        assert!(!d.contains(w), "{w}");
    }
}

#[test]
fn case_variants() {
    let d = dict();
    assert!(d.check("Kot") && d.check("KOT") && d.check("Koty") && d.check("WARSZAWĄ") && d.check("Warszawą"));
    assert!(!d.check("warszawa") && !d.check("kOT") && !d.check("KoT"));
    assert_eq!(casing("Kot"), Casing::Initial);
    assert_eq!(casing("KOT"), Casing::Upper);
    assert_eq!(casing("K"), Casing::Initial);
    assert_eq!(casing("kot"), Casing::Lower);
    assert_eq!(casing("McDonald"), Casing::Mixed);
    assert_eq!(capitalize("żółw"), "Żółw");
}

#[test]
fn forms_are_all_accepted() {
    let d = dict();
    let mut forms = d.forms("kot");
    forms.sort();
    forms.dedup();
    assert_eq!(forms, ["kot", "kotem", "koty", "niekot", "niekotem", "niekoty"]);
    for stem in d.stems().map(str::to_string).collect::<Vec<_>>() {
        for f in d.forms(&stem) {
            assert!(d.contains(&f), "{stem} → {f}");
        }
    }
}

#[test]
fn compact_format_round_trips() {
    let d = dict();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(back.len(), d.len());
    assert_eq!(back.rule_count(), d.rule_count());
    assert_eq!(back.comments(), ["test data"]);
    assert_eq!(back.rep(), d.rep());
    assert_eq!(back.map(), d.map());
    assert_eq!(back.try_chars(), "aeoiułkr");
    for w in ["kotem", "niekoty", "pieska", "krową", "Warszawą"] {
        assert!(back.contains(w), "{w}");
    }
    assert!(!back.contains("kotą"));
}

#[test]
fn flag_utf8_and_default_modes_are_accepted() {
    // `FLAG UTF-8` (VERO pt_BR) and `FLAG DEFAULT` are per-character flag modes: the same
    // mode the reader assumes without a FLAG line.
    for mode in ["UTF-8", "DEFAULT"] {
        let aff = format!("SET UTF-8\nFLAG {mode}\nSFX a Y 1\nSFX a   0   y   [^y]\n");
        let d = AffixDict::from_hunspell(&aff, "1\nkot/a\n", &[]).unwrap();
        assert!(d.contains("kot") && d.contains("koty"), "{mode}");
    }
    assert!(AffixDict::from_hunspell("FLAG UTF-8\n", "0\n", &[]).is_ok(), "FLAG alone is harmless");
}

#[test]
fn emoji_flags_with_a_variation_selector_load() {
    // RLA-ES (the Spanish dictionaries in LibreOffice and most Linux distributions) writes its
    // telephone flag as ☎ followed by U+FE0F, in the rule and on the stems.
    let aff = "SET UTF-8\nFLAG UTF-8\nPFX \u{260e}\u{fe0f} Y 1\nPFX \u{260e}\u{fe0f} 0 tele .\nSFX S Y 1\nSFX S 0 s [aeiou]\n";
    let d = AffixDict::from_hunspell(aff, "2\nbanco/S\u{260e}\u{fe0f}\nbasura/S\n", &[]).unwrap();
    assert!(d.contains("banco") && d.contains("bancos"));
    assert!(d.contains("telebanco") && d.contains("telebancos"), "the ☎ prefix applies");
    assert!(d.contains("basuras") && !d.contains("telebasura"), "only to stems that carry it");
    assert!(!d.flag_keys.contains(&FlagKey::from(EMOJI_SELECTOR)), "the selector is not a flag");
}

#[test]
fn emoji_forbiddenword_and_nosuggest_flags_load() {
    // The same selector after the flag of the directives that name one flag.
    let aff = "SET UTF-8\nFLAG UTF-8\nFORBIDDENWORD \u{260e}\u{fe0f}\nNOSUGGEST \u{1f6ab}\u{fe0f}\n";
    let d = AffixDict::from_hunspell(aff, "3\nbanco\nbanko/\u{260e}\u{fe0f}\nbancu/\u{1f6ab}\u{fe0f}\n", &[]).unwrap();
    assert!(d.contains("banco") && d.contains("bancu"));
    assert!(!d.contains("banko"), "a stem with the FORBIDDENWORD flag is not a word");
}

#[test]
fn more_than_64_distinct_flags_load() {
    // VERO pt_BR uses 106 distinct flags (FLAG UTF-8). Build one with every basic Latin
    // letter, the digits and eight accented letters: 70 flags, more than 64.
    let mut flags = String::new();
    let mut aff = String::from("SET UTF-8\nFLAG UTF-8\n");
    let chars: Vec<char> = ('a'..='z').chain('A'..='Z').chain('0'..='9').chain(['á', 'é', 'í', 'ó', 'ú', 'ç', 'ã', 'õ']).collect();
    assert_eq!(chars.len(), 70);
    for c in chars {
        aff.push_str(&format!("SFX {c} N 1\nSFX {c}   0   {c}   .\n"));
        flags.push(c);
    }
    let d = AffixDict::from_hunspell(&aff, &format!("1\nverbo/{flags}\n"), &[]).unwrap();
    assert!(d.contains("verbo"));
    assert!(d.contains("verboa") && d.contains("verboz") && d.contains("verbo9"));
    assert!(d.contains("verboç") && d.contains("verboã"), "multi-byte UTF-8 flags");
    assert!(!d.contains("verboß"), "ß is not a flag and not a stem");
}

#[test]
fn more_than_128_distinct_flags_load() {
    // The Hungarian dictionary uses 158 single-byte flags: 200 flags here, U+0080 up.
    let flags: String = (0x80u32..0x80 + 200).filter_map(char::from_u32).collect();
    let mut aff = String::from("SET UTF-8\n");
    for (i, c) in flags.chars().enumerate() {
        aff.push_str(&format!("SFX {c} N 1\nSFX {c} 0 x{i} .\n"));
    }
    let half: String = flags.chars().step_by(2).collect();
    let d = AffixDict::from_hunspell(&aff, &format!("1\nszó/{half}\n"), &[]).unwrap();
    assert!(d.contains("szóx0") && d.contains("szóx198") && d.contains("szóx130"));
    assert!(!d.contains("szóx1") && !d.contains("szóx199") && !d.contains("szóx131"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.contains("szóx198") && !back.contains("szóx199"));
    // 257 distinct flags load too now (each flag gets a u16 id).
    let many: String = (0x100u32..0x100 + 257).filter_map(char::from_u32).collect();
    assert!(AffixDict::from_hunspell("SET UTF-8\n", &format!("1\nszó/{many}\n"), &[]).is_ok());
}

#[test]
fn forbidden_words_are_rejected() {
    // VERO pt_BR marks spellings it must reject (flag ý under FLAG UTF-8) as stems carrying
    // FORBIDDENWORD: listed, but never correct and never inflected.
    let aff = "SET UTF-8\nFLAG UTF-8\nFORBIDDENWORD ý\nSFX a Y 1\nSFX a   0   y   [^y]\n";
    let d = AffixDict::from_hunspell(aff, "2\nkot/a\nimundo/ý\n", &[]).unwrap();
    assert!(d.check("kot") && d.check("koty") && d.check("Kot") && d.check("KOT"));
    assert!(!d.check("imundo") && !d.check("Imundo") && !d.check("IMUNDO"));
    assert!(!d.contains("imundoy"), "a forbidden stem must not inflect");
    assert!(d.forms("imundo").is_empty());
}

#[test]
fn continuation_classes_chain_suffixes() {
    // `SFX A 0 able/B`: the form derived by A carries flag B, so a suffix of class B may
    // follow (workable → workables). The bare stem has A only, so `works` stays wrong.
    let aff = "SET UTF-8\nFLAG UTF-8\nSFX A N 1\nSFX A   0   able/B   .\nSFX B N 1\nSFX B   0   s   .\n";
    let d = AffixDict::from_hunspell(aff, "1\nwork/A\n", &[]).unwrap();
    assert!(d.contains("work") && d.contains("workable") && d.contains("workables"));
    assert!(!d.contains("works"), "the stem has A, not B");
    assert!(!d.contains("workabless"), "B declares no continuation");
    assert!(d.check("Workable") && d.check("WORKABLES"));
}

#[test]
fn continuation_chains_stop_at_two_suffixes() {
    // Hunspell strips at most two suffixes: work + able/B + s/C is a word, a third (C) is not.
    let aff = "SET UTF-8\nFLAG UTF-8\nSFX A N 1\nSFX A   0   able/B   .\nSFX B N 1\nSFX B   0   s/C   .\nSFX C N 1\nSFX C   0   x   .\n";
    let d = AffixDict::from_hunspell(aff, "1\nwork/A\n", &[]).unwrap();
    assert!(d.contains("workable") && d.contains("workables"));
    assert!(!d.contains("workablesx"), "three chained suffixes exceed the limit");
    assert!(!d.forms("work").iter().any(|f| f == "workablesx"));
}

#[test]
fn forms_include_continuation_chains() {
    let aff = "SET UTF-8\nFLAG UTF-8\nSFX A N 1\nSFX A   0   able/B   .\nSFX B N 1\nSFX B   0   s   .\n";
    let d = AffixDict::from_hunspell(aff, "1\nwork/A\n", &[]).unwrap();
    let mut f = d.forms("work");
    f.sort();
    f.dedup();
    assert_eq!(f, ["work", "workable", "workables"]);
    for form in d.forms("work") {
        assert!(d.contains(&form), "every listed form is accepted: {form}");
    }
}

#[test]
fn continuation_classes_merge_the_flags_of_a_group() {
    // `s` under B and under C share one group (same append, same strip): a form carrying
    // either flag takes it, and the stem carrying neither doesn't.
    let aff = "SET UTF-8\nSFX A N 1\nSFX A 0 able/C .\nSFX D N 1\nSFX D 0 er/B .\nSFX B N 1\nSFX B 0 s .\nSFX C N 1\nSFX C 0 s [e]\n";
    let d = AffixDict::from_hunspell(aff, "2\nwork/A\nplay/D\n", &[]).unwrap();
    assert!(d.contains("workables") && d.contains("players"));
    assert!(!d.contains("works") && !d.contains("plays") && !d.contains("workabless"));
    assert!(d.forms("work").iter().any(|f| f == "workables") && d.forms("play").iter().any(|f| f == "players"));
}

/// `n` suffix rules of flag A on one stem, `rule(i)` giving each rule's strip, append and condition.
fn many_rules(n: usize, rule: impl Fn(usize) -> String) -> AffixDict {
    let mut aff = format!("SET UTF-8\nSFX A Y {n}\n");
    for i in 0..n {
        aff.push_str(&format!("SFX A {}\n", rule(i)));
    }
    AffixDict::from_hunspell(&aff, "1\nkot/A\n", &[]).unwrap()
}

#[test]
fn self_continuing_rules_check_in_linear_time() {
    // Thousands of empty-append rules that continue with their own flag: each check used to
    // follow every rule from every rule, millions of steps a word (#3613). The checks take
    // about a millisecond; the old checker took 14 s for two of them at 16,000 rules, so at
    // 40,000 (quadratic: minutes) the 10 s bound leaves a busy machine a wide margin.
    let d = many_rules(40_000, |_| "0 0/A .".into());
    let t = std::time::Instant::now();
    assert!(d.check("kot") && !d.check("pies") && !d.check("kotx"));
    assert!(t.elapsed().as_secs_f64() < 10.0, "{:?}", t.elapsed());
}

#[test]
fn forms_of_self_continuing_rules_are_bounded() {
    // Every rule's form continues with every rule: millions of strings for one stem (#3613).
    let same = many_rules(3000, |_| "0 0/A .".into());
    let distinct = many_rules(3000, |i| format!("0 x{i}/A ."));
    let t = std::time::Instant::now();
    let (a, b) = (same.forms("kot"), distinct.forms("kot"));
    // The cap on the count is what tells the old code (9 million forms each) apart; the
    // generous time bound (tens of ms here) only backs up the rule-visit cap.
    assert!(t.elapsed().as_secs_f64() < 10.0, "{:?}", t.elapsed());
    assert!(a.len() <= MAX_FORMS && b.len() <= MAX_FORMS, "{} {}", a.len(), b.len());
    assert!(b.iter().any(|f| f == "kotx0x1"), "chained forms are still listed");
}

#[test]
fn many_strips_per_suffix_index_in_linear_time() {
    // One append with thousands of strips: the index used to scan the append's groups for
    // each rule. Building 200,000 rules takes well under a second; the old index took 3 s at
    // 30,000 and over 10 minutes at 300,000.
    let n = 200_000;
    let t = std::time::Instant::now();
    let d = many_rules(n, |i| format!("k{i} y ."));
    assert!(t.elapsed().as_secs_f64() < 10.0, "{:?}", t.elapsed());
    assert_eq!(d.sfx_index.get("y").map(Vec::len), Some(n));
    assert!(!d.contains("koty"));
}

#[test]
fn compact_format_round_trips_continuations_and_forbidden() {
    let aff = "SET UTF-8\nFLAG UTF-8\nFORBIDDENWORD ý\nSFX A N 1\nSFX A   0   able/B   .\nSFX B N 1\nSFX B   0   s   .\n";
    let d = AffixDict::from_hunspell(aff, "2\nwork/A\nimundo/ý\n", &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.contains("work") && back.contains("workable") && back.contains("workables"));
    assert!(!back.contains("works"), "the stem has A, not B, after a round trip");
    assert!(!back.check("imundo") && !back.contains("imundoy"), "forbidden survives a round trip");
}

#[test]
fn dic_bom_glued_to_the_count_line_is_skipped() {
    // VERO's .dic opens with a UTF-8 BOM glued to its count line; it must not become a stem.
    let aff = "SET UTF-8\nSFX a Y 1\nSFX a   0   y   [^y]\n";
    let d = AffixDict::from_hunspell(aff, "\u{feff}1\nkot/a\n", &[]).unwrap();
    assert_eq!(d.len(), 1);
    assert!(d.contains("kot") && d.contains("koty"));
}

#[test]
fn flag_aliases_stand_for_flag_sets() {
    // `AF`: `/1` in the `.dic` and in a continuation class means the first alias's flags.
    // Morphological aliases (`AM`) and the fields after a stem are skipped.
    let aff = "SET UTF-8\nAF 2\nAF ab # 1\nAF c # 2\nAM 1\nAM po:noun\nSFX a Y 1\nSFX a 0 ok/2 . 1\nSFX b Y 1\nSFX b 0 at . 1\nSFX c Y 1\nSFX c 0 ban . 1\n";
    let d = AffixDict::from_hunspell(aff, "1\nház/1\t1\n", &[]).unwrap();
    assert!(d.contains("ház") && d.contains("házok") && d.contains("házat") && d.contains("házokban"));
    assert!(!d.contains("házban"), "the stem has a and b, not c");
    // An undefined alias reads as no flags, as in Hunspell: the stem loads, without affixes.
    let undefined = AffixDict::from_hunspell(aff, "2\nház/3\nkert/1\n", &[]).unwrap();
    assert!(undefined.check("ház") && !undefined.check("házok"), "alias 3 is not defined");
    assert!(undefined.check("kertok"), "the other entries keep their aliases");
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.contains("házokban") && !back.contains("házban"), "aliases are expanded in the compact format");
}

#[test]
fn iconv_and_ignore_normalize_input() {
    // Ligatures read as their letters; `IGNORE` characters vanish from input and stems.
    let aff = "SET UTF-8\nICONV 2\nICONV ﬁ fi\nICONV ﬄ ffl\nIGNORE ()\nSFX a Y 1\nSFX a 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "2\nfine/a\nwaf(fle)/a\n", &[]).unwrap();
    assert!(d.check("ﬁne") && d.check("ﬁnes") && d.check("Fine") && d.contains("ﬁne"));
    assert!(d.check("waffle") && d.check("waﬄes") && d.check("wa(ffle)"));
    assert!(!d.check("ﬁner"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("ﬁnes") && back.check("waﬄe") && back.check("waf(fle)"));
}

#[test]
fn alias_numbers_are_read_as_hunspell_reads_them() {
    // Checked with the Hunspell 1.7 command line. With an `AF` table a flag field is the alias
    // its leading digits number: `fal/2x` is alias 2, `lab/b` has no flags.
    let aff = "SET UTF-8\nAF 2\nAF B\nAF C\nSFX B Y 1\nSFX B 0 s .\nSFX C Y 1\nSFX C 0 x .\nSFX b Y 1\nSFX b 0 q .\n";
    let d = AffixDict::from_hunspell(aff, "3\nlab/b\nfal/2x\nkut/1\n", &[]).unwrap();
    assert!(d.check("lab") && !d.check("labq") && !d.check("labs"));
    assert!(d.check("falx") && !d.check("fals") && d.check("kuts"));
    // A table without its count line is rejected: no alias applies.
    let broken = AffixDict::from_hunspell("SET UTF-8\nAF B\nAF C\nSFX B Y 1\nSFX B 0 s .\n", "2\nlab/1\nfal/2\n", &[]).unwrap();
    assert!(broken.check("lab") && !broken.check("labs") && !broken.check("fals"));
    // `AF 0` is rejected the same way: `lab/B` keeps its literal flag.
    let zero = AffixDict::from_hunspell("SET UTF-8\nAF 0\nSFX B Y 1\nSFX B 0 s .\n", "1\nlab/B\n", &[]).unwrap();
    assert!(zero.check("lab") && zero.check("labs"));
    // An undefined alias in a continuation class leaves the rule in force, without it.
    let cont = AffixDict::from_hunspell("SET UTF-8\nAF 1\nAF a\nSFX a Y 1\nSFX a 0 ok/7 .\n", "1\nhaz/1\n", &[]).unwrap();
    assert!(cont.check("haz") && cont.check("hazok"));
}

#[test]
fn a_rep_count_line_may_carry_a_comment() {
    let d = AffixDict::from_hunspell("SET UTF-8\nREP 1 # confusions\nREP x y\n", "1\nay\n", &[]).unwrap();
    assert_eq!(d.rep(), [("x".to_string(), "y".to_string())]);
}

#[test]
fn count_lines_may_carry_a_comment() {
    // Hunspell reads the first `AF` and `ICONV` line as the count, comment and all: `AF 2 #`
    // is no alias, and `ICONV 1 # …` doesn't turn `1` into `#`.
    let aff = "SET UTF-8\nAF 2 # aliases\nAF a # 1\nAF b # 2\nICONV 1 # ligatures\nICONV q y\nSFX a Y 1\nSFX a 0 ok .\nSFX b Y 1\nSFX b 0 at .\n";
    let d = AffixDict::from_hunspell(aff, "3\nhaz/1\nkert/2\na1\n", &[]).unwrap();
    assert!(d.check("hazok") && !d.check("hazat") && d.check("kertat") && !d.check("kertok"), "alias 1 is `a`");
    assert!(d.check("a1") && !d.check("a#"));
    // The UTF-8 reader keeps such a line as it is, so both readers agree on it.
    let (a, dic) = crate::hunspell::decode(aff.as_bytes(), "1\nhaz/1\n".as_bytes()).unwrap();
    assert!(a.contains("AF 2 # aliases") && a.contains("ICONV 1 # ligatures"), "{a}");
    assert!(AffixDict::from_hunspell(&a, &dic, &[]).unwrap().check("hazok"));
}

#[test]
fn dic_entries_may_hold_spaces() {
    // `ad hoc/B` is one entry, as in Hunspell; `ad` alone is no word.
    let aff = "SET UTF-8\nSFX B Y 1\nSFX B 0 s .\n";
    // Checked with the Hunspell 1.7 command line: without a tab or a morphological field the
    // whole line is the entry (`kot/B 12` has the flag `B`), and leading blanks belong to it
    // (`  lab` is no `lab`).
    let d = AffixDict::from_hunspell(aff, "5\nad hoc/B\tpo:adv\nword/B po:noun\nkot/B 12\n  lab/B\nés így/B\n", &[]).unwrap();
    assert!(d.contains("ad hoc") && d.contains("ad hocs") && d.contains("és így") && d.contains("és ígys"));
    assert!(!d.contains("ad") && !d.contains("hoc") && !d.contains("ads") && !d.contains("így"));
    assert!(d.contains("words") && d.contains("kots") && !d.contains("kot/B"));
    assert!(!d.contains("lab") && !d.contains("labs"));
}

#[test]
fn ignored_characters_leave_the_stripped_text_alone() {
    // Hunspell removes `IGNORE` characters from the appended text, not from the stripped
    // one: `SFX a bx c` never matches the stem `abx` (stored as `ab`), so `ac` is wrong,
    // while `SFX a b cx` appends `c` (checked with the Hunspell 1.7 command line).
    let strip = AffixDict::from_hunspell("SET UTF-8\nIGNORE x\nSFX a Y 1\nSFX a bx c .\n", "1\nabx/a\n", &[]).unwrap();
    assert!(strip.check("ab") && strip.check("abx") && !strip.check("ac"));
    let add = AffixDict::from_hunspell("SET UTF-8\nIGNORE x\nSFX a Y 1\nSFX a b cx .\n", "1\nab/a\n", &[]).unwrap();
    assert!(add.check("ac") && add.check("acx"));
    for d in [&strip, &add] {
        let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
        assert_eq!(back.check("ac"), d.check("ac"));
    }
}

/// CHECKSHARPS answers as Hunspell 1.7 gives them on the same files (`*` correct, `&` not).
#[test]
fn checksharps_reads_ss_as_sharp_s_in_capitals() {
    let dic = "6\nstraße\nMaße\nfußball/k\nmesse\nGruß/k\nSchloss\n";
    let words = [
        ("straße", true, true),
        ("Straße", true, true),
        ("STRASSE", false, true),
        ("STRAßE", true, true),
        ("STRAẞE", true, true),
        ("Strasse", false, false),
        ("strasse", false, false),
        ("MASSE", false, true),
        ("MAßE", true, true),
        ("maße", false, false),
        ("FUSSBALL", false, true),
        ("fußball", true, true),
        ("Fußball", false, true),
        ("FUßBALL", false, false),
        ("MESSE", true, true),
        ("MEßE", false, false),
        ("GRUSS", false, true),
        ("Gruß", true, true),
        ("gruß", false, false),
        ("GRUß", false, false),
        ("SCHLOSS", true, true),
        ("SCHLOß", false, false),
    ];
    for (sharps, aff) in [(false, "SET UTF-8\nKEEPCASE k\n"), (true, "SET UTF-8\nCHECKSHARPS\nKEEPCASE k\n")] {
        let d = AffixDict::from_hunspell(aff, dic, &[]).unwrap();
        let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
        for (w, without, with) in words {
            let want = if sharps { with } else { without };
            assert_eq!(d.check(w), want, "{w}, CHECKSHARPS {sharps}");
            assert_eq!(back.check(w), want, "{w}, CHECKSHARPS {sharps}, compact");
        }
    }
    assert_eq!(casing("STRAßE"), Casing::Upper);
    assert_eq!(casing("Maß"), Casing::Initial);
    // Many `SS` pairs: only the first MAX_SHARPS are varied, and a long word costs nothing.
    let d = AffixDict::from_hunspell("CHECKSHARPS\n", "1\naßaßaßaßaß\n", &[]).unwrap();
    assert!(d.check("ASSASSASSASSASS") && !d.check(&"SS".repeat(60)));
}

#[test]
fn oconv_converts_suggestions_only() {
    // Answers as Hunspell 1.7 gives them on the same files: `kbt` → `k’t`.
    let aff = "SET UTF-8\nTRY ab\nOCONV 1\nOCONV a ’\n";
    let d = AffixDict::from_hunspell(aff, "1\nkat\n", &[]).unwrap();
    assert!(d.check("kat") && !d.check("k’t"), "OCONV doesn't change what is correct");
    assert_eq!(d.output("kat"), "k’t");
    assert!(matches!(d.output("kot"), std::borrow::Cow::Borrowed("kot")));
    assert_eq!(crate::affix_suggest::suggest_in(&d, "kbt", 4, &crate::affix_suggest::Tuning::DEFAULT), ["k’t"]);
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert_eq!(back.output("kat"), "k’t");
    // The count line may carry a comment, as for ICONV.
    let d = AffixDict::from_hunspell("OCONV 1 # apostrophes\nOCONV ' ’\n", "1\nl'eau\n", &[]).unwrap();
    assert_eq!(d.output("l'eau"), "l’eau");
}

#[test]
fn conversions_take_the_longest_match_and_the_later_of_equal_ones() {
    let aff = "ICONV 3\nICONV a x\nICONV ab y\nICONV a z\n";
    let d = AffixDict::from_hunspell(aff, "2\ny\nzc\n", &[]).unwrap();
    assert_eq!(d.normalize("ab"), "y");
    assert_eq!(d.normalize("ac"), "zc");
    assert!(d.check("ab") && d.check("ac"));
}

#[test]
fn conversion_tables_are_capped() {
    let pairs = |n: usize| (0..n).map(|i| format!("ICONV q{i} r\n")).collect::<String>();
    let ok = AffixDict::from_hunspell(&format!("ICONV {MAX_CONV_PAIRS}\n{}", pairs(MAX_CONV_PAIRS)), "1\nr\n", &[]).unwrap();
    assert!(ok.check("q65535") && !ok.check("q65536"));
    let err = AffixDict::from_hunspell(&format!("ICONV 1\n{}", pairs(MAX_CONV_PAIRS + 1)), "1\nr\n", &[]).unwrap_err();
    assert!(err.contains("ICONV table larger"), "{err}");
    let long = "x".repeat(MAX_CONV_BYTES + 1);
    for aff in [format!("ICONV 1\nICONV {long} y\n"), format!("OCONV 1\nOCONV y {long}\n")] {
        assert!(AffixDict::from_hunspell(&aff, "1\nr\n", &[]).is_err(), "{aff}");
    }
    let raw = format!("{MAGIC}\nOCONV {long} y\nWORDS\n\0kot\n");
    assert!(AffixDict::from_bytes(&miniz_oxide::deflate::compress_to_vec(raw.as_bytes(), 6)).is_err());
}

#[test]
fn long_input_is_rejected_before_it_is_converted() {
    let d = AffixDict::from_hunspell("ICONV 1\nICONV ﬁ fi\nIGNORE -\n", "1\nfine\n", &[]).unwrap();
    // Ignored characters may take a word past MAX_WORD_BYTES and still leave a word.
    let spaced = "f-i-n-e".to_string() + &"-".repeat(MAX_WORD_BYTES);
    assert!(spaced.len() > MAX_WORD_BYTES && spaced.len() <= MAX_INPUT_BYTES && d.check(&spaced));
    // Past MAX_INPUT_BYTES the input is no word and is returned as it is, unconverted.
    let long = "ﬁ".repeat(MAX_INPUT_BYTES);
    assert!(matches!(d.normalize(&long), std::borrow::Cow::Borrowed(_)));
    assert!(!d.check(&long) && !d.contains(&long) && !d.can_suggest(&long));
}

#[test]
fn normalizing_borrows_when_nothing_changes() {
    let d = AffixDict::from_hunspell("SET UTF-8\nICONV 1\nICONV ﬁ fi\nIGNORE ()\n", "1\nfine\n", &[]).unwrap();
    assert!(matches!(d.normalize("fine"), std::borrow::Cow::Borrowed("fine")));
    assert_eq!(d.normalize("(ﬁ)ne"), "fine");
    assert!(matches!(dict().normalize("kot"), std::borrow::Cow::Borrowed("kot")));
}

#[test]
fn keepcase_stems_match_only_as_written() {
    let aff = "SET UTF-8\nKEEPCASE k\nSFX a Y 1\nSFX a 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "3\npH/k\nkm/ka\nkot/a\n", &[]).unwrap();
    assert!(d.check("pH") && d.check("km") && d.check("kms"));
    assert!(!d.check("PH") && !d.check("Km") && !d.check("KM") && !d.check("Kms"));
    assert!(d.check("Kot") && d.check("KOTS"), "other stems still follow the sentence and heading case");
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("km") && !back.check("Km"));
}

#[test]
fn substandard_and_lemma_present_words_are_correct() {
    // `SUBSTANDARD` and `LEMMA_PRESENT` steer morphological generation, and a SUBSTANDARD
    // stem is never suggested; the stems and rules they mark are still correct.
    let aff = "SET UTF-8\nSUBSTANDARD x\nLEMMA_PRESENT l\nSFX a Y 2\nSFX a 0 y .\nSFX a 0 em/x .\n";
    let d = AffixDict::from_hunspell(aff, "2\nkot/a\npies/axl\n", &[]).unwrap();
    for dict in [&d, &AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap()] {
        assert!(dict.check("kot") && dict.check("koty") && dict.check("kotem"));
        assert!(dict.check("pies") && dict.check("piesy") && dict.check("Piesem"));
        assert!(dict.can_suggest("koty") && dict.can_suggest("kotem") && dict.contains_suggestible("kot"));
        assert!(!dict.can_suggest("piesy") && !dict.can_suggest("Pies") && !dict.contains_suggestible("pies"));
    }
}

#[test]
fn unsupported_directives_are_refused_and_the_rest_load() {
    // A dictionary that needs a directive outside this engine fails to load rather than
    // accepting the wrong words.
    for dir in UNSUPPORTED {
        assert!(AffixDict::from_hunspell(&format!("SET UTF-8\n{dir} X\n"), DIC, &[]).is_err(), "{dir} should be refused");
    }
    // The directives this engine reads, with no other change to the dictionary (`AF` would
    // turn the flags of `DIC` into alias numbers; it has tests of its own).
    for line in ["AM 1\nAM po:noun", "ICONV 1\nICONV ﬁ fi", "IGNORE ()", "KEEPCASE k", "SUBSTANDARD x", "LEMMA_PRESENT l"] {
        let d = AffixDict::from_hunspell(&format!("{AFF}{line}\n"), DIC, &[]).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert!(d.check("koty") && d.check("niekoty") && !d.check("kotx"), "{line}");
    }
}

/// SCOWL's ordinal rules (its English `.aff`, verbatim): `21st`, `112th` and `1000th` are
/// compounds of digit stems, `1th` is only a part, and nothing else compounds.
const ORDINALS_AFF: &str = "SET UTF-8\nCOMPOUNDMIN 1\nONLYINCOMPOUND c\nCOMPOUNDRULE 2\nCOMPOUNDRULE n*1t\nCOMPOUNDRULE n*mp\nWORDCHARS 0123456789\nSFX S Y 1\nSFX S 0 s .\n";
const ORDINALS_DIC: &str = "13\n0/nm\n0th/pt\n1/n1\n1st/p\n1th/tc\n2/nm\n2nd/p\n2th/tc\n3/nm\n3rd/p\n3th/tc\n4/nm\n4th/pt\ncat/S\n";

#[test]
fn compound_rules_make_ordinals() {
    let d = AffixDict::from_hunspell(ORDINALS_AFF, ORDINALS_DIC, &[]).unwrap();
    for w in ["1st", "2nd", "4th", "21st", "22nd", "23rd", "11th", "12th", "13th", "111th", "1000th", "104th", "cats"] {
        assert!(d.check(w), "{w}");
    }
    for w in ["1th", "2th", "11st", "12nd", "13rd", "21th", "catcat", "cat1st", "1stcat", "1st1st", "", "x"] {
        assert!(!d.check(w), "{w}");
    }
    // Only-in-compound parts are never suggested on their own.
    assert!(!d.can_suggest("1th") && d.forms("1th").is_empty());
    // The compact format keeps the rules.
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("21st") && back.check("111th") && !back.check("1th") && !back.check("21th"));
}

#[test]
fn compound_rules_need_two_parts_and_compoundmin() {
    // `COMPOUNDMIN` defaults to 3; a pattern a lone stem matches still needs two parts.
    let aff = "SET UTF-8\nONLYINCOMPOUND c\nCOMPOUNDRULE a*b?\n";
    let d = AffixDict::from_hunspell(aff, "3\nfoo/a\nbar/ab\nqux/c\n", &[]).unwrap();
    assert!(d.check("foobar") && d.check("foofoo") && d.check("foofoofoobar") && d.check("foo"));
    assert!(!d.check("qux") && !d.check("quxfoo") && !d.check("fooqux") && !d.check("foobarx"));
    let d = AffixDict::from_hunspell(aff, "2\nab/ab\ncd/a\n", &[]).unwrap();
    assert!(!d.check("abcd"), "parts shorter than COMPOUNDMIN 3");
    let d = AffixDict::from_hunspell(&format!("{aff}COMPOUNDMIN 2\n"), "2\nab/ab\ncd/a\n", &[]).unwrap();
    assert!(d.check("cdab"));
}

/// craft-libs#483 review: a forbidden word isn't rebuilt from its parts.
#[test]
fn compound_never_rebuilds_a_forbidden_word() {
    let d = AffixDict::from_hunspell("SET UTF-8\nFORBIDDENWORD X\nCOMPOUNDMIN 1\nCOMPOUNDRULE a*\n", "2\nx/a\nxx/X\n", &[]).unwrap();
    assert!(!d.check("xx") && d.check("xxx") && d.check("x"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(!back.check("xx") && back.check("xxx"));
}

/// craft-libs#483 review: `ONLYINCOMPOUND` survives a round trip without compound rules too.
#[test]
fn onlyincompound_round_trips_without_compound_rules() {
    let d = AffixDict::from_hunspell("SET UTF-8\nONLYINCOMPOUND c\n", "2\nfoo\nqux/c\n", &[]).unwrap();
    assert!(d.check("foo") && !d.check("qux"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("foo") && !back.check("qux"), "a part-only stem stays a part");
}

/// With long or numeric flags a `COMPOUNDRULE` writes each flag in parentheses, as Hunspell
/// reads them (checked black-box against the `hunspell` CLI); a bare flag is refused.
#[test]
fn compound_rules_read_long_and_numeric_flags() {
    let aff = "SET UTF-8\nFLAG long\nCOMPOUNDMIN 1\nONLYINCOMPOUND cc\nCOMPOUNDRULE 1\nCOMPOUNDRULE (nn)*(mm)(pp)\n";
    let d = AffixDict::from_hunspell(aff, "4\n1/nnmm\n2/nnmm\n1st/pp\n1th/ppcc\n", &[]).unwrap();
    for w in ["1st", "21st", "11st", "221st", "21th"] {
        assert!(d.check(w), "{w}");
    }
    assert!(!d.check("1th") && !d.check("12"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("221st") && !back.check("1th"));
    let aff = "SET UTF-8\nFLAG num\nCOMPOUNDMIN 1\nONLYINCOMPOUND 9\nCOMPOUNDRULE 1\nCOMPOUNDRULE (1)*(2)(3)\n";
    let d = AffixDict::from_hunspell(aff, "3\n1/1,2\n2/1,2\n1st/3\n", &[]).unwrap();
    assert!(d.check("21st") && d.check("221st") && !d.check("12st"));
    for bad in ["FLAG long\nCOMPOUNDRULE nn*", "FLAG long\nCOMPOUNDRULE (n)", "FLAG num\nCOMPOUNDRULE (1", "FLAG num\nCOMPOUNDRULE (x)"] {
        assert!(AffixDict::from_hunspell(&format!("SET UTF-8\n{bad}\n"), "1\nx\n", &[]).is_err(), "{bad}");
    }
}

/// A NEEDAFFIX stem is no part of a `COMPOUNDRULE` compound (as the `hunspell` CLI answers).
#[test]
fn needaffix_stems_are_no_compound_part() {
    let d = AffixDict::from_hunspell("SET UTF-8\nNEEDAFFIX N\nCOMPOUNDRULE 1\nCOMPOUNDRULE ab\n", "2\nfoo/a\nbar/bN\n", &[]).unwrap();
    assert!(d.check("foo") && !d.check("bar") && !d.check("foobar"));
}

/// craft-libs#488 review: NEEDAFFIX, CIRCUMFIX and FULLSTRIP as Hunspell 1.7.5 reads them.
/// Each case is a reviewer's repro `.aff`/`.dic`; the right and wrong words are the `hunspell`
/// CLI's answers on the same files (black-box) for every prefix/stem/suffix combination of
/// them, and `forms()` lists exactly the right ones.
#[test]
fn needaffix_circumfix_and_fullstrip_match_hunspell() {
    const CASES: &[(&str, &str, &str, &str)] = &[
        // rv488-hs/circ
        (
            "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX S Y 1\nSFX S 0 y/c .\nSFX T Y 1\nSFX T 0 z .\n",
            "1\nstrom/PST\n",
            "nestromy strom stromz",
            "nenestrom nenestromy nenestromyy nenestromyz nenestromz nenestromzy nenestromzz nestrom nestromyy nestromyz nestromz nestromzy nestromzz stromy stromyy stromyz stromzy stromzz",
        ),
        // rv488-hs/circ_chain
        (
            "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX A Y 1\nSFX A 0 able/Bc .\nSFX B Y 1\nSFX B 0 s .\n",
            "1\nwork/AP\n",
            "neworkable work",
            "nenework neneworkable neneworkableable neneworkables neneworks neneworksable neneworkss nework neworkableable neworks neworksable neworkss workable workableable workables works worksable workss",
        ),
        // rv488-hs/circN
        (
            "SET UTF-8\nCIRCUMFIX c\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 ne/cN .\nSFX S Y 1\nSFX S 0 y/c .\n",
            "1\nstrom/PS\n",
            "nestromy strom",
            "nenestrom nenestromy nenestromyy nestrom nestromyy stromy stromyy",
        ),
        // rv488-hs/circ_nocross
        (
            "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX S N 1\nSFX S 0 y/c .\n",
            "1\nstrom/PS\n",
            "strom",
            "nenestrom nenestromy nenestromyy nestrom nestromy nestromyy stromy stromyy",
        ),
        // rv488-hs/comp
        ("SET UTF-8\nNEEDAFFIX N\nCOMPOUNDRULE 1\nCOMPOUNDRULE ab\n", "2\nfoo/a\nbar/bN\n", "foo", "bar"),
        // rv488-hs/full
        ("SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A ab x .\nPFX B Y 1\nPFX B ab y .\n", "1\nab/AB\n", "ab x y", ""),
        // rv488-hs/full_pfxsfx
        ("SET UTF-8\nFULLSTRIP\nCIRCUMFIX c\nPFX P Y 1\nPFX P ab y/c .\nSFX S Y 1\nSFX S ab z/c .\n", "1\nab/PS\n", "ab", "y z"),
        // rv488-hs/homo2
        ("SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 s .\n", "2\nfoo/NA\nfoo\n", "foo foos", "fooss"),
        // rv488-hs/homo
        ("SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 s .\n", "2\nfoo/NA\nfoo/B\n", "foo foos", "fooss"),
        // rv488-hs/pfxna
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re/N .\nSFX S Y 1\nSFX S 0 s/N .\n",
            "1\nwork/PS\n",
            "work",
            "rerework rereworks rereworkss rework reworks reworkss works workss",
        ),
        // rv488-hs/pfxstem
        ("SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\n", "1\nbaz/NP\n", "rebaz", "baz rerebaz"),
        // rv488-hs/pN_s
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re/N .\nSFX S Y 1\nSFX S 0 s .\n",
            "1\nwork/PS\n",
            "reworks work works",
            "rerework rereworks rereworkss rework reworkss workss",
        ),
        // rv488-hs/p_sN
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\nSFX S Y 1\nSFX S 0 s/N .\n",
            "1\nwork/PS\n",
            "rework reworks work",
            "rerework rereworks rereworkss reworkss works workss",
        ),
        // rv488-hs/sN_chain
        ("SET UTF-8\nNEEDAFFIX N\nSFX B Y 1\nSFX B 0 x/AN .\nSFX A Y 1\nSFX A 0 s .\n", "1\nbar/B\n", "bar barxs", "bars barss barsx barx barxx"),
        // rv488-hs/sNnocross
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\nSFX S N 1\nSFX S 0 s/N .\n",
            "1\nwork/PS\n",
            "rework work",
            "rerework rereworks rereworkss reworks reworkss works workss",
        ),
        // rv488-hs/stemN_ps
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\nSFX S Y 1\nSFX S 0 s .\n",
            "1\nwork/PSN\n",
            "rework reworks works",
            "rerework rereworks rereworkss reworkss work workss",
        ),
        // rv488-hs/zeroaff
        ("SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 0 .\n", "1\nfoo/NA\n", "foo", ""),
        // rv488-second2/t11
        (
            "SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A ab x .\nPFX B Y 1\nPFX B ab y .\nSFX C Y 1\nSFX C 0 z .\n",
            "1\nab/ABC\n",
            "ab abz x y yz",
            "abzz xz yzz",
        ),
        // rv488-second2/t12
        ("SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A x 0 .\nPFX P Y 1\nPFX P 0 q .\n", "1\nx/AP\n", "q qx x", "qq qqx"),
        // rv488-second2/t13
        ("SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A x 0/B .\nSFX B Y 1\nSFX B 0 y .\n", "1\nx/A\n", "x y", "xy xyy"),
        // rv488-second2/t1
        ("SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 s .\n", "2\nfoo/N\nfoo/A\n", "foo foos", "fooss"),
        // rv488-second2/t2
        (
            "SET UTF-8\nCIRCUMFIX X\nPFX A Y 1\nPFX A 0 leg/X .\nPFX B Y 1\nPFX B 0 legesleg/X .\nSFX C Y 3\nSFX C 0 obb .\nSFX C 0 obb/AX .\nSFX C 0 obb/BX .\n",
            "1\nnagy/C\n",
            "legeslegnagyobb legnagyobb nagy nagyobb",
            "legesleglegeslegnagy legesleglegeslegnagyobb legesleglegeslegnagyobbobb legesleglegnagy legesleglegnagyobb legesleglegnagyobbobb legeslegnagy legeslegnagyobbobb leglegeslegnagy leglegeslegnagyobb leglegeslegnagyobbobb leglegnagy leglegnagyobb leglegnagyobbobb legnagy legnagyobbobb nagyobbobb",
        ),
        // rv488-second2/t30
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\nSFX B Y 1\nSFX B 0 x/N .\n",
            "1\nbar/BP\n",
            "bar rebar rebarx",
            "barx barxx rebarxx rerebar rerebarx rerebarxx",
        ),
        // rv488-second2/t31
        (
            "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne .\nPFX Q Y 1\nPFX Q 0 qe/c .\nSFX S Y 1\nSFX S 0 y/c .\nSFX T Y 1\nSFX T 0 z .\n",
            "1\nstrom/PQST\n",
            "nestrom nestromz qestromy strom stromz",
            "nenestrom nenestromy nenestromyy nenestromyz nenestromz nenestromzy nenestromzz neqestrom neqestromy neqestromyy neqestromyz neqestromz neqestromzy neqestromzz nestromy nestromyy nestromyz nestromzy nestromzz qenestrom qenestromy qenestromyy qenestromyz qenestromz qenestromzy qenestromzz qeqestrom qeqestromy qeqestromyy qeqestromyz qeqestromz qeqestromzy qeqestromzz qestrom qestromyy qestromyz qestromz qestromzy qestromzz stromy stromyy stromyz stromzy stromzz",
        ),
        // rv488-second2/t32
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re/N .\nSFX B Y 1\nSFX B 0 x/N .\n",
            "1\nbar/BP\n",
            "bar",
            "barx barxx rebar rebarx rebarxx rerebar rerebarx rerebarxx",
        ),
        // rv488-second2/t3
        (
            "SET UTF-8\nPFX A Y 1\nPFX A 0 leg .\nSFX C Y 1\nSFX C 0 obb/A .\n",
            "1\nnagy/C\n",
            "legnagyobb nagy nagyobb",
            "leglegnagy leglegnagyobb leglegnagyobbobb legnagy legnagyobbobb nagyobbobb",
        ),
        // rv488-second2/t4
        (
            "SET UTF-8\nCIRCUMFIX X\nPFX A Y 1\nPFX A 0 leg/X .\nSFX C Y 2\nSFX C 0 obb .\nSFX C 0 obb/X .\n",
            "1\nnagy/AC\n",
            "legnagyobb nagy nagyobb",
            "leglegnagy leglegnagyobb leglegnagyobbobb legnagy legnagyobbobb nagyobbobb",
        ),
        // rv488-second2/t5
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX A Y 1\nPFX A 0 leg/N .\nSFX C Y 1\nSFX C 0 obb/A .\n",
            "1\nnagy/C\n",
            "legnagyobb nagy nagyobb",
            "leglegnagy leglegnagyobb leglegnagyobbobb legnagy legnagyobbobb nagyobbobb",
        ),
        // rv488-second2/t6
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX A Y 1\nPFX A 0 leg .\nSFX C Y 1\nSFX C 0 obb/AN .\n",
            "1\nnagy/C\n",
            "legnagyobb nagy",
            "leglegnagy leglegnagyobb leglegnagyobbobb legnagy legnagyobbobb nagyobb nagyobbobb",
        ),
        // rv488-second2/t7
        (
            "SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 able/B .\nSFX B Y 1\nSFX B 0 s/N .\n",
            "1\nwork/A\n",
            "work workable workables",
            "workableable works worksable workss",
        ),
        // rv488-second2/t8
        (
            "SET UTF-8\nNEEDAFFIX N\nPFX P Y 1\nPFX P 0 re .\nSFX Z Y 1\nSFX Z 0 0 .\nSFX S Y 1\nSFX S 0 s .\n",
            "2\nfoo/NPS\nbar/NZ\n",
            "bar foos refoo refoos",
            "bars barss foo fooss rebar rebars rebarss refooss rerebar rerebars rerebarss rerefoo rerefoos rerefooss",
        ),
        // rv488-second2/t9
        (
            "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX A Y 1\nSFX A 0 able/B .\nSFX B Y 1\nSFX B 0 s/c .\n",
            "1\nwork/AP\n",
            "work workable workables",
            "nenework neneworkable neneworkableable neneworkables neneworks neneworksable neneworkss nework neworkable neworkableable neworkables neworks neworksable neworkss workableable works worksable workss",
        ),
    ];
    for (aff, dic, right, wrong) in CASES {
        let d = AffixDict::from_hunspell(aff, dic, &[]).unwrap();
        for w in right.split_whitespace() {
            assert!(d.check(w), "{w} is right:\n{aff}{dic}");
        }
        for w in wrong.split_whitespace() {
            assert!(!d.check(w), "{w} is wrong:\n{aff}{dic}");
        }
        let mut forms: Vec<String> = dic.lines().skip(1).flat_map(|l| d.forms(l.split('/').next().unwrap_or(""))).collect();
        forms.sort();
        forms.dedup();
        let mut want: Vec<&str> = right.split_whitespace().collect();
        want.sort_unstable();
        assert_eq!(forms, want, "forms():\n{aff}{dic}");
    }
}

/// A circumfix prefix with two suffixes (`ne` + `work` + `able/Bc` + `s`): Hunspell 1.7.5
/// takes `neworkables`; this engine allows a prefix with one suffix only.
#[test]
#[ignore = "prefix + two suffixes is not supported (craft-libs#488 review P3)"]
fn a_circumfix_prefix_with_a_suffix_chain() {
    let aff = "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX A Y 1\nSFX A 0 able/Bc .\nSFX B Y 1\nSFX B 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "1\nwork/AP\n", &[]).unwrap();
    assert!(d.check("neworkables"));
}

#[test]
fn a_prefix_with_a_suffix_chain() {
    // The CLI's answers (`meg` + `épít` + `és` + `e` in Hungarian): every rule must allow
    // cross products, and the prefix's flag comes from the stem or a continuation class.
    let rules = |p: char, a: char, b: char, a_cont: &str, b_cont: &str| {
        format!("SET UTF-8\nPFX P {p} 1\nPFX P 0 un .\nSFX A {a} 1\nSFX A 0 a/B{a_cont} .\nSFX B {b} 1\nSFX B 0 b{b_cont} .\n")
    };
    let d = |aff: &str, dic: &str| AffixDict::from_hunspell(aff, &format!("1\n{dic}\n"), &[]).unwrap();
    let yes = d(&rules('Y', 'Y', 'Y', "", ""), "foo/PA");
    assert!(yes.check("unfooab") && yes.check("unfooa") && yes.check("fooab"));
    for (p, a, b) in [('Y', 'Y', 'N'), ('Y', 'N', 'Y'), ('N', 'Y', 'Y')] {
        assert!(!d(&rules(p, a, b, "", ""), "foo/PA").check("unfooab"), "{p}{a}{b}");
    }
    assert!(d(&rules('Y', 'Y', 'Y', "P", ""), "foo/A").check("unfooab"), "flag from the inner suffix");
    let outer = d(&rules('Y', 'Y', 'Y', "", "/P"), "foo/A");
    assert!(outer.check("unfooab") && !outer.check("unfooa"), "flag from the outer suffix");
    assert!(!d(&rules('Y', 'Y', 'Y', "", ""), "foo/A").check("unfooab"), "no prefix flag");
}

#[test]
fn a_prefix_and_a_suffix_come_from_one_homonym() {
    // `stop/PA` and `stop/S`: `unstop` and `stops` are right, `unstops` wrong (the CLI's
    // answers), also after a round trip through the compact form.
    let aff = "SET UTF-8\nPFX P Y 1\nPFX P 0 un .\nSFX A Y 1\nSFX A 0 a/B .\nSFX B Y 1\nSFX B 0 b .\nSFX S Y 1\nSFX S 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "3\nstop/PA\nstop/S\ngo/PS\n", &[]).unwrap();
    let again = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    for d in [d, again] {
        assert!(d.check("unstop") && d.check("stops") && d.check("unstopa") && d.check("unstopab") && d.check("ungos"));
        assert!(!d.check("unstops"));
    }
}

#[test]
fn a_word_in_capitals_may_be_a_mixed_case_stem() {
    // The CLI's answers: `KB` and `KBS` from `kB/SU`, not `KBT`, `UNKB` or `Kb`.
    let aff = "SET UTF-8\nSFX S Y 1\nSFX S 0 s .\nPFX U Y 1\nPFX U 0 un .\nSFX T Y 1\nSFX T 0 t .\n";
    let d = AffixDict::from_hunspell(aff, "2\nkB/SU\naBcD\n", &[]).unwrap();
    assert!(d.check("KB") && d.check("KBS") && d.check("kB") && d.check("ABCD"));
    assert!(!d.check("KBT") && !d.check("UNKB") && !d.check("Kb") && !d.check("kb") && !d.check("Abcd"));
}

#[test]
fn a_word_in_capitals_is_no_compound_of_a_mixed_case_stem() {
    // The CLI's answers: `MSCS` and `ABC` right, `MSCFOO` and `ABCFOO` wrong.
    let d = AffixDict::from_hunspell("SET UTF-8\nCOMPOUNDFLAG X\nSFX S Y 1\nSFX S 0 s .\n", "3\nMSc/SX\naBc/X\nfoo/X\n", &[]).unwrap();
    assert!(d.check("MSCS") && d.check("MSC") && d.check("ABC") && d.check("MScfoo"));
    assert!(!d.check("MSCFOO") && !d.check("ABCFOO") && !d.check("FOOABC"));
}

#[test]
fn homonyms_kept_apart_survive_the_compact_form() {
    // `foo/XN` beside `foo/S`: `foos` right, `foobar` wrong (the CLI's answers), also after
    // a round trip (the NEEDAFFIX flag is written).
    let aff = "SET UTF-8\nCOMPOUNDFLAG X\nNEEDAFFIX N\nCOMPOUNDMIN 1\nSFX S Y 1\nSFX S 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "3\nfoo/XN\nfoo/S\nbar/X\n", &[]).unwrap();
    let again = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    for d in [d, again] {
        assert!(d.check("foos") && d.check("foo"));
        assert!(!d.check("foobar") && !d.check("foosbar") && !d.check("barfoo"));
    }
}

#[test]
fn many_prefixes_adding_nothing_stay_cheap() {
    // A thousand prefix rules that add nothing, and 40 chained suffix lengths of 50 flags
    // each: every prefix leaves the same root, so they share one walk.
    let mut aff = String::from("SET UTF-8\nFLAG num\n");
    for f in 1..=1000 {
        aff.push_str(&format!("PFX {f} Y 1\nPFX {f} 0 0 .\n"));
    }
    let conts: Vec<String> = (0..50).map(|k| (5000 + k * 7).to_string()).collect();
    for len in 1..=40 {
        for k in 0..50 {
            let f = 5000 + (len - 1) * 50 + k;
            aff.push_str(&format!("SFX {f} Y 1\nSFX {f} 0 {}/{} .\n", "a".repeat(len), conts.join(",")));
        }
    }
    let d = AffixDict::from_hunspell(&aff, "2\nb/1,5000\nab/2\n", &[]).unwrap();
    let t = std::time::Instant::now();
    for n in [40, 119] {
        let _ = d.check(&"a".repeat(n));
    }
    let _ = d.suggest(&format!("{}x", "a".repeat(39)), 8);
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}

#[test]
fn prefixes_with_and_without_cross_products_on_one_root() {
    // The CLI's answers: `redo` and `rework` right, from `c` (no cross products) beside `a`
    // and `b` (cross products) adding the same `re`.
    let aff = "SET UTF-8\nPFX a Y 1\nPFX a 0 re .\nPFX b Y 1\nPFX b 0 re .\nPFX c N 1\nPFX c 0 re .\nSFX S Y 1\nSFX S 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "2\ndo/c\nwork/cS\n", &[]).unwrap();
    assert!(d.check("redo") && d.check("rework") && d.check("works"));
    assert!(!d.check("reworks"));
    let aff = "SET UTF-8\nFLAG num\nPFX 1 Y 1\nPFX 1 0 re .\nPFX 2 Y 1\nPFX 2 0 re .\nPFX 3 N 1\nPFX 3 0 re .\nPFX 4 N 1\nPFX 4 0 re .\nPFX 5 N 1\nPFX 5 0 re .\n";
    let d = AffixDict::from_hunspell(aff, "5\nfoo/3\nbar/4\nbaz/5\nqux/1\nquu/2\n", &[]).unwrap();
    assert!(["refoo", "rebar", "rebaz", "requx", "requu"].iter().all(|w| d.check(w)));
}

#[test]
fn many_prefixes_with_their_own_roots_stay_cheap() {
    // A thousand prefix rules each stripping its own letter (a thousand roots), and 40
    // chained suffix lengths of 50 flags each: one word's rules share one budget.
    let mut aff = String::from("SET UTF-8\nFLAG num\n");
    for f in 1..=1000u32 {
        let c = char::from_u32(0x4e00 + f).unwrap();
        aff.push_str(&format!("PFX {f} Y 1\nPFX {f} {c} 0 .\n"));
    }
    let conts: Vec<String> = (0..50).map(|k| (5000 + k * 7).to_string()).collect();
    for len in 1..=40 {
        for k in 0..50 {
            let f = 5000 + (len - 1) * 50 + k;
            aff.push_str(&format!("SFX {f} Y 1\nSFX {f} 0 {}/{} .\n", "a".repeat(len), conts.join(",")));
        }
    }
    let d = AffixDict::from_hunspell(&aff, "2\nb/1,5000\nab/2\n", &[]).unwrap();
    let t = std::time::Instant::now();
    for n in [40, 119] {
        let _ = d.check(&"a".repeat(n));
    }
    let _ = d.suggest(&format!("{}x", "a".repeat(39)), 8);
    assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
}

#[test]
fn suggesting_with_a_long_try_line_shares_one_budget() {
    // craft-libs#497 review: a thousand one-letter prefixes, 40 chained suffix lengths and
    // a 128-letter TRY line. Each TRY candidate is a simple-word check; one suggest call
    // shares one budget between them (17 s in a release build before CP7.7).
    let mut aff = String::from("SET UTF-8\nFLAG num\nTRY ");
    aff.extend(('a'..='z').chain((0x100..0x166).filter_map(char::from_u32)));
    aff.push('\n');
    for f in 1..=1000u32 {
        let c = char::from_u32(0x4e00 + f).unwrap();
        aff.push_str(&format!("PFX {f} Y 1\nPFX {f} {c} 0 .\n"));
    }
    let conts: Vec<String> = (0..286).map(|k| (5000 + k * 7).to_string()).collect();
    for len in 1..=40 {
        for k in 0..50 {
            let f = 5000 + (len - 1) * 50 + k;
            aff.push_str(&format!("SFX {f} Y 1\nSFX {f} 0 {}/{} .\n", "a".repeat(len), conts.join(",")));
        }
    }
    assert_eq!(aff.lines().nth(2).map(|l| l.chars().count()), Some(4 + 128));
    let d = AffixDict::from_hunspell(&aff, "2\nb/1,5000\nab/2\n", &[]).unwrap();
    let t = std::time::Instant::now();
    let _ = d.suggest(&format!("{}x{}", "a".repeat(20), "a".repeat(19)), 8);
    assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
}

#[test]
fn a_check_out_of_steps_says_so() {
    // craft-libs#497 review: the near-word check of a compound treats a candidate whose
    // steps ran out as a word (the compound is refused), not as no word.
    let d = AffixDict::from_hunspell("SET UTF-8\nSFX A Y 1\nSFX A 0 s .\n", "1\nkot/A\n", &[]).unwrap();
    let e = d.excluded(true);
    assert_eq!(d.simple_word_budgeted("kots", e, true), Some(true));
    let _steps = SuggestSteps::start();
    SUGGEST_STEPS.with(|s| s.set(Some(0)));
    assert_eq!(d.simple_word_budgeted("kots", e, true), None);
    assert_eq!(d.simple_word_budgeted("kot", e, true), Some(true));
}

#[test]
fn many_stems_sharing_big_homonym_aliases_load_fast() {
    // craft-libs#497 review: 16 homonyms per stem, each an `AF` alias of thousands of flags,
    // on thousands of stems. Each stem's sets are worked out once per distinct entry list.
    let n = 4000;
    let mut aff = String::from("SET UTF-8\nFLAG num\nAF 16\n");
    for k in 0..16 {
        let flags: Vec<String> = (1..=n).filter(|&f| f != n - k).map(|f| f.to_string()).collect();
        aff.push_str(&format!("AF {}\n", flags.join(",")));
    }
    aff.push_str("SFX 1 Y 1\nSFX 1 0 s .\n");
    let stems: Vec<String> = (0..2000).map(|i| format!("w{i}")).collect();
    let mut dic = format!("{}\n", stems.len() * 16);
    for s in &stems {
        for k in 1..=16 {
            dic.push_str(&format!("{s}/{k}\n"));
        }
    }
    let t = std::time::Instant::now();
    let d = AffixDict::from_hunspell(&aff, &dic, &[]).unwrap();
    assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
    assert!(d.check("w7") && d.check("w1999s") && !d.check("w2000"));
}

#[test]
fn hostile_compound_rules_are_refused_or_cheap() {
    for bad in ["COMPOUNDRULE *a", "COMPOUNDRULE (ab)", "COMPOUNDRULE a**", "COMPOUNDMIN x", "ONLYINCOMPOUND ab"] {
        assert!(AffixDict::from_hunspell(&format!("SET UTF-8\n{bad}\n"), DIC, &[]).is_err(), "{bad}");
    }
    let many: String = (0..100).map(|_| "COMPOUNDRULE a*\n").collect();
    assert!(AffixDict::from_hunspell(&format!("SET UTF-8\n{many}"), DIC, &[]).is_err());
    // Every part a stem and a pattern of stars: still answers at once.
    let star: String = std::iter::repeat_n("a*", 16).collect();
    let d = AffixDict::from_hunspell(&format!("SET UTF-8\nCOMPOUNDMIN 1\nCOMPOUNDRULE {star}b\n"), "1\nx/a\n", &[]).unwrap();
    let t = std::time::Instant::now();
    assert!(!d.check(&"x".repeat(48)) && !d.check(&"x".repeat(200)));
    assert!(t.elapsed().as_secs() < 2);
}

#[test]
fn fields_split_on_spaces_and_tabs_only() {
    // A flag byte read as U+00A0 (no-break space) or U+0085 is a flag, not a separator.
    let aff = "SET UTF-8\nSFX \u{a0} Y 1\nSFX \u{a0} 0 y .\nSFX \u{85} Y 1\nSFX \u{85} 0 em .\n";
    let d = AffixDict::from_hunspell(aff, "1\nkot/\u{a0}\u{85}\n", &[]).unwrap();
    assert!(d.contains("koty") && d.contains("kotem"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.contains("koty") && back.contains("kotem"));
    // The same bytes as one-flag directives, as Hunspell reads them.
    let aff = "SET UTF-8\nFORBIDDENWORD \u{a0}\nKEEPCASE \u{85}\nSFX a Y 1\nSFX a 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "3\nkot/a\nrossz/\u{a0}\npH/\u{85}\n", &[]).unwrap();
    for dict in [&d, &AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap()] {
        assert!(dict.check("kots") && !dict.check("rossz"));
        assert!(dict.check("pH") && !dict.check("PH"));
    }
}

/// craft-libs#482 review: an English dictionary with compounding and output conversion, as the
/// usual en_US and en_GB ones have, loads for English: `COMPOUNDRULE` makes its ordinals, its
/// `ONLYINCOMPOUND` stems are words only inside them, `COMPOUNDFLAG` is ignored (other
/// languages read it) and `OCONV` is applied to suggestions. (A synthetic `.aff`
/// written for this test.)
#[test]
fn english_dictionaries_ignore_compounding_and_output_conversion() {
    let aff = "SET UTF-8\nTRY esianrtolcdugmphbyfvkwz'\nICONV 1\nICONV ’ '\nOCONV 1\nOCONV ' ’\nNOSUGGEST !\n\
               COMPOUNDMIN 1\nONLYINCOMPOUND c\nCOMPOUNDRULE 2\nCOMPOUNDRULE n*1t\nCOMPOUNDRULE n*mp\nCOMPOUNDFLAG Z\n\
               WORDCHARS 0123456789\nSFX S Y 1\nSFX S 0 s [^sxzhy]\nSFX D Y 1\nSFX D 0 ed [^ey]\n";
    let dic = "7\nhouse/S\nwalk/SD\n0th/pc\n1st/p\n1th/tc\n1/n1\nthe\n";
    assert!(AffixDict::from_hunspell(&format!("{aff}COMPOUNDMORESUFFIXES\n"), dic, &[]).is_err(), "refused with COMPOUNDFLAG");
    let d = AffixDict::from_hunspell_english(aff, dic, &[]).unwrap();
    for w in ["house", "houses", "walked", "walks", "the"] {
        assert!(d.check(w), "{w}");
    }
    assert!(!d.check("0th") && !d.check("1th"), "parts of compounds only");
    assert!(d.check("1st") && d.check("11th") && d.check("111th") && !d.check("11st"), "ordinals by COMPOUNDRULE");
    assert!(!d.check("housed") && !d.check("hous"));
    // OCONV is read and applied to suggestions, as Hunspell does: `'` is shown as `’`.
    let d = AffixDict::from_hunspell_english(aff, "2\ndon't\nwalk/S\n", &[]).unwrap();
    assert_eq!(d.output("don't"), "don’t");
    assert!(d.check("don't") && d.check("don’t"), "ICONV reads it back");
    assert!(crate::affix_suggest::suggest_in(&d, "donn't", 4, &crate::affix_suggest::Tuning::DEFAULT).contains(&"don’t".to_string()));
    // The directives that change which words are right are still refused.
    assert!(AffixDict::from_hunspell_english("PSEUDOROOT X\n", dic, &[]).is_err(), "PSEUDOROOT");
}

/// The English path reads `ONLYINCOMPOUND` in the dictionary's FLAG mode too.
#[test]
fn english_onlyincompound_reads_long_and_numeric_flags() {
    for (aff, dic) in [
        ("FLAG long\nONLYINCOMPOUND cc\nCOMPOUNDMIN 1\nSFX Ss Y 1\nSFX Ss 0 s .\n", "3\nhouse/Ss\n1st/cc\nnth/Sscc\n"),
        ("FLAG num\nONLYINCOMPOUND 7\nCOMPOUNDMIN 1\nSFX 1 Y 1\nSFX 1 0 s .\n", "3\nhouse/1\n1st/7\nnth/1,7\n"),
    ] {
        let d = AffixDict::from_hunspell_english(aff, dic, &[]).unwrap();
        assert!(d.check("house") && d.check("houses"), "{aff}");
        assert!(!d.check("1st") && !d.check("nth") && !d.check("nths"), "{aff}");
    }
}

#[test]
fn unsupported_and_broken_input_is_an_error() {
    // `FLAG long` and `FLAG num` load now (see the tests below); an unknown mode doesn't.
    assert!(AffixDict::from_hunspell("FLAG long\n", DIC, &[]).is_ok());
    assert!(AffixDict::from_hunspell("FLAG num\n", DIC, &[]).is_ok());
    assert!(AffixDict::from_hunspell("FLAG bogus\n", DIC, &[]).is_err());
    assert!(AffixDict::from_hunspell("COMPOUNDFLAG X\nCHECKCOMPOUNDPATTERN 1\nCHECKCOMPOUNDPATTERN a b c\n", DIC, &[]).is_err());
    assert!(AffixDict::from_hunspell("SFX a Y 1\nSFX a 0 y [ab\n", DIC, &[]).is_err());
    assert!(AffixDict::from_bytes(b"not deflate").is_err());
    let ok = dict().to_bytes().unwrap();
    assert!(AffixDict::from_bytes(&ok[..ok.len() / 2]).is_err());
    let garbage = miniz_oxide::deflate::compress_to_vec(b"WCAFF2\nWORDS\n\x09abc\n", 6);
    assert!(AffixDict::from_bytes(&garbage).is_err(), "prefix longer than the previous word");
    let empty = AffixDict::default();
    assert!(!empty.contains("kot") && !empty.check("Kot") && empty.forms("kot").is_empty());
}

/// A small `FLAG num` dictionary (made-up words): prefixes, suffixes, a continuation class,
/// a cross product, the largest flag number, and numeric FORBIDDENWORD and NOSUGGEST flags.
const NUM_AFF: &str = "SET UTF-8\nFLAG num\nFORBIDDENWORD 900\nNOSUGGEST 901\nPFX 7 Y 1\nPFX 7 0 ne .\nSFX 12 Y 2\nSFX 12 0 a .\nSFX 12 0 om/1503 .\nSFX 1503 N 1\nSFX 1503 0 e .\nSFX 65535 N 1\nSFX 65535 0 i .\n";
const NUM_DIC: &str = "5\ngrad/12,7\nmost/65535\nkuca/12,901\ngradd/900\nlist/1503\n";

fn assert_num_dict(d: &AffixDict) {
    for w in ["grad", "grada", "gradom", "gradome", "negrad", "negrada", "negradom", "most", "mosti", "list", "liste", "Grada", "MOSTI"] {
        assert!(d.check(w), "{w}");
    }
    // 1503 reaches `grad` only through `om`; 12 and 65535 belong to other stems; 7 is a prefix.
    for w in ["grade", "gradi", "mosta", "nemost", "lista", "listom", "gradomi", "7grad", "grad12"] {
        assert!(!d.check(w), "{w}");
    }
    assert!(!d.check("gradd") && !d.check("Gradd") && d.forms("gradd").is_empty(), "numeric FORBIDDENWORD");
    assert!(d.check("kuca") && d.check("kucaa") && !d.can_suggest("kuca") && !d.can_suggest("kucaom"), "numeric NOSUGGEST");
    assert!(d.can_suggest("gradome"));
    let mut forms = d.forms("grad");
    forms.sort();
    forms.dedup();
    assert_eq!(forms, ["grad", "grada", "gradom", "gradome", "negrad", "negrada", "negradom"]);
}

#[test]
fn flag_num_dictionaries_load_and_check() {
    let d = AffixDict::from_hunspell(NUM_AFF, NUM_DIC, &[]).unwrap();
    assert_eq!((d.len(), d.rule_count()), (5, 5));
    assert_num_dict(&d);
}

#[test]
fn flag_num_dictionaries_round_trip_the_compact_format() {
    let d = AffixDict::from_hunspell(NUM_AFF, NUM_DIC, &["num test"]).unwrap();
    let bytes = d.to_bytes().unwrap();
    let raw = miniz_oxide::inflate::decompress_to_vec(&bytes).unwrap();
    assert!(raw.starts_with(b"WCAFF2\nFLAG num\n"), "the mode is written before any flag");
    let text = String::from_utf8_lossy(&raw);
    assert!(text.contains("FORBIDDENWORD 900\n") && text.contains("SFX 12 Y 0 om/1503 .\n") && text.contains("grad/7,12\n"));
    let back = AffixDict::from_bytes(&bytes).unwrap();
    assert_num_dict(&back);
    assert_eq!(back.comments(), ["num test"]);
    assert_eq!(back.to_bytes().unwrap(), bytes, "a second round trip writes the same data");
}

/// A small `FLAG long` dictionary: two-character flags that share characters with each other.
const LONG_AFF: &str = "SET UTF-8\nFLAG long\nFORBIDDENWORD !!\nSFX Aa Y 1\nSFX Aa 0 s/Bb .\nSFX Bb N 1\nSFX Bb 0 ly .\nPFX Cc Y 1\nPFX Cc 0 un .\nSFX aA N 1\nSFX aA 0 er .\n";
const LONG_DIC: &str = "5\nkind/AaCc\nhope/Bb\nbadd/!!\nodd/Aab\nwarm/aA\n";

fn assert_long_dict(d: &AffixDict) {
    // `odd/Aab`: an odd last flag character is dropped, as Hunspell does (`odds` is right).
    for w in ["kind", "kinds", "kindsly", "unkind", "unkinds", "hope", "hopely", "warm", "warmer", "odd", "odds"] {
        assert!(d.check(w), "{w}");
    }
    for w in ["kindly", "kinder", "hopes", "unhope", "warms", "unwarm", "badd", "unodd"] {
        assert!(!d.check(w), "{w}");
    }
}

#[test]
fn flag_aliases_hold_long_and_numeric_flags() {
    // `AF` aliases stand for flags written in the FLAG mode: `AaBb` is two long flags,
    // `12,7` two numbers. Answers match Hunspell 1.7's on the same files.
    let aff = "SET UTF-8\nFLAG long\nAF 2\nAF AaBb\nAF Bb\nSFX Aa Y 1\nSFX Aa 0 s/2 .\nSFX Bb Y 1\nSFX Bb 0 ly .\n";
    let d = AffixDict::from_hunspell(aff, "2\nkind/1\nhope/2\n", &[]).unwrap();
    for w in ["kind", "kinds", "kindly", "kindsly", "hope", "hopely"] {
        assert!(d.check(w), "{w}");
    }
    assert!(!d.check("hopes"));
    let aff = "SET UTF-8\nFLAG num\nAF 1\nAF 12,7\nSFX 12 Y 1\nSFX 12 0 a .\nPFX 7 Y 1\nPFX 7 0 ne .\n";
    let d = AffixDict::from_hunspell(aff, "1\ngrad/1\n", &[]).unwrap();
    for w in ["grad", "grada", "negrad", "negrada"] {
        assert!(d.check(w), "{w}");
    }
    assert!(AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap().check("negrada"));
    // A stem's alias reads as its own flags would: an odd last long-flag character is
    // dropped (Hunspell: `odd` and `odds` right).
    let d = AffixDict::from_hunspell("FLAG long\nAF 1\nAF Aab\nSFX Aa Y 1\nSFX Aa 0 s .\n", "2\nodd/1\nkot\n", &[]).unwrap();
    assert!(d.check("odd") && d.check("odds") && d.check("kot"));
}

#[test]
fn a_substandard_homonym_past_the_merged_cap_keeps_its_flag() {
    let n = MAX_FLAGS_PER_STEM / MAX_FLAGS_PER_ENTRY + 1;
    let x = 60_000;
    let d = AffixDict::from_hunspell(&format!("FLAG num\nSUBSTANDARD {x}\nSFX 1 Y 1\nSFX 1 0 a .\n"), &homonyms(n, &x.to_string()), &[]).unwrap();
    assert!(d.check("same") && d.check("samea") && !d.contains_suggestible("same"));
}

#[test]
fn flag_long_dictionaries_load_check_and_round_trip() {
    let d = AffixDict::from_hunspell(LONG_AFF, LONG_DIC, &[]).unwrap();
    assert_eq!(d.len(), 5);
    assert_long_dict(&d);
    let bytes = d.to_bytes().unwrap();
    assert!(miniz_oxide::inflate::decompress_to_vec(&bytes).unwrap().starts_with(b"WCAFF2\nFLAG long\n"));
    let back = AffixDict::from_bytes(&bytes).unwrap();
    assert_long_dict(&back);
    assert_eq!(back.to_bytes().unwrap(), bytes);
    assert!(AffixDict::from_hunspell("FLAG long\nSFX A Y 1\nSFX A 0 s .\n", "0\n", &[]).is_err(), "a one-character long flag");
}

#[test]
fn malformed_flag_numbers_are_refused_in_the_aff_and_read_as_hunspell_does_in_the_dic() {
    for rule in ["x", "65536", "99999999999999999999999", "+5", "-1", "1,2", " "] {
        let aff = format!("FLAG num\nSFX {rule} Y 1\nSFX {rule} 0 a .\n");
        assert!(AffixDict::from_hunspell(&aff, "1\nw/1\n", &[]).is_err(), "rule flag `{rule}`");
    }
    // Hunspell drops a rule whose continuation classes don't read; refusing the file is safe.
    for cont in ["2,,3", "2,", ",2", "a", "70000", "x2"] {
        let aff = format!("FLAG num\nSFX 1 Y 1\nSFX 1 0 a/{cont} .\n");
        assert!(AffixDict::from_hunspell(&aff, "1\nw/1\n", &[]).is_err(), "continuation `{cont}`");
    }
    for directive in ["FORBIDDENWORD 1,2", "FORBIDDENWORD abc", "NOSUGGEST -1", "NOSUGGEST", "FORBIDDENWORD 65536"] {
        assert!(AffixDict::from_hunspell(&format!("FLAG num\n{directive}\n"), "0\n", &[]).is_err(), "{directive}");
    }
    assert!(AffixDict::from_hunspell("SFX a Y 1\nSFX a 0 y .\nFLAG num\n", "0\n", &[]).is_err(), "FLAG after flags");
    assert!(AffixDict::from_hunspell("FLAG num\nFLAG num\nSFX 1 Y 1\nSFX 1 0 a .\n", "0\n", &[]).is_ok());
    // The .dic: as Hunspell 1.7 reads them (black-box, same files), an item that is no number
    // from 0 to 65,535 is flag 0 and the word stays: `1,,2` is 1, 0 and 2; `x2`, `-1` and
    // `70000` are 0; `000001` is 1. A real Nepali entry, `निजामती/I15,22`, keeps flag 22.
    let aff = "SET UTF-8\nFLAG num\nSFX 1 Y 1\nSFX 1 0 a .\nSFX 2 Y 1\nSFX 2 0 b .\nSFX 0 Y 1\nSFX 0 0 z .\n";
    let dic =
        "10\nok/1\nempty/1,,2\nbig/70000\nhuge/123456789012345678901234567890\nsign/-1\ncomma/1,\nletters/a1\nzeros/000001\nनिजामती/I15,2\nplain\n";
    let d = AffixDict::from_hunspell(aff, dic, &[]).unwrap();
    assert_eq!(d.len(), 10);
    for w in [
        "ok",
        "oka",
        "plain",
        "empty",
        "emptya",
        "emptyb",
        "emptyz",
        "bigz",
        "hugez",
        "signz",
        "comma",
        "commaa",
        "commaz",
        "lettersz",
        "zerosa",
        "निजामतीb",
        "निजामतीz",
    ] {
        assert!(d.check(w), "{w}");
    }
    for w in ["bigb", "biga", "signa", "lettersa", "zerosz", "निजामतीa", "plainz"] {
        assert!(!d.check(w), "{w}");
    }
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("emptyz") && back.check("निजामतीb") && !back.check("bigb"));
    // Text after a number's digits is ignored, as Hunspell does (Nepali's `0/17X`).
    let d = AffixDict::from_hunspell("FLAG num\nSFX 1 Y 1\nSFX 1 0 a/2X .\nSFX 2 N 1\nSFX 2 0 i .\n", "1\nw/1x,7\n", &[]).unwrap();
    assert!(d.check("wa") && d.check("wai") && !d.check("wi"));
    // Flag 0 is a flag, as Hunspell reads it (the Turkish dictionary's `SFX 0 N 1`).
    let d = AffixDict::from_hunspell("FLAG num\nSFX 0 N 1\nSFX 0 0 de .\n", "2\nev/0\nsu/0,1\n", &[]).unwrap();
    assert!(d.check("evde") && d.check("sude") && !d.check("kolde"));
    assert!(AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap().check("evde"));
}

/// craft-libs#487 review: 21,000 suffix rules whose continuation classes are one 1,024-flag
/// `AF` alias made every lookup cost rules × set size (172 ms a `check()`). A set is tested
/// once per lookup now, against a bitmap.
#[test]
fn big_continuation_sets_on_many_rules_stay_fast() {
    // Rules 1–1,024 continue with alias 1 (their own flags), so a chain step looks for 1,024
    // flags; the other 20,000 rules on the same suffix continue with alias 2, 1,024 flags none
    // of which is looked for: each of them is a full miss.
    let set = |from: usize| (from..from + 1024).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
    let mut aff = format!("SET UTF-8\nFLAG num\nAF 2\nAF {}\nAF {}\n", set(1), set(40_001));
    for n in 1..=21_024 {
        let alias = if n <= 1024 { 1 } else { 2 };
        aff.push_str(&format!("SFX {n} Y 1\nSFX {n} 0 a/{alias} .\n"));
    }
    let d = AffixDict::from_hunspell(&aff, "1\nstem/1\n", &[]).unwrap();
    assert!(d.check("stem") && d.check("stema") && d.check("stemaa"));
    let t = std::time::Instant::now();
    for w in ["qqaa", "qqaaa", "xaaa", "stemaaa", "zzzzaa", "qa", "aaaa", "bbaa"] {
        for _ in 0..5 {
            std::hint::black_box(d.check(w));
        }
    }
    assert!(t.elapsed().as_millis() < 2000, "{:?} for 40 checks", t.elapsed());
}

/// The same stem on 200,000 lines merges in linear time (no sort for a homonym that adds
/// nothing).
#[test]
fn many_repeated_homonyms_load_fast() {
    let big: Vec<String> = (1..=1024).map(|n| n.to_string()).collect();
    let mut dic = String::from("200008\n");
    for k in 0..8 {
        let part: Vec<String> = (0..1024).map(|n| (k * 1024 + n + 1).to_string()).collect();
        dic.push_str(&format!("same/{}\n", part.join(",")));
    }
    for _ in 0..200_000 {
        dic.push_str("same\n");
    }
    let t = std::time::Instant::now();
    let d = AffixDict::from_hunspell("FLAG num\n", &dic, &[]).unwrap();
    assert!(d.check("same") && big.len() == 1024);
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}

/// craft-libs#487 review: an `AF` table is read in the final FLAG mode (Hunspell would read an
/// alias before a `FLAG num` line one character at a time; no real dictionary does that), is
/// shared by the stems using it rather than copied, and is capped.
#[test]
fn aliases_are_read_once_in_the_final_mode_and_capped() {
    let d = AffixDict::from_hunspell("AF 1\nAF 1,2\nFLAG num\nSFX 1 Y 1\nSFX 1 0 a .\nSFX 2 Y 1\nSFX 2 0 b .\n", "2\nw/1\nv/1\n", &[]).unwrap();
    assert!(d.check("wa") && d.check("wb") && d.check("va"));
    let many: String = (0..=MAX_ALIASES).map(|_| "AF 1\n").collect();
    assert!(AffixDict::from_hunspell(&format!("FLAG num\nAF {}\n{many}", MAX_ALIASES + 1), "0\n", &[]).is_err());
    let big = format!("AF {}\n", "1,".repeat(MAX_ALIAS_BYTES / 2 + 1));
    assert!(AffixDict::from_hunspell(&format!("FLAG num\nAF 1\n{big}"), "0\n", &[]).is_err());
}

/// ONLYINCOMPOUND survives the compact format without compound rules too, and with them it
/// sits after COMPOUNDMIN, where one-character data has always written it.
#[test]
fn onlyincompound_round_trips_in_place() {
    let d = AffixDict::from_hunspell("ONLYINCOMPOUND c\n", "2\npart/c\nword\n", &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("word") && !back.check("part"));
    let d = AffixDict::from_hunspell("COMPOUNDMIN 1\nONLYINCOMPOUND c\nCOMPOUNDRULE 1\nCOMPOUNDRULE ab\n", "3\nx/a\ny/b\nz/bc\n", &[]).unwrap();
    let raw = String::from_utf8(miniz_oxide::inflate::decompress_to_vec(&d.to_bytes().unwrap()).unwrap()).unwrap();
    assert!(raw.contains("COMPOUNDMIN 1\nONLYINCOMPOUND c\nCOMPOUNDRULE ab\n"), "{raw}");
}

#[test]
fn thousands_of_flags_load() {
    // Serbian uses 1,903 numeric flags, Belarusian and Sinhala 5,000, each stem one or two:
    // 3,000 flags here, all appending `a` (one suffix group of 3,000 rules), plus flags up to
    // the largest number.
    let mut aff = String::from("SET UTF-8\nFLAG num\n");
    let mut dic = String::from("3002\n");
    for n in 1..=3000 {
        aff.push_str(&format!("SFX {n} N 1\nSFX {n} 0 a [^q]\n"));
        dic.push_str(&format!("w{n}/{n}\n"));
    }
    aff.push_str("SFX 65000 N 1\nSFX 65000 0 i .\n");
    dic.push_str("dva/3000,65000\nq/1\n");
    let d = AffixDict::from_hunspell(&aff, &dic, &[]).unwrap();
    assert_eq!(d.rule_count(), 3001);
    for w in ["w1a", "w1500a", "w3000a", "dva", "dvaa", "dvai", "q"] {
        assert!(d.check(w), "{w}");
    }
    assert!(!d.check("qa") && !d.check("w1i") && d.check("w1"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("w2999a") && back.check("dvai") && !back.check("w7i"));

    // More than 128 one-character flags (#4966): 300 letters, each its own suffix.
    let letters: Vec<char> = (0x100u32..0x100 + 300).filter_map(char::from_u32).collect();
    let mut aff = String::from("SET UTF-8\nFLAG UTF-8\n");
    for c in &letters {
        aff.push_str(&format!("SFX {c} N 1\nSFX {c} 0 {c} .\n"));
    }
    let all: String = letters.iter().take(256).collect();
    let d = AffixDict::from_hunspell(&aff, &format!("2\nbase/{all}\nlast/{}\n", letters[299]), &[]).unwrap();
    assert!(d.check(&format!("base{}", letters[0])) && d.check(&format!("base{}", letters[255])));
    assert!(!d.check(&format!("base{}", letters[299])) && d.check(&format!("last{}", letters[299])));
}

#[test]
fn flag_counts_are_capped() {
    let field = |n: usize| (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let aff = "FLAG num\nSFX 1 Y 1\nSFX 1 0 a .\n";
    // At most MAX_FLAGS_PER_ENTRY flags per .dic entry: one more and the entry is skipped.
    let dic = format!("2\nfull/{}\nover/{}\n", field(MAX_FLAGS_PER_ENTRY), field(MAX_FLAGS_PER_ENTRY + 1));
    let d = AffixDict::from_hunspell(aff, &dic, &[]).unwrap();
    assert!(d.check("fulla") && !d.check("over") && !d.check("overa"));
    let cont = format!("FLAG num\nSFX 1 Y 1\nSFX 1 0 a/{} .\n", field(MAX_FLAGS_PER_ENTRY + 1));
    assert!(AffixDict::from_hunspell(&cont, "0\n", &[]).is_err(), "too many continuation classes");
    // More distinct flags than a u16 id holds (only one-character flags can get there): an
    // error, not a panic or a wrapped id.
    let chars: Vec<char> =
        (0x4e00u32..0xa000).chain(0xac00..0xd7a4).chain(0x20000..0x2a6e0).filter_map(char::from_u32).take(MAX_DISTINCT_FLAGS + 1).collect();
    assert_eq!(chars.len(), MAX_DISTINCT_FLAGS + 1);
    let mut dic = String::new();
    for (i, chunk) in chars.chunks(MAX_FLAGS_PER_ENTRY).enumerate() {
        dic.push_str(&format!("w{i}/{}\n", chunk.iter().collect::<String>()));
    }
    let err = AffixDict::from_hunspell("", &dic, &[]).unwrap_err();
    assert!(err.contains("distinct flags"), "{err}");
    // Homonyms are merged into one set. The per-entry cap is for .dic fields, not merged sets:
    // two entries adding up to more than MAX_FLAGS_PER_ENTRY are both kept (up to
    // MAX_FLAGS_PER_STEM), and the compact data reloads with the word and all its flags.
    let aff2 = format!("{aff}SFX {n} Y 1\nSFX {n} 0 b .\n", n = MAX_FLAGS_PER_ENTRY + 5);
    let first = field(MAX_FLAGS_PER_ENTRY);
    let second = (MAX_FLAGS_PER_ENTRY + 1..=MAX_FLAGS_PER_ENTRY + 10).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let dic = format!("2\nsame/{first}\nsame/{second}\n");
    let d = AffixDict::from_hunspell(&aff2, &dic, &[]).unwrap();
    assert!(d.check("same") && d.check("samea") && d.check("sameb"), "the second homonym's flags are merged");
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("same") && back.check("samea") && back.check("sameb"));
}

/// `n` homonym lines of `same`, each with MAX_FLAGS_PER_ENTRY distinct numeric flags (the
/// first starting at 1), the last one ending with the flags `extra` (comma-separated).
fn homonyms(n: usize, extra: &str) -> String {
    let mut dic = format!("{n}\n");
    for e in 0..n {
        let flags = e * MAX_FLAGS_PER_ENTRY + 1..=(e + 1) * MAX_FLAGS_PER_ENTRY;
        let mut flags: Vec<String> = flags.map(|i| i.to_string()).collect();
        if e + 1 == n && !extra.is_empty() {
            // Still exactly MAX_FLAGS_PER_ENTRY flags: the entry itself is within its cap.
            let extra: Vec<&str> = extra.split(',').collect();
            flags.truncate(MAX_FLAGS_PER_ENTRY - extra.len());
            flags.extend(extra.iter().map(ToString::to_string));
        }
        dic.push_str(&format!("same/{}\n", flags.join(",")));
    }
    dic
}

#[test]
fn merged_homonyms_between_the_entry_and_stem_caps_round_trip() {
    // Eight full entries: exactly MAX_FLAGS_PER_STEM merged flags, the largest set build()
    // makes. The compact reader takes it back, word and flags.
    let n = MAX_FLAGS_PER_STEM / MAX_FLAGS_PER_ENTRY;
    let last = MAX_FLAGS_PER_STEM;
    let aff = format!("FLAG num\nSFX 1 Y 1\nSFX 1 0 a .\nSFX {last} Y 1\nSFX {last} 0 z .\n");
    let d = AffixDict::from_hunspell(&aff, &homonyms(n, ""), &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    for d in [&d, &back] {
        assert!(d.check("same") && d.check("samea") && d.check("samez"));
    }
    // One flag past the stem cap in compact data is refused, as an oversized .dic entry is.
    let flags = (1..=MAX_FLAGS_PER_STEM + 1).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let raw = format!("{MAGIC}\nFLAG num\nSFX 1 Y 0 a .\nWORDS\n\0same/{flags}\n\0kot\n");
    let d = AffixDict::from_bytes(&miniz_oxide::deflate::compress_to_vec(raw.as_bytes(), 6)).unwrap();
    assert!(!d.check("same") && d.check("kot"));
}

#[test]
fn a_homonym_past_the_merged_cap_keeps_its_exclusions() {
    // Nine full homonyms: the ninth takes the merged set past MAX_FLAGS_PER_STEM, and it is
    // the only one with the exclusion flag. Its other flags are dropped; the exclusion isn't.
    let n = MAX_FLAGS_PER_STEM / MAX_FLAGS_PER_ENTRY + 1;
    let x = 60_000;
    let rules = "SFX 1 Y 1\nSFX 1 0 a .\n";
    let forbidden = AffixDict::from_hunspell(&format!("FLAG num\nFORBIDDENWORD {x}\n{rules}"), &homonyms(n, &x.to_string()), &[]).unwrap();
    let back = AffixDict::from_bytes(&forbidden.to_bytes().unwrap()).unwrap();
    for d in [&forbidden, &back] {
        assert!(!d.check("same") && !d.check("samea") && !d.contains("same"), "still forbidden");
        assert!(!d.contains_suggestible("same"));
    }
    let no_suggest = AffixDict::from_hunspell(&format!("FLAG num\nNOSUGGEST {x}\n{rules}"), &homonyms(n, &x.to_string()), &[]).unwrap();
    let back = AffixDict::from_bytes(&no_suggest.to_bytes().unwrap()).unwrap();
    for d in [&no_suggest, &back] {
        assert!(d.check("same") && d.check("samea"), "correct");
        assert!(!d.contains_suggestible("same") && !d.contains_suggestible("samea"), "but never suggested");
    }
    // Both exclusions on the overflowing homonym: both carried.
    let both = format!("FLAG num\nFORBIDDENWORD {x}\nNOSUGGEST {y}\n{rules}", y = x + 1);
    let d = AffixDict::from_hunspell(&both, &homonyms(n, &format!("{x},{}", x + 1)), &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    for d in [&d, &back] {
        assert!(!d.check("same") && !d.contains_suggestible("same"));
    }
    // A single entry over MAX_FLAGS_PER_ENTRY is not a word, but its exclusion still rules
    // its homonym out.
    let over = format!("{},{x}", (2..=MAX_FLAGS_PER_ENTRY + 1).map(|i| i.to_string()).collect::<Vec<_>>().join(","));
    let d = AffixDict::from_hunspell(&format!("FLAG num\nFORBIDDENWORD {x}\n{rules}"), &format!("2\nsame/1\nsame/{over}\n"), &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    for d in [&d, &back] {
        assert!(!d.check("same") && !d.check("samea"), "an oversized entry's FORBIDDENWORD applies");
    }
    // Alone, an oversized entry is still no word, exclusion or not.
    let d = AffixDict::from_hunspell(&format!("FLAG num\nNOSUGGEST {x}\n{rules}"), &format!("1\nsame/{over}\n"), &[]).unwrap();
    assert!(!d.check("same"));
}

#[test]
fn an_unused_directive_flag_is_not_written_in_character_mode() {
    // FORBIDDENWORD names a flag no stem or rule uses: the compact data leaves it out, as it
    // did before flag ids, in one-character mode only.
    let d = AffixDict::from_hunspell("FORBIDDENWORD F\nNOSUGGEST N\nSFX a Y 1\nSFX a 0 y .\n", "1\nkot/a\n", &[]).unwrap();
    let raw = String::from_utf8(miniz_oxide::inflate::decompress_to_vec(&d.to_bytes().unwrap()).unwrap()).unwrap();
    assert!(!raw.contains("FORBIDDENWORD") && !raw.contains("NOSUGGEST"), "{raw}");
    // A used one is still written.
    let d = AffixDict::from_hunspell("FORBIDDENWORD F\nSFX a Y 1\nSFX a 0 y .\n", "2\nkot/a\nzly/F\n", &[]).unwrap();
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.check("koty") && !back.check("zly"));
}

#[test]
fn compact_data_without_a_flag_line_still_loads() {
    // Data from before homonyms were kept apart (WCAFF1) is refused, not read with old answers.
    let v1 = miniz_oxide::deflate::compress_to_vec(b"WCAFF1\nWORDS\n\x00kot\n", 6);
    assert!(AffixDict::from_bytes(&v1).is_err());
    // One-character flags need no FLAG line.
    let old = miniz_oxide::deflate::compress_to_vec(b"WCAFF2\n# old data\nFORBIDDENWORD F\nSFX a Y 0 y .\nWORDS\n\x00kot/a\n\x00zly/F\n", 6);
    let d = AffixDict::from_bytes(&old).unwrap();
    assert!(d.check("kot") && d.check("koty") && !d.check("zly"));
    // One-character dictionaries still write no FLAG line, so their data is unchanged.
    let raw = miniz_oxide::inflate::decompress_to_vec(&dict().to_bytes().unwrap()).unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("\nFLAG"));
    // A FLAG line after a flag, or an unknown one, is bad data.
    for head in ["WCAFF2\nSFX a Y 0 y .\nFLAG num\n", "WCAFF2\nFLAG bogus\n"] {
        let bytes = miniz_oxide::deflate::compress_to_vec(format!("{head}WORDS\n\x00kot\n").as_bytes(), 6);
        assert!(AffixDict::from_bytes(&bytes).is_err(), "{head:?}");
    }
}

proptest::proptest! {
    #[test]
    fn lookups_never_panic(s in "\\PC{0,40}", bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..200)) {
        let d = dict();
        let _ = d.check(&s);
        let _ = d.forms(&s);
        let _ = AffixDict::from_bytes(&bytes);
        let _ = AffixDict::from_bytes(&miniz_oxide::deflate::compress_to_vec(&bytes, 1));
    }

    /// FULLSTRIP, NEEDAFFIX and CIRCUMFIX on hostile stems and words: no panic, and every
    /// form `forms()` lists is one the checker accepts.
    #[test]
    fn needaffix_circumfix_and_fullstrip_never_panic(stem in "[abx]{0,4}", word in "\\PC{0,12}") {
        let aff = "SET UTF-8\nFULLSTRIP\nNEEDAFFIX N\nCIRCUMFIX C\nPFX P Y 2\nPFX P a 0/C .\nPFX P 0 x/N a\nSFX S Y 3\nSFX S b 0/C b\nSFX S 0 x/NT .\nSFX S ab y .\nSFX T Y 1\nSFX T x 0 .\n";
        if let Ok(d) = AffixDict::from_hunspell(aff, &format!("2\n{stem}/PSN\n{stem}b/PS\n"), &[]) {
            let _ = d.check(&word);
            for s in [stem.clone(), format!("{stem}b")] {
                for f in d.forms(&s) {
                    proptest::prop_assert!(!f.is_empty() && d.contains(&f), "{s} → {f:?}");
                }
            }
        }
    }

    #[test]
    fn any_flag_field_reads_without_panicking(field in "[0-9,a-zA-Z!\u{fe0f}\u{1f6ab}]{0,24}", word in "\\PC{0,12}") {
        for mode in ["num", "long", "UTF-8"] {
            let aff = format!("FLAG {mode}
FORBIDDENWORD {field}
SFX {field} Y 1
SFX {field} 0 a/{field} .
");
            for aff in [aff.as_str(), &format!("FLAG {mode}
SFX 12 Y 1
SFX 12 0 a .
")] {
                if let Ok(d) = AffixDict::from_hunspell(aff, &format!("2
w/{field}
x/12
"), &[]) {
                    let _ = (d.check(&word), d.check("wa"), d.forms("w"), d.forms("x"));
                    // Data the writer takes, the reader takes back.
                    if let Ok(bytes) = d.to_bytes() {
                        let back = AffixDict::from_bytes(&bytes);
                        proptest::prop_assert!(back.is_ok(), "{mode} {field:?}: {back:?}");
                    }
                }
            }
        }
    }
}
/// `FULLSTRIP` lets an affix replace a whole stem; without it a stem must outlive its strip.
#[test]
fn fullstrip_allows_an_affix_to_replace_the_whole_stem() {
    let aff = "SET UTF-8\nSFX A Y 1\nSFX A ab x .\nPFX B Y 1\nPFX B ab y .\n";
    let dic = "1\nab/AB\n";
    let without = AffixDict::from_hunspell(aff, dic, &[]).unwrap();
    for w in ["ab", "x", "y"] {
        assert_eq!(without.contains(w), w == "ab", "{w} without FULLSTRIP");
    }
    let with = AffixDict::from_hunspell(&format!("FULLSTRIP\n{aff}"), dic, &[]).unwrap();
    for w in ["ab", "x", "y"] {
        assert!(with.contains(w), "{w} with FULLSTRIP");
    }
    assert!(with.forms("ab").iter().all(|f| with.contains(f)));
    assert!(!without.forms("ab").iter().any(|f| f == "x" || f == "y"));
    let back = AffixDict::from_bytes(&with.to_bytes().unwrap()).unwrap();
    assert!(back.contains("x") && back.contains("y"));
}

/// `NEEDAFFIX`: a stem or a suffix form that needs another affix is not a word without one.
#[test]
fn needaffix_stems_and_forms_need_one_more_affix() {
    let aff = "SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 s .\nSFX B Y 1\nSFX B 0 x/AN .\nPFX P Y 1\nPFX P 0 re .\n";
    let d = AffixDict::from_hunspell(aff, "3\nfoo/NA\nbar/AB\nbaz/NP\n", &[]).unwrap();
    for w in ["foos", "bar", "bars", "barxs", "rebaz"] {
        assert!(d.contains(w), "{w}");
    }
    assert!(!d.contains("foo"), "a stem that needs an affix");
    assert!(!d.contains("baz"), "a stem that needs an affix");
    // `barx` carries NEEDAFFIX through its continuation class: it needs `s` after it.
    assert!(!d.contains("barx"), "a form that needs another affix");
    for stem in d.stems().map(str::to_string).collect::<Vec<_>>() {
        for f in d.forms(&stem) {
            assert!(d.contains(&f), "{stem} → {f}");
        }
    }
    assert!(!d.forms("foo").iter().any(|f| f == "foo") && d.forms("foo").iter().any(|f| f == "foos"));
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(!back.contains("foo") && back.contains("foos"));
}

/// `CIRCUMFIX`: a circumfix prefix and suffix only work as a pair.
#[test]
fn circumfix_prefix_and_suffix_only_work_together() {
    let aff = "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX S Y 1\nSFX S 0 y/c .\nSFX T Y 1\nSFX T 0 z .\n";
    let d = AffixDict::from_hunspell(aff, "1\nstrom/PST\n", &[]).unwrap();
    for w in ["strom", "stromz", "nestromy"] {
        assert!(d.contains(w), "{w}");
    }
    for w in ["stromy", "nestrom", "nestromz"] {
        assert!(!d.contains(w), "{w}");
    }
    for f in d.forms("strom") {
        assert!(d.contains(&f), "strom → {f}");
    }
    let mut forms = d.forms("strom");
    forms.sort();
    assert_eq!(forms, ["nestromy", "strom", "stromz"]);
    let back = AffixDict::from_bytes(&d.to_bytes().unwrap()).unwrap();
    assert!(back.contains("nestromy") && !back.contains("stromy") && !back.contains("nestrom"));
}

/// A chained suffix form is listed when the checker accepts it: also when its last suffix
/// needs another affix or is a circumfix one (as hunspell 1.7.5 reads a chain), not when it
/// would replace a whole stem without `FULLSTRIP`.
#[test]
fn chained_forms_follow_needaffix_circumfix_and_fullstrip() {
    let chain = |aff: &str, dic: &str, stem: &str| {
        let d = AffixDict::from_hunspell(aff, dic, &[]).unwrap();
        let mut forms = d.forms(stem);
        forms.sort();
        forms.dedup();
        for f in &forms {
            assert!(d.contains(f), "{stem} → {f}");
        }
        (d, forms)
    };
    let (d, forms) = chain("SET UTF-8\nNEEDAFFIX N\nSFX A Y 1\nSFX A 0 able/B .\nSFX B Y 1\nSFX B 0 s/N .\n", "1\nwork/A\n", "work");
    // The last suffix's NEEDAFFIX is met by the one under it (hunspell 1.7.5 takes it).
    assert_eq!(forms, ["work", "workable", "workables"]);
    assert!(d.contains("workables"), "its last suffix has another affix under it");
    let (d, forms) = chain("SET UTF-8\nCIRCUMFIX c\nSFX A Y 1\nSFX A 0 able/B .\nSFX B Y 1\nSFX B 0 s/c .\n", "1\nwork/A\n", "work");
    // Hunspell 1.7.5 ignores a chain's last suffix's CIRCUMFIX: `workables` is right.
    assert_eq!(forms, ["work", "workable", "workables"]);
    assert!(d.contains("workables"), "a chain's last suffix needs no prefix");
    let aff = "SFX A Y 1\nSFX A 0 y/B .\nSFX B Y 1\nSFX B xy z .\n";
    let (d, forms) = chain(&format!("SET UTF-8\n{aff}"), "1\nx/A\n", "x");
    assert_eq!(forms, ["x", "xy"]);
    assert!(!d.contains("z"), "a whole stem is replaced only with FULLSTRIP");
    let (d, forms) = chain(&format!("SET UTF-8\nFULLSTRIP\n{aff}"), "1\nx/A\n", "x");
    assert_eq!(forms, ["x", "xy", "z"]);
    assert!(d.contains("z"));
}

/// `forms()` never lists an empty form, which `FULLSTRIP` can produce when a suffix replaces
/// the whole stem with nothing, at the first suffix or further along a chain.
#[test]
fn forms_never_list_an_empty_word() {
    let aff = "SET UTF-8\nFULLSTRIP\nSFX A Y 2\nSFX A x 0 .\nSFX A 0 y/B .\nSFX B Y 1\nSFX B xy 0 .\n";
    let d = AffixDict::from_hunspell(aff, "1\nx/A\n", &[]).unwrap();
    let mut forms = d.forms("x");
    forms.sort();
    assert_eq!(forms, ["x", "xy"]);
    assert!(!d.contains(""));
}

/// A circumfix suffix in the middle of a suffix chain still needs its prefix: `workable`
/// carries the circumfix, so neither it nor `workables` is a word without `ne`.
#[test]
fn circumfix_suffix_inside_a_chain_still_needs_its_prefix() {
    let aff = "SET UTF-8\nCIRCUMFIX c\nPFX P Y 1\nPFX P 0 ne/c .\nSFX A Y 1\nSFX A 0 able/Bc .\nSFX B Y 1\nSFX B 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "1\nwork/AP\n", &[]).unwrap();
    let mut forms = d.forms("work");
    forms.sort();
    assert_eq!(forms, ["neworkable", "work"]);
    for f in &forms {
        assert!(d.contains(f), "{f}");
    }
    for w in ["workable", "workables", "nework"] {
        assert!(!d.contains(w), "{w}");
    }
}

/// An empty form (`FULLSTRIP` stripping a whole stem to nothing) is no word itself, but what
/// a continuation class or a prefix adds to it is: `x` + `SFX A x 0/B`, `SFX B 0 y` makes `y`.
#[test]
fn an_empty_suffix_form_still_feeds_chains_and_prefixes() {
    let aff = "SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A x 0/B .\nSFX B Y 1\nSFX B 0 y .\n";
    let d = AffixDict::from_hunspell(aff, "1\nx/A\n", &[]).unwrap();
    let mut forms = d.forms("x");
    forms.sort();
    assert_eq!(forms, ["x", "y"]);
    assert!(d.contains("y") && !d.contains(""));
    let aff = "SET UTF-8\nFULLSTRIP\nSFX A Y 1\nSFX A x 0 .\nPFX P Y 1\nPFX P 0 q x\n";
    let d = AffixDict::from_hunspell(aff, "1\nx/AP\n", &[]).unwrap();
    let mut forms = d.forms("x");
    forms.sort();
    assert_eq!(forms, ["qx", "x"], "`q` over the empty form fails the condition `x`");
    assert!(d.contains("qx") && !d.contains("q"));
}

/// A prefix condition is checked on what the prefix is added to: the stem for the prefix
/// alone, the suffix form in a cross product, as `contains()` does. Under the condition `ab`,
/// `ac` + `-c+b` = `ab` takes `z`, and `ab` + `-b+c` = `ac` does not.
#[test]
fn prefix_conditions_apply_to_the_suffix_form_in_cross_products() {
    let aff = "SET UTF-8\nPFX P Y 1\nPFX P 0 z ab\nSFX A Y 2\nSFX A c b c\nSFX A b c b\n";
    let d = AffixDict::from_hunspell(aff, "2\nac/AP\nab/AP\n", &[]).unwrap();
    for stem in ["ac", "ab"] {
        for f in d.forms(stem) {
            assert!(d.contains(&f), "{stem} → {f}");
        }
    }
    assert!(d.forms("ac").iter().any(|f| f == "zab"), "the cross product meets the condition");
    assert!(!d.forms("ab").iter().any(|f| f == "zac") && !d.contains("zac"), "the cross product fails it");
}
