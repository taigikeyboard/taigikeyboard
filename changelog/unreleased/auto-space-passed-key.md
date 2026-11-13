---
platforms: [windows, linux]
section: Input
---
- **A key typed right after Auto-Space keeps its place.** Picking `hó`, then
  typing `2` and `,` gives `hó 2,`; the comma no longer deletes the digit. On
  Linux, the comma now takes the auto space's place only when the space is
  still right before the cursor. (#869)
