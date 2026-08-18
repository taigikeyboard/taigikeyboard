# macOS candidate window — MacishType port

Plan for replacing the macOS candidate bar (one SwiftUI row in a borderless
`NSPanel`) with a port of MacishType's candidate window, so the panel matches
the native macOS input-method look: the system accent colour, three layouts,
light and dark, and both the Sequoia (vibrancy) and Tahoe (glass) chrome.

Upstream: `references/MacishType/macos/MacishType/` (MIT, © 2026 Luke Chang;
gitignored clone). Ported files carry the upstream attribution in their header.
Codex pre-impl review ran 2026-08-18; its rulings are folded in below.

## Goal

| Axis | What it means here |
|---|---|
| 8 colours | Highlight follows the macOS accent colour picked in System Settings → Appearance (blue / purple / pink / red / orange / yellow / green / graphite). When the system is set to Multicolour, the HOST app's `NSAccentColorName` is used when it resolves — best-effort: apps without a named accent asset, and XPC-hosted fields, fall back to the system accent. No colour setting of our own. |
| 3 layouts | 水平 (horizontal, width-packed pages) · 垂直 (vertical, scrolling) · 展開式 (one row, chevron expands into a width-packed grid). A three-way setting. |
| Light / dark | The panel follows the system appearance and repaints on appearance change. Upstream's per-client appearance needs the undocumented `windowEffectiveAppearance` selector — rejected; system appearance is the supported fallback. |
| Sequoia / Tahoe | Sequoia = `NSVisualEffectView` + 6pt corners + row highlight bar. Tahoe = `NSGlassEffectView` + capsule corners + inset pill highlight. Auto-resolves by OS; an override setting exists, and a forced Tahoe resolves to Sequoia below macOS 26 (`NSGlassEffectView` does not exist there). One private-API carve-out survives, guarded by `responds(to:)` and commented: upstream's `_adaptiveAppearance` KVC on `NSGlassEffectView`, without which small glass surfaces flip their own light/dark. |

## What is ported and what is not

Ported (adapted): base panel chrome (backdrop, corner masks, frame animation,
drag), item view, separator / highlight / chevron / page-arrow views, the three
layout panels, and the accent-colour observer (`ThemeManager` →
`CandidateAccentColor`).

Deliberately NOT carried over (each confirmed by the Codex pass):

| Upstream thing | Why not |
|---|---|
| `Candidate.annotation` + the vertical panel's column-alignment / top-3 width heuristic | Taigi candidates are one already-composed string. Column alignment exists to align the annotation column; without it, padding labels to a shared width changes nothing visible. Widths are measured eagerly over the displayed list instead of lazily corrected. |
| `Candidate.payload` | Absolute index into the controller's retained array is the identity. |
| `-1` no-selection sentinel, "first arrow reveals", empty confirm `("", -1, nil)`, `initialHighlight < 0` | Our IME always selects index 0 on every fresh fetch; the suspended-selection state is unreachable. |
| `indexLabels: String` + `candidateIndex(for:)` / `navigationIntent(...)` | Key classification stays in `ComposingKeyIntent` (the tested seam). Labels become a fixed `["⌃1"…"⌃9"]`; the index column width is the measured widest label, computed once per font configuration — upstream's `indexFontSize + 2` clips two glyphs. |
| Upstream placement (`topLeftPoint`, composition-start/end fallback, its screen lookup) | `CandidatePanelPositioning` + `ScreenLookup` are pure, tested, and clamp oversized panels; replacing them would regress testability. Panels that resize in place (vertical full render, expand) re-derive frames through the same pure function. |
| `windowEffectiveAppearance` client-appearance read | Undocumented selector. System appearance instead. |
| Double-click commit + `candidateConfirmed` delegate | A mouse commit is a new asynchronous entry into the engine and needs owner-token + generation + live-client validation. Deferred to its own later PR; in this track a click selects, and only keys commit. |
| 200-candidate display cap | KEPT (`maxDisplayCandidates = 200`), stated here so the truncation is a decision, not an accident. |

## Architecture change: who owns navigation

Today `CandidateListModel` (controller-side, headless) owns the list, the
highlight and a fixed 9-per-page window; the presenter renders one page of
strings. Width-packed pages, a scrolling layout and an expandable grid make
page geometry a function of measured glyph widths, so navigation moves to
where the measuring is: the panel.

