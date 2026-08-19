//! `PrefixIndex` — fst::Set wrapper backed by mmap'd `dictionary.fst`.
//!
//! Wire format (mirrors `dictionary/build/create_fst.py` / fst-builder):
//!     key_bytes (UTF-8) || 0xFF || rowid_le_4
//!
//! `lookup_prefix` does a byte-range scan on `[prefix+0xFF, prefix+0x100)`
//! and decodes the trailing 4-byte rowid per hit. Insertion order is
//! preserved through fst's deterministic byte-sorted iteration. Replaces
//! both platforms' MARISA-trie + native-bridge stack.

// 中文: PrefixIndex — 包裝 mmap 過的 dictionary.fst 提供前綴查詢。
// 中文: 條目格式為 key_bytes || 0xFF || rowid_le_4;以 byte-range 掃描搭配 0xFF 分隔符即可決定前綴邊界。

use fst::{IntoStreamer, Set, Streamer};
use mmap_host::MmapHandle;

use crate::error::LexiconError;

// 中文: key 與 rowid 之間的分隔位元組;選用 0xFF 是因為它大於任何合法 UTF-8 byte,可保證掃描邊界正確。
const SEPARATOR: u8 = 0xFF;

// 中文: 對外的前綴索引 — 持有 fst::Set 與條目數。
pub struct PrefixIndex {
    set: Set<MmappedSetData>,
    entry_count: u64,
}

/// Wrapper allowing `fst::Set` to take ownership of the mmap handle while
/// exposing `AsRef<[u8]>`.
struct MmappedSetData {
    handle: MmapHandle,
}

impl AsRef<[u8]> for MmappedSetData {
    fn as_ref(&self) -> &[u8] {
        self.handle.as_slice()
    }
}

impl PrefixIndex {
    /// Open `dictionary.fst` mmap'd readonly. Validates that fst can parse
    /// the bytes; deeper format checks happen on first use.
    // 中文: 以唯讀 mmap 開啟 dictionary.fst,並驗證 fst crate 能解析。
    pub fn open(path: &std::path::Path) -> Result<Self, LexiconError> {
        let handle = MmapHandle::open_readonly(path).map_err(|source| LexiconError::Mmap {
            path: path.display().to_string(),
            source,
        })?;
        let data = MmappedSetData { handle };
        let set = Set::new(data).map_err(|e| {
            LexiconError::InvalidBinary(format!("fst load `{}`: {}", path.display(), e))
        })?;
        let entry_count = set.len() as u64;
        Ok(Self { set, entry_count })
    }

    // 中文: 回傳前綴索引的條目總數。
    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    /// Return rowids whose stored key has the given prefix. Insertion order
    /// preserved (deterministic per-build, matches Android `distinct()`
    /// behavior — D-12 parity correction).
    ///
    /// Range: half-open `[prefix, prefix_succ)` where `prefix_succ` is the
    /// next sibling prefix in lex order (last byte +1). All wire entries
    /// of the form `key + 0xFF + rowid_le_4` whose `key` starts with
    /// `prefix` fall in this range.
    // 中文: 前綴查詢 — 回傳所有 key 以 prefix 開頭的 rowid,維持 fst byte-sort 的插入順序。
    pub fn lookup_prefix(&self, prefix: &str) -> Vec<u32> {
        let prefix_bytes = prefix.as_bytes();
        if prefix_bytes.is_empty() {
            return Vec::new();
        }
        let lo: Vec<u8> = prefix_bytes.to_vec();
        let Some(hi) = next_lex_sibling(prefix_bytes) else {
            // prefix is all 0xFF — no successor; fall back to a half-open
            // upper bound that includes everything (effectively scans
            // tail). Edge case for keys built from non-UTF-8 bytes; in
            // practice never reached for `tl:` / `poj:` / `hanzi:` prefixes.
            return self.scan_from(&lo);
        };

        let mut stream = self.set.range().ge(&lo).lt(&hi).into_stream();
        decode_rowids(&mut stream, prefix_bytes.len())
    }

