---
name: refactor-checklist
description: Post-refactor verification checklist for cross-platform keyboard project. Invoked by Claude after completing refactoring tasks to ensure nothing was missed.
user-invocable: false
---

# Refactor Checklist

After completing a refactoring task, verify each item below.

## Checklist

1. **No behavior change** — Refactored code produces identical output for identical input. No user-visible change.
2. **Cross-platform alignment** — If an iOS file changed, check the Android counterpart (and vice versa) per `docs/file-structure.md`. Flag if the other side needs a matching change.
3. **No orphaned code** — Removed or renamed functions/types are not still referenced elsewhere. Search the codebase to confirm.
4. **Comments updated** — Comments near changed code still describe the current behavior. Flag stale comments.
5. **Tests still relevant** — Test files for changed modules still test the right things. No test references broken symbols.
6. **Docs still accurate** — If the change affects a module documented in `docs/engine/` or `docs/ui/`, note which docs may need updating.

## Output

```markdown
## Refactor Checklist

| # | Check                   | Status | Notes |
|---|-------------------------|--------|-------|
| 1 | No behavior change      | OK/FLAG | ... |
| 2 | Cross-platform alignment| OK/FLAG | ... |
| 3 | No orphaned code        | OK/FLAG | ... |
| 4 | Comments updated        | OK/FLAG | ... |
| 5 | Tests still relevant    | OK/FLAG | ... |
| 6 | Docs still accurate     | OK/FLAG | ... |
```

Only flag items that need attention. Keep notes concise.
