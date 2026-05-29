"""kautian_accent_wordgen stage — generate word-level accent variants (req2).

Runs after `frequency` (so generated rows inherit the base word's frequency —
DD4b equal rank) and before `poj` (so the generated readings flow through the
poj/numtone/notone/abbrev/source column derivation like any other row). No-op
unless the kautian provenance maps are stashed — only the kautian config wires
this stage in. See `common/kautian_accent_wordgen.py` for the algorithm + the
USER-locked decisions (DD1/DD4/DD4b/DD9).
"""

from __future__ import annotations

from common.kautian_accent_wordgen import apply_word_accent_generation
from common.kautian_provenance import PROVENANCE_META_KEY
from common.taigi_bridge import BridgeDeadError, convert_tl_to_poj_strict
from pipeline.context import PipelineContext


def _poj_convertible(reading: str) -> bool:
    """True when `reading` survives strict TL→POJ conversion.

    The downstream `poj` stage aborts the WHOLE kautian build on any
    non-convertible tl, so generated readings are validated here first and
    skipped on failure. A `BridgeDeadError` (Node subprocess died) is
    re-raised — it is not a per-row failure and must fail the build loud,
    never silently drop generated rows.
    """
    try:
        convert_tl_to_poj_strict(reading)
        return True
    except BridgeDeadError:
        raise
    except RuntimeError:
        return False


def run(ctx: PipelineContext) -> None:
    maps = ctx.get_meta(PROVENANCE_META_KEY)
    if not maps:
        ctx.logger.info("  [skip] no kautian provenance maps stashed")
        return

    df, report = apply_word_accent_generation(
        ctx.current_df(),
        accent_map=maps["accent"],
        poj_convertible=_poj_convertible,
    )
    ctx.logger.info(
        "  accent wordgen: scanned_main=%d generating=%d rows_emitted=%d "
        "collisions_merged=%d skipped_nonconvertible=%d ambiguous_skipped=%d "
        "misaligned=%d",
        report.main_rows_scanned,
        report.words_generating,
        report.rows_emitted,
        report.collisions_merged,
        report.skipped_nonconvertible,
        report.ambiguous_accents_skipped,
        report.misaligned_rows,
    )
    ctx.set_df(df)