    /// Prefix lookup whose hydration budget (`cap`) is spent on the
    /// **shortest matched keys first**, with optional per-key exclusion via
    /// `skip(matched_key)`. Returns at most `cap` rowids.
    ///
    /// Motivation: the wire format is `key || 0xFF || rowid_le_4`, and the
    /// `0xFF` separator is greater than any UTF-8 byte, so a short exact key
    /// (`tps:ㄍㄚ`) byte-sorts AFTER every longer extension of it
    /// (`tps:ㄍㄚㄅㄧ`…). A plain byte-ordered `lookup_prefix(..).take(cap)`
    /// therefore front-loads the deepest, longest (and usually rarest)
    /// entries and buries the shortest readings — for a single-initial
    /// continuous query (`ㄍ`) that means the high-frequency single-syllable
    /// candidates never reach the downstream ranker. Bucketing the
    /// survivors by matched-key byte length and filling `cap` shortest-first
    /// fixes the budget bias.
    ///
    /// This is a **hydration budget policy only** — the visible candidate
    /// order is still decided by the caller's `SortKey` (recency / score /
    /// frequency). Length just decides which rowids enter the pool.
    ///
    /// `skip` is evaluated on each `matched_key` during the scan (e.g. to
    /// drop TPS acronym / abbrev key surfaces) so excluded keys never
    /// consume a bucket slot. `matched_key` is the UTF-8 key reconstructed
    /// from the wire entry by dropping the trailing `0xFF || rowid_le_4`;
    /// the separator sits at the fixed offset `entry.len() - 5` (the rowid
    /// little-endian bytes may contain `0xFF`, so it is located by offset,
    /// never by searching for `0xFF`). `lookup_prefix` is unchanged so
    /// normal `search` keeps its byte-order acronym matching.
    // 中文: 前綴查詢,但 hydration 預算 (cap) 優先給「最短 matched key」,並可用
    // 中文:   skip(matched_key) 逐 key 排除。回傳至多 cap 個 rowid。
    // 中文: 動機:wire = key||0xFF||rowid,0xFF 大於任何 UTF-8 byte,故短 exact key
    // 中文:   (tps:ㄍㄚ) byte 序排在其所有長延伸 (tps:ㄍㄚㄅㄧ…) 之後。直接
    // 中文:   lookup_prefix(..).take(cap) 會 front-load 最長最冷僻的詞、埋掉短讀音 →
    // 中文:   裸聲母 (ㄍ) 連續查詢時高頻單音節候選進不了 ranker。依 matched key 長度
    // 中文:   分桶、短鍵優先填 cap,即修正此預算偏差。
    // 中文: 此為「hydration 預算政策」,非最終排序 — 畫面順序仍由 caller 的 SortKey 決定。
    // 中文:   matched_key 由 wire entry 去尾端 0xFF||rowid_le_4 還原;separator 在固定偏移
    // 中文:   entry.len()-5 (rowid bytes 可能含 0xFF,以偏移定位,絕不搜尋 0xFF)。
    pub fn lookup_prefix_shortest_first(
        &self,
        prefix: &str,
        cap: usize,
        mut skip: impl FnMut(&str) -> bool,
    ) -> Vec<u32> {
        let prefix_bytes = prefix.as_bytes();
        if prefix_bytes.is_empty() || cap == 0 {
            return Vec::new();
        }
        let lo: Vec<u8> = prefix_bytes.to_vec();
        let Some(hi) = next_lex_sibling(prefix_bytes) else {
            // prefix is all 0xFF — no successor (mirrors `lookup_prefix`;
            // never reached for `tps:` / `tl:` / `poj:` prefixes). Bucketing
            // adds no value on this edge; fall back to a plain filtered scan.
            let mut stream = self.set.range().ge(&lo).into_stream();
            let mut out = decode_rowids_filtered(&mut stream, lo.len(), &mut skip);
            out.truncate(cap);
            return out;
        };
        let mut stream = self.set.range().ge(&lo).lt(&hi).into_stream();
        let min_key_len = prefix_bytes.len();
        // Bucket survivors by matched-key byte length. The full range is
        // scanned (byte order ≠ length order, so a short key can appear
        // anywhere); the `BTreeMap` then yields the buckets in ascending
        // length order, so draining it into the cap is shortest-first with
        // no extra sort. Same total rowid memory as `lookup_prefix`, one
        // extra O(n) bucketing pass. Within a length, scan (byte) order is
        // preserved as the stable tiebreak.
        let mut buckets: std::collections::BTreeMap<usize, Vec<u32>> =
            std::collections::BTreeMap::new();
        use fst::Streamer;
        while let Some(entry) = stream.next() {
            if let Some((key_len, rowid)) = decode_entry_filtered(entry, min_key_len, &mut skip) {
                buckets.entry(key_len).or_default().push(rowid);
            }
        }
        let mut out: Vec<u32> = Vec::with_capacity(cap);
        for (_len, bucket) in buckets {
            if out.len() >= cap {
                break;
            }
            let remaining = cap - out.len();
            out.extend(bucket.into_iter().take(remaining));
        }
        out
    }

