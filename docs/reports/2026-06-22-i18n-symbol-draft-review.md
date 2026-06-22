# i18n Symbol-namespace TL/POJ draft — review sheet (2026-06-22)

**Status: REVIEW-PENDING.** Round S authored the `symbol` namespace (5 keyboard-overlay category labels). `tailo` is a Claude DRAFT, char-composed from authoritative single-char readings in `dictionary/output/dictionary.csv` — NOT linguistically verified as compounds. `poj` is mechanically derived from `tailo` via `tools/i18n/derive_poj.py` (strict bridge); fix `tailo` then re-derive, never hand-edit `poj`. No release / no production-promotion before USER linguistic sign-off (same gate as P3b/P3c TL+POJ sheets).

## The 5 labels

| key | hanji | tailo (DRAFT) | poj (derived) | en | ja | confidence | evidence |
|---|---|---|---|---|---|---|---|
| fullWidth | 全形 | tsuân-hîng | choân-hêng | Full-width | 全角 | HIGH | 全 `tsuân` + 形 `hîng`, both top-freq single-char readings |
| halfWidth | 半形 | puànn-hîng | pòaⁿ-hêng | Half-width | 半角 | HIGH | 半 `puànn` + 形 `hîng`, top-freq |
| hiragana | 平仮名 | pîng-ké-miâ | pêng-ké-miâ | Hiragana | ひらがな | MED | 平 literary `pîng` + 假名 `ké-miâ` (dict compound `假名,ké-miâ`). 仮=假 shinjitai → same reading. Verify 平 literary-vs-`pênn`. |
| katakana | 片仮名 | phiàn-ké-miâ | phiàn-ké-miâ | Katakana | カタカナ | MED | 片 literary `phiàn` (vs `phìnn`) + 假名 `ké-miâ`. Verify 片 reading in this compound. |
| kaomoji | 顏文字 | gân-bûn-jī | gân-bûn-jī | Kaomoji | 顔文字 | MED | 顏 `gân` + 文 `bûn` + 字 `jī` (正音 j- over l-). NB dict has the *concept* word 表情文字 `piáu-tsîng-bûn-jī`; this draft romanizes the displayed hanji 顏文字 instead (mirrors layout.json convention = TL of the same hanji). USER may prefer the concept word. |

## Notes for the reviewer (Codex pre-impl F1)
- NONE of the 5 compounds (全形/半形/平仮名/片仮名/顏文字) exist as `dictionary.csv` entries. Only the **components** + `假名 ké-miâ` + `表情文字 piáu-tsîng-bûn-jī` do. So these are char-composed drafts, not dictionary-verified compounds.
- `ja` values use native Japanese terms: 全角/半角 (zenkaku/hankaku), ひらがな/カタカナ (kana forms, not kanji 平仮名/片仮名), 顔文字 (JP 顔, not 顏).
- `en` follows common keyboard-symbol terminology.
- The hanji column is unchanged from the pre-i18n hardcoded labels (`SymbolData`), so the漢字 display is byte-identical to today.

## ⚠ Known visual risk — USER dogfood decision (Codex pre+post-impl MEDIUM)
The symbol overlay has a **fixed 5-equal-width** tab row (iOS HStack, Android M3 PrimaryTabRow). Pre-i18n labels were all 3-char Hanji (全形…) which never wrapped. The new en/TL/POJ labels are longer ("Full-width", "phiàn-ké-miâ" = 12 chars) and on a narrow keyboard (~360dp → ~72dp/tab) may wrap to 2 lines / grow tab height when display language = English / Tâi-lô / POJ / pseudo.

Current code keeps the existing no-`lineLimit` rendering (shows full text, nothing truncated) — deliberately NOT shipping a speculative truncate/shrink/scroll policy, because (a) the visual gate is qualitative dogfood (code-review-rules §9), and (b) the label strings themselves are under review here, so the fix may be "shorten the labels" rather than "change the tab layout".

**Dogfood-acceptance item**: open symbol overlay under each display language (esp. English + Tâi-lô + pseudo) on a small device; if tabs wrap/look bad, USER decides: shorten labels (string review) OR add lineLimit+autoshrink (iOS) / maxLines+ellipsis (Android) / scrollable tab row.

## How to apply corrections
Edit `i18n/symbol.json` `tailo` (and `hanji`/`en`/`ja` if needed), then:
```
make i18n-derive-poj   # re-derive poj from corrected tailo
make i18n              # regenerate artifacts
make i18n-test && make i18n-check && python3 tools/i18n/derive_poj.py --check
```
