@AGENTS.md
@docs/output-contract.md

# Claude Code

Everything above is shared with every agent. This section is Claude Code–specific.

- **Path-scoped rules** — `.claude/rules/*.md` are pointers: each carries a `paths:` glob and names the `docs/` guide to read, once per session, when a matching file is first touched. Rule content lives only in `docs/`.
- **Skills** (`.claude/skills/`) — `fetch-emoji` (re-pull upstream Unicode + CLDR data for a version bump).
