//! Typed-separator and Syllable Separator rendering of a romanization (§49, §55).

use crate::api::InputMode;

/// §55 — `roman` with the separator at every typed boundary replaced by
/// the `-` run the user typed there. `runs` are `(typed offset, run
/// length)` pairs (`(4, 1)` for a `-`, `(7, 2)` for a `--`), `typed` the
/// lowercased typed body the offsets index into, `reading` the canonical
/// TL the walk aligns ([`crate::typed_syllable_walk`]). `roman` is
/// `reading` under a separator-preserving transform — recased, or a
/// custom entry's native POJ form — so it is tokenized by the same
/// separators as the reading's syllables and rewritten by index rather
/// than walked itself. A boundary that is not a syllable end of the
/// reading, or ends its last syllable, changes nothing; a reading that
/// runs past the typed body (a partial-prefix extension) is rewritten on
/// the syllables the typed body covers. `None` when nothing changed or
/// the reading does not align with the typed body (an abbreviation body,
/// a TPS / English buffer).
pub fn render_typed_separators(
    mode: InputMode,
    typed: &str,
    runs: &[(usize, usize)],
    reading: &str,
    roman: &str,
) -> Option<String> {
    let walk = crate::typed_syllable_walk(mode, typed, reading)?;
    // Token byte ranges of `roman` — the `split` + drop-empty rule of
    // `tl_syllable_tokens`, so the count matches the walk's syllables.
    // Every separator is one ASCII byte.
    let mut tokens: Vec<std::ops::Range<usize>> = Vec::with_capacity(walk.ends.len());
    let mut offset = 0;
    for piece in roman.split(crate::tps::TL_SYLLABLE_SEPARATORS) {
        if !piece.is_empty() {
            tokens.push(offset..offset + piece.len());
        }
        offset += piece.len() + 1;
    }
    // The walk stops where the typed text ends, so a partial-prefix row
    // (`tāi-gi̍k` under `taigi`) aligns only its leading syllables.
    debug_assert!(tokens.len() >= walk.ends.len(), "{roman:?} vs {reading:?}");
    if tokens.len() < walk.ends.len() {
        return None;
    }
    // The separator after token `k`, and the typed run that replaces it.
    let separator_after =
        |k: usize| tokens[k].end..tokens.get(k + 1).map_or(roman.len(), |t| t.start);
    let typed_run = |k: usize| {
        (k + 1 < tokens.len())
            .then(|| runs.iter().find(|&&(at, _)| Some(&at) == walk.ends.get(k)))
            .flatten()
            .map(|&(_, run)| run)
    };
    let changes = (0..tokens.len()).any(|k| {
        typed_run(k).is_some_and(|run| {
            let stored = &roman[separator_after(k)];
            stored.len() != run || stored.bytes().any(|b| b != b'-')
        })
    });
    if !changes {
        return None;
    }
    let mut out = String::with_capacity(roman.len());
    for k in 0..tokens.len() {
        out.push_str(&roman[tokens[k].clone()]);
        match typed_run(k) {
            Some(run) => out.extend(std::iter::repeat_n('-', run)),
            None => out.push_str(&roman[separator_after(k)]),
        }
    }
    Some(out)
}

/// Syllable Separator (`AppConfig.syllable_separator`, `behavioral-invariants.md`
/// §49) — the rendered form of a dictionary-supplied romanization with each
/// run of `-` rewritten by [`push_syllable_joiner_run`]: `joiner` `""` (None)
/// gives `tâi-uân` → `tâiuân`, `hōo--guá` → `hōo·guá`; `" "` (Space) gives
/// `tâi uân`, `hōo ·guá`. The joiner only ever stands between two syllables:
/// a run that opens the text or a word (`--ah` → `·ah`) or closes one adds
/// none. Spaces (word boundaries) are kept. Display only: identity keys
/// (`display_text`, `canonical_tl`) never pass through here, and user-typed
/// hyphens are rendered by `to_tone_marks` as typed.
pub fn syllable_joiner_display(roman: &str, joiner: &str) -> String {
    let mut out = String::with_capacity(roman.len());
    let mut chars = roman.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '-' {
            out.push(c);
            continue;
        }
        let mut run_len = 1;
        while chars.next_if_eq(&'-').is_some() {
            run_len += 1;
        }
        // A piece rendered on its own (a nailed `--guá`) opens here too;
        // `composing::nailed::push_boundary` writes its joiner.
        let opens_word = out.is_empty() || out.ends_with(' ');
        let closes_word = matches!(chars.peek(), None | Some(' '));
        let joiner = if opens_word || closes_word {
            ""
        } else {
            joiner
        };
        push_syllable_joiner_run(&mut out, run_len, joiner);
    }
    out
}