New seam (`CandidatePresenter`):

```
controller                                   panel
  show(labels, anchoredTo:, hostWindowLevel:,
       hostBundleIdentifier:, ownedBy:)  ──▶  rebuilds layout, selects 0
  navigate(direction, ownedBy:)          ──▶  moves / pages / scrolls / expands
  selectedCandidateIndex(ownedBy:)       ──▶  absolute index (Space commit)
  candidateIndex(forSlot:, ownedBy:)     ──▶  slot → absolute (⌃n commit)
  hide(ownedBy:) / hideForHandover()
```

- The controller retains `[ContinuousCandidate]`; `isShowingCandidates` (the
  key contract's input) stays `!fetchedCandidates.isEmpty`, synchronous.
- The panel is authoritative for selection; nothing mirrors it back
  (a `didChangeSelection` callback would be a second source of truth).
- `hide` clears navigation state, not just `orderOut`.
- Every entry is owner-token guarded, as today.
- `ComposingKeyIntent` keeps its table; `.moveHighlight`/`.pageCandidates`
  collapse into `.navigate(CandidateNavigation)` with the six raw directions,
  and each layout interprets them (upstream behaviour). No wrapping anywhere —
  the D4 clamp rule holds.

Test strategy: the key contract keeps its end-to-end coverage through the
recording presenter double; the pure geometry (row packing, grid packing,
slot→index mapping) is extracted into value types with their own tests; what
remains panel-internal (AppKit layout, scroll, animation) is dogfood-gated,
which is also upstream's own test posture.

## Settings

| Key | Values | Default |
|---|---|---|
| `candidateLayout` | `horizontal` · `vertical` · `expandable` | `horizontal` until PR3 lands, then `expandable` (MacishType's default). The setting UI only ever offers implemented values. |
| `candidateWindowStyle` | `auto` · `sequoia` · `tahoe` | `auto`; `tahoe` on macOS < 26 resolves to `sequoia` |

Presentation-only, so they live beside the store's macOS-only keys, not in
`EngineSettings`. Accent colour and font size are not settings.

## Phases

| PR | Scope | Est. |
|---|---|---|
| 1 | Seam move + chrome primitives (backdrop, item view, accent, page arrows) + horizontal paged panel + pure packing types + test rewires. Default layout horizontal; no new setting yet. | ~1500 LOC |
| 2 | Vertical panel + `candidateLayout` setting (two values) + settings UI row + i18n keys | ~700 LOC |
| 3 | Expandable panel + chevron + row highlight + `expandable` value + default flip + `candidateWindowStyle` override + i18n | ~1000 LOC |

Follow-up candidates, explicitly unscoped and unscheduled: double-click mouse
commit; per-client appearance if Apple ever documents a supported read.

## Best-practices alignment

- `~/.claude/rules/planning.md` § Grounded in actual code — every citation read
  before writing; Codex ANALYSIS-ONLY pre-impl confirmed the seam direction and
  supplied the not-adopted list above.
- `.claude/rules/cross-platform-alignment.md` §5.1 — candidate navigation stays
  platform-side.
- `.claude/rules/doc-lookup.md` — `NSGlassEffectView`, `NSVisualEffectView`,
  `NSScroller` checked against the Xcode 26 SDK headers; the two upstream
  private-API uses called out instead of silently inherited.
- CLAUDE.md Core Principle #6 — seam moves to fit the layouts rather than
  bending the layouts to a fixed 9-per-page model.

| 主流做法 | 來源 file:line | 本 plan 對應 phase |
|---|---|---|
| Panel owns candidates + selection, host is a delegate | MacishType `CandidateWindow.swift:150-300` | PR1 |
| Width-packed horizontal page | MacishType `MacishHorizontalBasePanel.swift:12-34` | PR1 |
| Accent colour, Sequoia darken + Tahoe luminance clamp | MacishType `MacishBasePanel.swift:129-177` | PR1 |
| Vertical lazy render + scroll-anchored numbering | MacishType `MacishVerticalPanel.swift:255-330` | PR2 |
| Chevron expand into a column grid | MacishType `MacishHorizontalExpandablePanel.swift:99-220` | PR3 |
| Highlight clamps at both ends | McBopomofo `HorizontalCandidateController.swift:509` | PR1 |