    /// TPS ambiguity-aware exact lookup — one automaton walk returning
    /// every stored key that is a READING of `key` under the TPS
    /// ambiguity families (`INVARIANT_TPS_DEFOLD_ENUMERATE` §35), plus
    /// the rowids under each. Results are ordered substitution-count
    /// ascending (the user's literal text first), byte order within a
    /// count — the ordering contract the continuous fetch relies on for
    /// natural-reading-first dedupe.
    ///
    /// `final_only_offsets` = byte offsets (into `key`) of glyphs
    /// immediately before a stripped separator / 連字 barrier; those
    /// slots keep only Final-role readings (§31 — the user's explicit
    /// boundary must not be re-read as a syllable onset). Tone-mark
    /// restriction is derived inside the pattern builder.
    ///
    /// Scan range: the whole `tps:` sibling range of the literal key's
    /// family prefix — NOT the literal key's own narrow range, because an
    /// alternate glyph may byte-sort far from the literal (Codex
    /// pre-impl Q5: a literal-key range would exclude it).
    // 中文: TPS 歧義感知 exact 查詢 — 單次 automaton 走訪回傳 key 的所有讀法與其 rowids;
    // 中文:   排序 = 替換數升冪(使用者字面優先)、同數依 byte 序。final_only_offsets = barrier
    // 中文:   前一格的 byte 偏移(只許 Final 形);調號限制由 pattern builder 內部推導。
    // 中文: 掃描範圍 = 整個 tps: 家族 range,不能用字面 key 的窄 range(替代 glyph byte 序可能落在外)。
    pub fn lookup_exact_tps_readings(
        &self,
        key: &str,
        final_only_offsets: &[usize],
    ) -> Vec<(String, u32, u32)> {
        use fst::{IntoStreamer, Streamer};
        // Unambiguous key (no family glyph): the pattern could only match
        // the literal — use the narrow-range exact lookup, zero automaton.
        // 中文: 無歧義 glyph 的 key 只可能命中字面 → 走窄 range exact,免自動機。
        if !crate::tps_pattern::has_ambiguous_glyph(key) {
            return self
                .lookup_exact(key)
                .into_iter()
                .map(|rowid| (key.to_string(), rowid, 0))
                .collect();
        }
        let pattern = crate::tps_pattern::TpsKeyPattern::new(
            key,
            crate::tps_pattern::WireMode::ExactWire,
            final_only_offsets,
        );
        // Constrain the automaton scan to the family prefix's range so it
        // never touches `tl:` / `poj:` / `hanzi:` regions.
        let prefix = key
            .split(':')
            .next()
            .map(|p| format!("{p}:"))
            .unwrap_or_default();
        let mut builder = self.set.search(&pattern);
        if let Some(hi) = next_lex_sibling(prefix.as_bytes()) {
            builder = builder.ge(prefix.as_bytes()).lt(&hi);
        }
        let mut stream = builder.into_stream();
        let mut out: Vec<(String, u32, u32)> = Vec::new();
        while let Some(entry) = stream.next() {
            if entry.len() < 5 {
                continue;
            }
            // Wire = key || 0xFF || rowid_le_4: the separator sits at the
            // fixed offset len-5 (rowid bytes may themselves be 0xFF).
            let key_end = entry.len() - 5;
            let Ok(matched_key) = std::str::from_utf8(&entry[..key_end]) else {
                continue;
            };
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&entry[entry.len() - 4..]);
            let rowid = u32::from_le_bytes(buf);
            let subst = crate::tps_pattern::substitution_count(key, matched_key);
            out.push((matched_key.to_string(), rowid, subst));
        }
        // Substitution-count ascending; the automaton stream is already in
        // byte order, and the sort is stable, so ties keep byte order.
        out.sort_by_key(|(_, _, subst)| *subst);
        out
    }

    /// TPS ambiguity-aware variant of [`Self::lookup_prefix_shortest_first`]
    /// — the partial-prefix hydration for an incomplete TPS tail also
    /// considers every reading of the typed prefix (`ㄇ` surfaces both
    /// `ㄇ…` and `ㆬ…` words). Budget policy extends the existing
    /// contract: matched-key length ascending, then substitution count
    /// ascending, then byte order.
    // 中文: lookup_prefix_shortest_first 的 TPS 歧義感知版 — 部分前綴也考慮所有讀法
    // 中文:   (ㄇ 同時撈 ㄇ… 與 ㆬ… 詞)。預算排序:matched key 長度升冪 → 替換數升冪 → byte 序。
    pub fn lookup_prefix_shortest_first_tps_readings(
        &self,
        prefix_key: &str,
        cap: usize,
        mut skip: impl FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        use fst::{IntoStreamer, Streamer};
        if prefix_key.is_empty() || cap == 0 {
            return Vec::new();
        }
        // No unambiguous fast path here: the matched-key contract requires
        // the STORED key per hit (record guards + abbrev-face checks run on
        // it), and the pattern walk over an unambiguous prefix is already
        // pruned to the literal branch by `can_match` — same traversal cost
        // as the narrow range (Codex confirm 2026-08-19 finding 2).
        // 中文: 不設無歧義捷徑 — matched-key 契約需要每筆命中的「儲存 key」;
        // 中文:   無歧義 pattern 經 can_match 剪枝後本就只走字面分支,成本等同窄 range。
        let pattern = crate::tps_pattern::TpsKeyPattern::new(
            prefix_key,
            crate::tps_pattern::WireMode::StartsWith,
            &[],
        );
        let family = prefix_key
            .split(':')
            .next()
            .map(|p| format!("{p}:"))
            .unwrap_or_default();
        let mut builder = self.set.search(&pattern);
        if let Some(hi) = next_lex_sibling(family.as_bytes()) {
            builder = builder.ge(family.as_bytes()).lt(&hi);
        }
        let mut stream = builder.into_stream();
        // (matched_key_len, subst_on_typed_prefix, byte-order index) buckets;
        // the matched key travels with the rowid so record guards validate
        // against what the pattern actually hit (Codex post-impl BLOCK 1).
        let mut survivors: Vec<(usize, u32, usize, u32, String)> = Vec::new();
        let mut order = 0usize;
        while let Some(entry) = stream.next() {
            if entry.len() < 5 {
                continue;
            }
            let key_end = entry.len() - 5;
            let Ok(matched_key) = std::str::from_utf8(&entry[..key_end]) else {
                continue;
            };
            if skip(matched_key) {
                continue;
            }
            // Substitutions can only occur inside the typed prefix; the
            // charwise zip stops at the shorter side, so the shared helper
            // applies as-is.
            let subst = crate::tps_pattern::substitution_count(prefix_key, matched_key);
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&entry[entry.len() - 4..]);
            survivors.push((
                key_end,
                subst,
                order,
                u32::from_le_bytes(buf),
                matched_key.to_string(),
            ));
            order += 1;
        }
        survivors.sort_by_key(|&(len, subst, ord, _, _)| (len, subst, ord));
        survivors
            .into_iter()
            .take(cap)
            .map(|(_, _, _, rowid, matched_key)| (matched_key, rowid))
            .collect()
    }

    fn scan_from(&self, lo: &[u8]) -> Vec<u32> {
        let mut stream = self.set.range().ge(lo).into_stream();
        decode_rowids(&mut stream, lo.len())
    }

    /// Exact-match lookup — returns rowids whose key equals `key` exactly.
    /// Filters the prefix-scan output by entry-length parity.
    // 中文: 完全比對查詢 — 僅回傳 key 完全相等的 rowid;以條目長度等於 key+1+4 過濾掉同前綴的較長條目。
    pub fn lookup_exact(&self, key: &str) -> Vec<u32> {
        let key_bytes = key.as_bytes();
        if key_bytes.is_empty() {
            return Vec::new();
        }
        // Range `[key + 0xFF, next_lex_sibling(key)]`. Because the
        // separator is 0xFF, the upper bound is the next sibling of the
        // raw `key` (incrementing key's last byte) — that strictly
        // excludes longer prefix-matching entries.
        let mut lo = Vec::with_capacity(key_bytes.len() + 1);
        lo.extend_from_slice(key_bytes);
        lo.push(SEPARATOR);
        let Some(hi) = next_lex_sibling(key_bytes) else {
            return Vec::new();
        };
        let mut stream = self.set.range().ge(&lo).lt(&hi).into_stream();
        let mut out: Vec<u32> = Vec::new();
        while let Some(entry) = stream.next() {
            if entry.len() != key_bytes.len() + 1 + 4 {
                continue;
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&entry[entry.len() - 4..]);
            out.push(u32::from_le_bytes(buf));
        }
        out
    }
}

