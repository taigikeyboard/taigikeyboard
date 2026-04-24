"""Numeric-tone stage — add `tl_num` and `poj_num` from diacritic forms."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.romanization import to_numeric_tone

# POJ non-ASCII characters that must be folded to ASCII in poj_num for trie
# lookup to match ASCII user input. Mirrors kesi `tsuan_ascii`. Regression
# guard: pipeline refuses to produce a poj_num column containing these.
_POJ_NON_ASCII_CHARS = "͘ⁿᴺ"  # o-dot, ⁿ, ᴺ


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["tl_num"] = df["tl"].apply(lambda x: to_numeric_tone(str(x)))
    df["poj_num"] = df["poj"].apply(lambda x: to_numeric_tone(str(x), ascii_only=True))

    pattern = f"[{_POJ_NON_ASCII_CHARS}]"
    bad = df[df["poj_num"].str.contains(pattern, regex=True, na=False)]
    if not bad.empty:
        samples = bad.head(5).apply(
            lambda r: f"{r['poj']!r} → {r['poj_num']!r}", axis=1
        ).tolist()
        raise AssertionError(
            f"{ctx.source_name}: {len(bad)} poj_num rows contain POJ non-ASCII "
            f"(o͘ / ⁿ / ᴺ) after ascii folding; samples: {samples}"
        )

    ctx.set_df(df)
