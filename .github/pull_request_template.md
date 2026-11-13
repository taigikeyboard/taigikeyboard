## Summary

<!-- What changed and why, in 1-3 bullets. Link the issue if there is one. -->

## Platforms touched

- [ ] Engine (`engine/`)
- [ ] iOS
- [ ] Android
- [ ] macOS
- [ ] Windows
- [ ] Linux
- [ ] Dictionary (`dictionary/`)
- [ ] Docs / CI only

## Risk

<!-- Tier from REVIEW.md § Risk tiers (High / Normal / Low) and what could break. -->

## How to revert

<!-- `git revert <merge>` is enough? Or does it need a data migration rollback, a dictionary rebuild, or a version bump? -->

## Test plan

<!-- Commands per platform: docs/BUILDING.md § 4. -->

- [ ] Built and tested every platform ticked above (list the commands you ran)
- [ ] Engine or dictionary change: native artifacts regenerated before platform tests (docs/BUILDING.md § 5)
- [ ] Shared behaviour change: every platform that has it matches, or the divergence is recorded in `docs/architecture/behavioral-invariants.md`
- [ ] `i18n/` change: `make i18n` output committed
- [ ] Desktop-visible change: fragment in `changelog/unreleased/` (or label `no-changelog`)
- [ ] No personal data (addresses, account IDs) in the diff, commit messages or this description
