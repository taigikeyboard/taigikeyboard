@AGENTS.md
@docs/contributing/known-pitfalls.md

# Claude Code

Everything above is shared with every agent. This section is Claude Code–specific.

- **Path-scoped rules** — `.claude/rules/*.md` are pointers: each carries a `paths:` glob and names the `docs/contributing/` guide to read, once per session, when a matching file is first touched. Rule content lives only in `docs/contributing/`.
- **Enforcement** (`.claude/settings.json`) — `.claude/hooks/block-project-config.sh` plus `permissions.deny` block edits to `.xcodeproj` / `.pbxproj` / `.xcworkspace` (Core Principle #1); a `PreToolUse` hook runs `make lint` before `gh pr create` and blocks the PR on failure.
- **Skills** (`.claude/skills/`) — `audit`, `deps`, `upgrade-check` (read-only reports); `dogfood` (manual test sentences from `corpus/`); `e2e` (end-to-end typing run); `previews` (regenerate the mobile Theme / Layout card screenshots from the real keyboard view); `release-desktop` (one desktop platform's hotfix patch — the monthly desktop release is the automatic train, `docs/architecture/desktop-release.md`), `release-mobile` (prepare a mobile release); both stop before publishing — run only on the maintainer's instruction.
- **Agents** (`.claude/agents/`) — `Explore` (Haiku override of the built-in search agent), `log-reader` (Haiku: build / test log → per-target failures), `phonetics-specialist` (cited TL/POJ/TPS answers), `refactor-reviewer` (behaviour-freeze review of a refactor diff).
- **LSP** — rust-analyzer / swift / kotlin plugins: prefer go-to-definition / references over grep + Read chains.
- **Maintainer-private instructions** — a gitignored `CLAUDE.local.md` at the repository root, when present.
