# PR numbers before the 2026-09-07 migration

> **Type**: Reference
> **Keywords**: `pull request`, `archive`, `migration`, `numbering`
> **Related**: ../../CLAUDE.md

---

## Summary

This repository was recreated on 2026-09-07 to shed pull-request refs that still
carried bug reporters' personal data. The full commit history came across intact;
the pull requests did not, and **numbering restarts at #1**. Every `#NNN` in
`docs/`, `changelog/`, `.claude/`, and project memory written before that date
refers to the old repository, `taigikeyboard/taigikeyboard-archive` (private).

## Resolving an old number

Resolve from git first — 528 squash-merge commits carry it in the subject, and the
commit is the diff:

    git log --all --oneline --grep="(#NNN)"

Only open the archive repository when you need the review discussion itself:

    gh pr view NNN -R taigikeyboard/taigikeyboard-archive

## Collisions

Old and new numbering will eventually collide. When a number resolves to two
different things, the archive is the older one.
