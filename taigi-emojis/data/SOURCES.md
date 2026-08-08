# Upstream Sources

All files in `data/` are pinned upstream snapshots, committed for reproducible builds.
Re-pull with `make fetch` only when bumping the version pin (then `make build` + review diff).

| File | Origin | Version | License | Used for |
|---|---|---|---|---|
| `emoji-test.txt` | https://unicode.org/Public/emoji/latest/emoji-test.txt (Version: 17.0, Date: 2025-08-04) | Emoji 17.0 | [Unicode](https://www.unicode.org/copyright.html) | Membership, order, groups/subgroups, names, emoji version, skin-tone variation source |
| `cldr-zh_Hant.xml` | cldr `release-48` `common/annotations/zh_Hant.xml` | CLDR 48 | [Unicode](https://www.unicode.org/copyright.html) | 華語 (zh-Hant) keywords |
| `cldr-zh_Hant-derived.xml` | cldr `release-48` `common/annotationsDerived/zh_Hant.xml` | CLDR 48 | [Unicode](https://www.unicode.org/copyright.html) | 華語 keywords (derived sequences) |
| `cldr-en.xml` | cldr `release-48` `common/annotations/en.xml` | CLDR 48 | [Unicode](https://www.unicode.org/copyright.html) | English keywords |
| `cldr-en-derived.xml` | cldr `release-48` `common/annotationsDerived/en.xml` | CLDR 48 | [Unicode](https://www.unicode.org/copyright.html) | English keywords (derived sequences) |

## Version pin

`MAX_EMOJI_VERSION = "E17.0"` in `scripts/generate.py`. iOS 26.4 and current Android render
the Emoji 17.0 set; older OS fonts that lack an E17 glyph drop it at load via each platform's
glyph filter — not by shipping per-version files. Bump the pin (`MAX_EMOJI_VERSION` +
`CLDR_VERSION` in `scripts/generate.py`, `UNICODE_EMOJI_VERSION` + `CLDR_TAG` in the
`Makefile`) together when raising it.

**E17 source note**: Unicode has not yet populated `/Public/emoji/17.0/`; the canonical
machine-readable `emoji-test.txt` for Emoji 17.0 (Version: 17.0, Date: 2025-08-04) is served
from `/Public/emoji/latest/`. The committed `data/emoji-test.txt` snapshot is the
reproducibility anchor. Point `UNICODE_BASE` back at the versioned path once it appears.
