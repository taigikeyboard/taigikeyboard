---
platforms: [linux]
section: Input
---
- **A key typed right after Auto-Space keeps its place.** Picking `hó`, then
  typing `2` and `,` gives `hó 2,`; the comma no longer deletes the digit. The
  comma takes the auto space's place only when that space is still right
  before the cursor, on Fcitx5 and IBus alike. (#869)
