@AGENTS.md
@docs/contributing/known-pitfalls.md
@docs/contributing/doc-lookup.md

# Claude Code

Everything above is shared with every agent. This section is Claude Code–specific.

- **Path-scoped rules** — `.claude/rules/*.md` are pointers: each carries a `paths:` glob and names the `docs/contributing/` guide to read when a matching file is touched. Rule content lives only in `docs/contributing/`.
- **Hook** — `.claude/hooks/block-project-config.sh` blocks edits to `.xcodeproj` / `.pbxproj` / `.xcworkspace` (Core Principle #1).
- **Skills** (`.claude/skills/`) — `audit`, `deps`, `upgrade-check` (read-only reports); `dogfood` (manual test sentences from `corpus/`); `e2e` (end-to-end typing run); `previews` (regenerate the mobile Theme / Layout card screenshots from the real keyboard view); `release-desktop`, `release-mobile` (prepare a release, stop before publishing — run only on the maintainer's instruction).
- **Agents** (`.claude/agents/`) — `phonetics-specialist` (cited TL/POJ/TPS answers), `refactor-reviewer` (behaviour-freeze review of a refactor diff).
- **Docs lookup** — the `find-docs` skill wraps the Context7 CLI that `docs/contributing/doc-lookup.md` names.
- **LSP** — rust-analyzer / swift / kotlin plugins: prefer go-to-definition / references over grep + Read chains.
- **Maintainer-private instructions** — a gitignored `CLAUDE.local.md` at the repository root, when present.
