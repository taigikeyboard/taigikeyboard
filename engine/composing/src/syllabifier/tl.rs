//! TL syllabifier — BFS over a `SyllableInventory` returning every span
//! ending reachable from `pos` by a chain of 1..=`max_syllables` valid
//! TL syllables.
//!
//! Why BFS rather than longest-match: `tsua` typed by the user must surface
//! both `珠 (tsu, span=3)` and `紙 / 珠仔 (tsua, span=4)` as Phase 5 candidate
//! sources — pure longest-match (khiin-rs `references/khiin-rs/khiin/src/data/segmenter.rs:122`)
//! would commit to `tsua` and lose the `珠` candidate. Global lattice
//! (librime `src/rime/algo/syllabifier.cc`) is over-built for our scope.
//! BFS with depth cap = canonical middle ground per `docs/roadmap.md` line
//! 203.

// 中文: TL 音節切分器 — 從 pos 出發 BFS,深度上限 max_syllables,以 SyllableInventory 判斷音節合法性。
// 中文: 之所以不用 longest-match,是因為使用者敲 tsua 時必須同時保留 「珠 (span=3)」與「紙/珠仔 (span=4)」兩條候選路徑。

use std::collections::{BTreeSet, VecDeque};

use lexicon::SyllableInventory;

/// Upper bound on a single TL syllable's byte length: max initial
/// `tsh` (3) + max final `uainnh` / `iaunnh` (6) + optional ASCII tone
/// digit (1) = 10 bytes. Sourced from `engine/phonetics/src/tables.rs:13`
/// (TL_INITIALS) and `tables.rs:21-33` (TL_FINALS); revisit if either
/// table grows.
// 中文: 單一 TL 音節最大 byte 長度上限 — 聲母 tsh(3) + 韻母 uainnh/iaunnh(6) + 可選聲調數字(1) = 10。
const MAX_SYLLABLE_BYTES: usize = 10;

/// Return every byte offset `e > pos` reachable from `pos` by a chain
/// of 1..=`max_syllables` syllables, where each chain link
/// `input[cur..end]` (after ASCII lowercasing) is a member of `inv`.
///
/// Contract:
/// - Returns ascending, deduplicated byte offsets.
/// - Returns empty `Vec` when `pos > input.len()`, `pos == input.len()`,
///   `max_syllables == 0`, or `pos` is not on a UTF-8 char boundary.
/// - Caller must pass canonicalized TL input (lowercase or mixed-case
///   ASCII; POJ→TL spelling already applied via
///   `phonetics::canonicalize_syllable`). Non-ASCII bytes won't match
///   the inventory and will silently produce no endings, but the
///   function will not panic.
///
/// Algorithm: FIFO BFS using `endings` itself as the visited set —
/// `BTreeSet::insert` returns `true` only on first arrival, which under
/// unit edge costs is also the minimum depth. Time: O(n × MAX_SYLLABLE_BYTES)
/// FST lookups, each O(syllable_len). Allocation: one
/// `to_ascii_lowercase` pass on `input`.
// 中文: 從 pos 出發,以 1..=max_syllables 條音節鏈走訪,回傳所有可達 byte 位移 (遞增去重)。
// 中文: 不在 UTF-8 邊界、超界、深度為 0 時都安全回傳空 Vec;呼叫端負責先把 POJ canonicalize 成 TL。
pub fn valid_span_endings(
    input: &str,
    pos: usize,
    inv: &SyllableInventory,
    max_syllables: usize,
) -> Vec<usize> {
    if max_syllables == 0 || pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let lowered = input.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
    let mut endings: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((pos, 0));

    while let Some((cur, depth)) = queue.pop_front() {
        if depth >= max_syllables {
            continue;
        }
        let upper = (cur + MAX_SYLLABLE_BYTES).min(lowered.len());
        for end in (cur + 1)..=upper {
            if !lowered.is_char_boundary(end) {
                continue;
            }
            if inv.contains(&lowered[cur..end])
                && !is_false_toneless_boundary(bytes, end)
                && endings.insert(end)
            {
                queue.push_back((end, depth + 1));
            }
        }
    }

    endings.into_iter().collect()
}

/// True when the FST hit at `..end` is a toneless syllable match that
/// sits immediately before an ASCII tone digit `1..=9`. Such matches
/// are false boundaries — the tone digit belongs to the matched
/// syllable, and the FST will also contain the longer numeric form
/// (e.g. `tai` followed by `5` is really `tai5`). Per
/// `phonetics::syllable.rs:18-20` only digits 1..=9 carry tone
/// meaning; '0' is not a tone marker.
// 中文: 判斷 ..end 的命中是否為「toneless 後接 tone digit」假邊界;是的話該 ending 不算數,等下一輪匹配完整數字調 key。
fn is_false_toneless_boundary(bytes: &[u8], end: usize) -> bool {
    let last_is_tone_digit =
        matches!(bytes.get(end - 1), Some(b) if b.is_ascii_digit() && *b != b'0');
    if last_is_tone_digit {
        return false;
    }
    matches!(bytes.get(end), Some(b) if b.is_ascii_digit() && *b != b'0')
}