/// Returns the next lex sibling prefix — i.e. the smallest byte sequence
/// strictly greater than `prefix` such that no string starting with
/// `prefix` is `>= sibling`. Computed by incrementing the last non-0xFF
/// byte and truncating; returns `None` when prefix is all 0xFF (no
/// successor in u8 space).
fn next_lex_sibling(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut out = prefix.to_vec();
    while let Some(last) = out.last_mut() {
        if *last < 0xFF {
            *last += 1;
            return Some(out);
        }
        out.pop();
    }
    None
}

fn decode_rowids(
    stream: &mut fst::set::Stream<'_, fst::automaton::AlwaysMatch>,
    min_key_len: usize,
) -> Vec<u32> {
    use fst::Streamer;
    let mut out: Vec<u32> = Vec::new();
    while let Some(entry) = stream.next() {
        if entry.len() < min_key_len + 1 + 4 {
            continue;
        }
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&entry[entry.len() - 4..]);
        out.push(u32::from_le_bytes(buf));
    }
    out
}

/// Parse one wire entry `key_bytes || 0xFF || rowid_le_4`, applying
/// `skip(matched_key)`. Returns `(matched_key_len, rowid)` for a surviving
/// entry, or `None` when the entry is too short or skipped. The separator
/// is at the fixed offset `entry.len() - 5`; the rowid little-endian bytes
/// may contain `0xFF`, so it is located by offset, never by searching for
/// `0xFF`. A non-UTF-8 key (never produced by the build pipeline) is kept
/// rather than dropped — the skip predicate is a filter, not a validator.
// 中文: 解析單一 wire entry (key||0xFF||rowid),套 skip(matched_key)。存活回
// 中文:   (matched_key_len, rowid),太短或被 skip 回 None。separator 在固定偏移
// 中文:   entry.len()-5 (rowid bytes 可能含 0xFF,以偏移定位)。非 UTF-8 key (build
// 中文:   pipeline 不會產生) 保留而非丟棄 — skip 是過濾器,不是驗證器。
fn decode_entry_filtered(
    entry: &[u8],
    min_key_len: usize,
    skip: &mut impl FnMut(&str) -> bool,
) -> Option<(usize, u32)> {
    if entry.len() < min_key_len + 1 + 4 {
        return None;
    }
    let key_bytes = &entry[..entry.len() - 5];
    if let Ok(key) = std::str::from_utf8(key_bytes) {
        if skip(key) {
            return None;
        }
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&entry[entry.len() - 4..]);
    Some((key_bytes.len(), u32::from_le_bytes(buf)))
}

/// [`decode_rowids`] variant that drops an entry when `skip(matched_key)`
/// is `true`. Thin stream wrapper over [`decode_entry_filtered`].
// 中文: decode_rowids 變體;skip 為真丟棄該 entry。為 decode_entry_filtered 的 stream 薄包裝。
fn decode_rowids_filtered(
    stream: &mut fst::set::Stream<'_, fst::automaton::AlwaysMatch>,
    min_key_len: usize,
    skip: &mut impl FnMut(&str) -> bool,
) -> Vec<u32> {
    use fst::Streamer;
    let mut out: Vec<u32> = Vec::new();
    while let Some(entry) = stream.next() {
        if let Some((_, rowid)) = decode_entry_filtered(entry, min_key_len, skip) {
            out.push(rowid);
        }
    }
    out
}