/// Appends one run of `run_len` hyphens between two syllables, rendered:
/// `joiner`, then one [`crate::tps::KHINSIANN_DOT`] per neutral-tone `--` (an
/// odd hyphen left over is the plain syllable boundary the joiner already
/// wrote). The dot stays glued to the syllable after it: `---` → ` ·`, never
/// `· `. `composing::nailed::push_boundary` calls it for a typed run between
/// nailed segments, where no run ever opens the text.
pub fn push_syllable_joiner_run(out: &mut String, run_len: usize, joiner: &str) {
    out.push_str(joiner);
    for _ in 0..run_len / 2 {
        out.push_str(crate::tps::KHINSIANN_DOT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_joiner_display_none_drops_hyphens_and_dots_the_khinsiann_marker() {
        let none = |roman| syllable_joiner_display(roman, "");
        assert_eq!(none("tâi-uân"), "tâiuân");
        assert_eq!(none("tshit-niû-má-senn"), "tshitniûmásenn");
        assert_eq!(none("hōo--guá"), "hōo\u{00b7}guá");
        assert_eq!(none("--ah"), "\u{00b7}ah");
        assert_eq!(none("tsáu--tshut-khì"), "tsáu\u{00b7}tshutkhì");
        // Longer runs: one dot per `--`, an odd hyphen adds nothing.
        assert_eq!(none("sia̍p---khì"), "sia̍p\u{00b7}khì");
        assert_eq!(none("a----b"), "a\u{00b7}\u{00b7}b");
        // Word boundaries and hyphen-free input are left alone.
        assert_eq!(none("tâi gí"), "tâi gí");
        assert_eq!(none("keng-lâm su-īⁿ"), "kenglâm suīⁿ");
    }

    #[test]
    fn syllable_joiner_display_space_spaces_syllables_and_glues_the_dot_forward() {
        let space = |roman| syllable_joiner_display(roman, " ");
        assert_eq!(space("tâi-uân"), "tâi uân");
        assert_eq!(space("tshit-niû-má-senn"), "tshit niû má senn");
        assert_eq!(space("hōo--guá"), "hōo \u{00b7}guá");
        assert_eq!(space("tsáu--tshut-khì"), "tsáu \u{00b7}tshut khì");
        assert_eq!(space("sia̍p---khì"), "sia̍p \u{00b7}khì");
        assert_eq!(
            space("tshiànn--tsha̍t---kòo-kenn"),
            "tshiànn \u{00b7}tsha̍t \u{00b7}kòo kenn"
        );
        // A run opening the text or a word, or closing one, adds no space.
        assert_eq!(space("--ah"), "\u{00b7}ah");
        assert_eq!(space("khioh piān--ê"), "khioh piān \u{00b7}ê");
        assert_eq!(space("tâi --ah"), "tâi \u{00b7}ah");
        assert_eq!(space("tai-"), "tai");
        assert_eq!(space("keng-lâm su-īⁿ"), "keng lâm su īⁿ");
    }

    #[test]
    fn push_syllable_joiner_run_writes_the_joiner_then_one_dot_per_pair() {
        let run = |run_len, joiner| {
            let mut out = String::new();
            push_syllable_joiner_run(&mut out, run_len, joiner);
            out
        };
        assert_eq!(run(1, ""), "");
        assert_eq!(run(2, ""), "\u{00b7}");
        assert_eq!(run(1, " "), " ");
        assert_eq!(run(2, " "), " \u{00b7}");
        assert_eq!(run(3, " "), " \u{00b7}");
        assert_eq!(run(4, " "), " \u{00b7}\u{00b7}");
    }
}

#[cfg(test)]
mod render_typed_separators_tests {
    use super::render_typed_separators;
    use crate::InputMode::{self, Poj, Tl};

    fn render(
        mode: InputMode,
        typed: &str,
        runs: &[(usize, usize)],
        roman: &str,
    ) -> Option<String> {
        render_typed_separators(mode, typed, runs, roman, roman)
    }

    // §55 — the reported shape: 放重利 stores a space, the user typed `-`.
    // trace: face `pang3tang7lai7`; typed `pangtanglai` → syllable ends
    // [4, 8, 11]; runs at 4 and 8 → tokens `pàng` | `tāng` | `lāi`,
    // separators ` ` and `-` → `-` and `-`.
    #[test]
    fn typed_hyphen_replaces_a_records_space() {
        for (typed, runs) in [
            ("pangtanglai", &[(4, 1), (8, 1)][..]),
            // Only the boundary the user typed is rewritten.
            ("pangtanglai", &[(4, 1)][..]),
            // The digits sit in the typed body; the runs index past them.
            ("pang3tang7lai", &[(5, 1), (10, 1)][..]),
            // A partial-prefix row: the typed body ends inside `lāi`.
            ("pangtangl", &[(4, 1)][..]),
        ] {
            assert_eq!(
                render(Tl, typed, runs, "pàng tāng-lāi").as_deref(),
                Some("pàng-tāng-lāi"),
                "{typed} {runs:?}"
            );
        }
    }

    // The roman may be recased or POJ-rendered; the walk reads the
    // canonical TL and the tokens come from the roman.
    #[test]
    fn rewrites_the_roman_by_the_readings_syllables() {
        assert_eq!(
            render_typed_separators(
                Tl,
                "pangtanglai",
                &[(4, 1)],
                "pàng tāng-lāi",
                "PÀNG TĀNG-LĀI"
            )
            .as_deref(),
            Some("PÀNG-TĀNG-LĀI")
        );
        // POJ mode walks the POJ face (`góa` ← `guá`) of the TL reading.
        assert_eq!(
            render(Poj, "goasi", &[(3, 2)], "guá sī").as_deref(),
            Some("guá--sī")
        );
    }

    // The walker's space join between two edges takes the typed run; a
    // part that opens with its own `--` (custom `--ah`, joined ` --ah`)
    // has its whole separator run replaced, never doubled.
    #[test]
    fn walker_space_join_takes_the_typed_run_without_doubling() {
        assert_eq!(
            render(Tl, "guasi", &[(3, 2)], "guá sī").as_deref(),
            Some("guá--sī")
        );
        assert_eq!(
            render(Tl, "khiah", &[(3, 2)], "khì --ah").as_deref(),
            Some("khì--ah")
        );
        // The run is verbatim (`---` stays `---`, §51).
        assert_eq!(
            render(Tl, "guasi", &[(3, 3)], "guá sī").as_deref(),
            Some("guá---sī")
        );
    }

    // Nothing to rewrite → `None`: typed and stored already agree, the
    // boundary is the reading's last syllable end (a trailing `tai-`) or
    // no syllable end at all, the reading ends before the boundary, or
    // the typed body is not the reading's (an abbreviation).
    #[test]
    fn leaves_the_roman_alone_when_nothing_changes() {
        for (typed, runs, roman) in [
            ("taigi", &[(3, 1)][..], "tâi-gí"),
            ("hoogua", &[(3, 2)][..], "hōo--guá"),
            ("tai", &[(3, 1)][..], "tâi"),
            ("khiah", &[(2, 1)][..], "khiah"),
            ("pangtanglai", &[(8, 1)][..], "pàng"),
            ("ptl", &[(1, 1)][..], "pàng tāng-lāi"),
        ] {
            assert_eq!(
                render(Tl, typed, runs, roman),
                None,
                "{typed} {runs:?} {roman}"
            );
        }
    }
}
