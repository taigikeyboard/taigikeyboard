//! `PrefixIndex` — fst::Set wrapper backed by mmap'd `dictionary.fst`.
//!
//! Wire format (mirrors `dictionary/build/create_fst.py` / fst-builder):
//!     key_bytes (UTF-8) || 0xFF || rowid_le_4
//!
//! The separator sits at the fixed offset `len - 5`; the rowid bytes may
//! themselves be `0xFF`, so a key is never located by searching for it.
//!
//! `lookup_prefix` does a byte-range scan on `[prefix+0xFF, prefix+0x100)`
//! and decodes the trailing 4-byte rowid per hit. Insertion order is
//! preserved through fst's deterministic byte-sorted iteration. Replaces
//! both platforms' MARISA-trie + native-bridge stack.

use fst::{Automaton, IntoStreamer, Set, Streamer};
use mmap_host::MmapHandle;

use crate::error::LexiconError;

// Separator byte between key and rowid; 0xFF is chosen because it is larger than any valid
// UTF-8 byte, guaranteeing correct scan boundaries.
const SEPARATOR: u8 = 0xFF;

/// Bytes a wire entry carries after its key: the separator + the rowid.
const WIRE_SUFFIX_LEN: usize = 1 + std::mem::size_of::<u32>();

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
    /// `skip` is evaluated per distinct `matched_key` (e.g. to drop TPS
    /// acronym / abbrev key surfaces) so excluded keys never consume a
    /// bucket slot; see [`Self::collect_shortest_first`] for the walk.
    /// `lookup_prefix` is unchanged so normal `search` keeps its byte-order
    /// acronym matching.
    pub fn lookup_prefix_shortest_first(
        &self,
        prefix: &str,
        cap: usize,
        skip: impl FnMut(&str) -> bool,
    ) -> Vec<u32> {
        if prefix.is_empty() || cap == 0 {
            return Vec::new();
        }
        // Bucket survivors by matched-key byte length; the `BTreeMap`
        // yields the buckets in ascending length order, so draining it
        // into the cap is shortest-first with no extra sort. Within a
        // length, scan (byte) order is preserved as the stable tiebreak.
        let mut buckets: std::collections::BTreeMap<usize, Vec<u32>> =
            std::collections::BTreeMap::new();
        self.collect_shortest_first(
            prefix.as_bytes(),
            prefix.len(),
            cap,
            fst::automaton::AlwaysMatch,
            skip,
            |matched_key, rowid| buckets.entry(matched_key.len()).or_default().push(rowid),
        );
        buckets.into_values().flatten().take(cap).collect()
    }

    /// Shared walk behind the two shortest-first lookups: streams the wire
    /// entries under `range_prefix` that `filter` accepts and hands every
    /// surviving `(matched_key, rowid)` to `on_hit`.
    ///
    /// Byte order ≠ length order, so a short key can sit anywhere in the
    /// range — but the FST is a trie, and an entry's byte length is its
    /// depth. The range is therefore walked in **widening entry-length
    /// bands**: each band admits [`BAND_CEILING_EXTRA_BYTES`] more key
    /// bytes past `min_key_len`, the last band is unbounded, and the walk
    /// stops after the first band that brings the survivor total to `cap`.
    /// Inside a band the automaton prunes every subtree deeper than the
    /// ceiling and, once the separator fixes an entry's length, every rowid
    /// subtree of a key an earlier band already emitted ([`EntryLenBand`]).
    /// A single-initial prefix fills a 500-rowid cap from the first band,
    /// so the walk never descends into the multi-syllable bulk of the range.
    ///
    /// Equivalence with a full scan: bands ascend, so every entry the walk
    /// never visits is longer than every entry it emitted, and a band is
    /// always drained completely — so once the emitted survivors reach
    /// `cap`, the caller's own length ordering over them (bucket / sort)
    /// selects exactly the `cap` shortest survivors of the whole range.
    /// Byte order inside a band is NOT length order (`tl:taa` streams
    /// before `tl:ta`), which is why the stop check sits after the band,
    /// never inside it.
    ///
    /// `skip` must be a pure predicate of the key: it is evaluated once per
    /// **distinct** key, not per entry — the rowids of one key are
    /// consecutive in the stream, and the TL / POJ acronym predicate
    /// backtracks through syllable splits, so calling it per rowid
    /// dominated the old scan. A skipped key's remaining rowids are not
    /// streamed either: the walk re-seeks to the key's lex sibling (every
    /// extension of the key byte-sorts BEFORE `key || 0xFF`, so nothing
    /// under the key is left behind) — acronym keys such as `tl:ts` carry
    /// thousands of rowids each and were ~80 % of the entries a
    /// single-initial band streamed. A non-UTF-8 key (never produced by
    /// the build pipeline) is dropped the same way.
    fn collect_shortest_first<A: Automaton>(
        &self,
        range_prefix: &[u8],
        min_key_len: usize,
        cap: usize,
        filter: A,
        mut skip: impl FnMut(&str) -> bool,
        mut on_hit: impl FnMut(&str, u32),
    ) {
        let hi = next_lex_sibling(range_prefix);
        let min_entry_len = min_key_len + WIRE_SUFFIX_LEN;
        let mut survivors = 0usize;
        let mut last_key = String::new();
        let mut floor = min_entry_len - 1;
        for ceiling in BAND_CEILING_EXTRA_BYTES
            .iter()
            .map(|extra| min_entry_len + extra)
            .chain([usize::MAX])
        {
            let band = EntryLenBand {
                min_exclusive: floor,
                max_inclusive: ceiling,
            };
            floor = ceiling;
            let mut resume_at: Vec<u8> = range_prefix.to_vec();
            'band: loop {
                let mut builder = self.set.search((&filter).intersection(band)).ge(&resume_at);
                if let Some(hi) = &hi {
                    builder = builder.lt(hi);
                }
                let mut stream = builder.into_stream();
                while let Some(entry) = stream.next() {
                    let Some((key_bytes, rowid)) = split_wire_entry(entry) else {
                        continue;
                    };
                    if key_bytes != last_key.as_bytes() {
                        match std::str::from_utf8(key_bytes) {
                            Ok(key) if !skip(key) => {
                                last_key.clear();
                                last_key.push_str(key);
                            }
                            _ => {
                                // The key starts with the non-empty UTF-8
                                // `range_prefix`, so it is never all 0xFF
                                // and the sibling always exists.
                                let Some(after_key) = next_lex_sibling(key_bytes) else {
                                    continue;
                                };
                                resume_at = after_key;
                                continue 'band;
                            }
                        }
                    }
                    on_hit(&last_key, rowid);
                    survivors += 1;
                }
                break;
            }
            if survivors >= cap {
                return;
            }
        }
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
    pub fn lookup_exact_tps_readings(
        &self,
        key: &str,
        final_only_offsets: &[usize],
    ) -> Vec<(String, u32, u32)> {
        // Unambiguous key (no family glyph): the pattern could only match
        // the literal — use the narrow-range exact lookup, zero automaton.
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
        let family = family_prefix(key);
        let mut builder = self.set.search(&pattern);
        if let Some(hi) = next_lex_sibling(family.as_bytes()) {
            builder = builder.ge(family.as_bytes()).lt(&hi);
        }
        let mut stream = builder.into_stream();
        let mut out: Vec<(String, u32, u32)> = Vec::new();
        while let Some(entry) = stream.next() {
            let Some((key_bytes, rowid)) = split_wire_entry(entry) else {
                continue;
            };
            let Ok(matched_key) = std::str::from_utf8(key_bytes) else {
                continue;
            };
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
    pub fn lookup_prefix_shortest_first_tps_readings(
        &self,
        prefix_key: &str,
        cap: usize,
        skip: impl FnMut(&str) -> bool,
    ) -> Vec<(String, u32)> {
        if prefix_key.is_empty() || cap == 0 {
            return Vec::new();
        }
        // No unambiguous fast path here: the matched-key contract requires
        // the STORED key per hit (record guards + abbrev-face checks run on
        // it), and the pattern walk over an unambiguous prefix is already
        // pruned to the literal branch by `can_match` — same traversal cost
        // as the narrow range (Codex confirm 2026-08-19 finding 2).
        let pattern = crate::tps_pattern::TpsKeyPattern::new(
            prefix_key,
            crate::tps_pattern::WireMode::StartsWith,
            &[],
        );
        // Walk the whole family range, NOT the literal key's own narrow
        // range: an alternate glyph may byte-sort far from the literal.
        let family = family_prefix(prefix_key);
        // (matched_key_len, subst_on_typed_prefix) survivors; the matched
        // key travels with the rowid so record guards validate against
        // what the pattern actually hit (Codex post-impl BLOCK 1).
        let mut survivors: Vec<(usize, u32, u32, String)> = Vec::new();
        self.collect_shortest_first(
            family.as_bytes(),
            prefix_key.len(),
            cap,
            &pattern,
            skip,
            |matched_key, rowid| {
                // Substitutions can only occur inside the typed prefix; the
                // charwise zip stops at the shorter side, so the shared
                // helper applies as-is.
                let subst = crate::tps_pattern::substitution_count(prefix_key, matched_key);
                survivors.push((matched_key.len(), subst, rowid, matched_key.to_string()));
            },
        );
        // Stable sort: byte order within equal (len, subst) is preserved.
        survivors.sort_by_key(|&(len, subst, _, _)| (len, subst));
        survivors
            .into_iter()
            .take(cap)
            .map(|(_, _, rowid, matched_key)| (matched_key, rowid))
            .collect()
    }

    fn scan_from(&self, lo: &[u8]) -> Vec<u32> {
        let mut stream = self.set.range().ge(lo).into_stream();
        decode_rowids(&mut stream, lo.len())
    }

    /// Exact-match lookup — returns rowids whose key equals `key` exactly.
    /// Filters the prefix-scan output by entry-length parity.
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
            if entry.len() != key_bytes.len() + WIRE_SUFFIX_LEN {
                continue;
            }
            if let Some((_, rowid)) = split_wire_entry(entry) {
                out.push(rowid);
            }
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

/// The `<family>:` prefix of a namespaced key (`tps:ㄍㄚ` → `tps:`) —
/// the range bound that keeps a family-wide automaton walk from touching
/// the `tl:` / `poj:` / `hanzi:` regions.
fn family_prefix(key: &str) -> String {
    key.split(':')
        .next()
        .map(|family| format!("{family}:"))
        .unwrap_or_default()
}

/// Split one wire entry into `(key_bytes, rowid)`; `None` when the entry
/// is too short to carry the separator + rowid suffix.
fn split_wire_entry(entry: &[u8]) -> Option<(&[u8], u32)> {
    let key_end = entry.len().checked_sub(WIRE_SUFFIX_LEN)?;
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&entry[key_end + 1..]);
    Some((&entry[..key_end], u32::from_le_bytes(buf)))
}

/// Key bytes past the typed prefix that each widening band of
/// [`PrefixIndex::collect_shortest_first`] admits (ceilings, not
/// increments) before the walk falls back to the unbounded range. A perf
/// heuristic only — the result is the same for any ascending ceilings.
/// Two bytes = two roman letters, enough for every 500-rowid production
/// roman prefix to fill its cap in one walk; a wider first band admits
/// the 3-letter acronym keys too and triples the `skip` calls. A
/// Bopomofo glyph is three bytes, so a TPS walk needs the +4 band — the
/// empty +2 pass costs microseconds. Later ceilings double so a sparse
/// prefix reaches the unbounded walk in a handful of cheap passes.
const BAND_CEILING_EXTRA_BYTES: [usize; 5] = [2, 4, 8, 16, 32];

/// `fst::Automaton` accepting exactly the wire entries whose byte length
/// lies in `(min_exclusive, max_inclusive]`, with subtree pruning on both
/// sides: nothing deeper than `max_inclusive` is descended, and once the
/// `0xFF` separator fixes an entry's length at `sep_depth + 5`, a length
/// already covered by an earlier band prunes that key's whole rowid
/// subtree. Keys are UTF-8, so the first `0xFF` on a path is always the
/// separator; later `0xFF` bytes belong to the rowid and are ignored.
#[derive(Clone, Copy)]
struct EntryLenBand {
    min_exclusive: usize,
    max_inclusive: usize,
}

/// `(depth, separator depth once seen)`.
type EntryLenState = (usize, Option<usize>);

impl EntryLenBand {
    fn admits(&self, entry_len: usize) -> bool {
        entry_len > self.min_exclusive && entry_len <= self.max_inclusive
    }
}

impl Automaton for EntryLenBand {
    type State = EntryLenState;

    fn start(&self) -> EntryLenState {
        (0, None)
    }

    fn is_match(&self, &(depth, sep): &EntryLenState) -> bool {
        sep.is_some_and(|sep_depth| depth == sep_depth + WIRE_SUFFIX_LEN && self.admits(depth))
    }

    fn can_match(&self, &(depth, sep): &EntryLenState) -> bool {
        match sep {
            Some(sep_depth) => {
                depth <= sep_depth + WIRE_SUFFIX_LEN && self.admits(sep_depth + WIRE_SUFFIX_LEN)
            }
            // The shortest completion of a key at this depth is the key
            // itself plus separator + rowid.
            None => depth + WIRE_SUFFIX_LEN <= self.max_inclusive,
        }
    }

    fn accept(&self, &(depth, sep): &EntryLenState, byte: u8) -> EntryLenState {
        let sep = sep.or((byte == SEPARATOR).then_some(depth));
        (depth + 1, sep)
    }
}

fn decode_rowids(
    stream: &mut fst::set::Stream<'_, fst::automaton::AlwaysMatch>,
    min_key_len: usize,
) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    while let Some(entry) = stream.next() {
        if entry.len() < min_key_len + WIRE_SUFFIX_LEN {
            continue;
        }
        if let Some((_, rowid)) = split_wire_entry(entry) {
            out.push(rowid);
        }
    }
    out
}
