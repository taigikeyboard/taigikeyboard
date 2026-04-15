# Code Review Rules (Cross-Platform)

Mandatory checklist for reviewing code changes. Apply in order before approving any modification.

## 1. Platform Best Practices

Idiomatic patterns for the target platform. Consistency with existing codebase overrides personal preference.

## 2. No Redundancy

- Duplicated logic → extract shared function
- Duplicated constant/string → extract named constant
- Cross-platform: consistent naming and structure for shared behavior

**Check**: Search for the same string, value, or logic elsewhere before adding new code.

## 3. Clean Code

- Single responsibility — one function/class does one thing
- Proper decoupling — no hidden dependencies between unrelated modules
- Clear boundaries — public API vs internal logic should be obvious

**Check**: Can you describe what this function does in one sentence without "and"?

## 4. No Over-Engineering

- Simplest solution that works — no speculative abstractions
- No wrappers/frameworks for a single use case
- Three similar lines is better than a premature abstraction

## 5. Regression Risk

- Trace all callers of modified functions — verify behavior is preserved
- Check cross-file dependencies (imports, protocols/interfaces, shared state)
- If one platform changes, verify the other is unaffected or needs a matching update

**Check**: List every file that calls the modified code. Confirm each still works.

## 6. Readability

Follow `rules/ai-friendly-code.md` for naming and structure. Additionally:

- Prefer early returns over deep nesting
- A function should be understandable from its file alone where possible
- No misleading or outdated comments — wrong comments are worse than none

## 7. Efficient Review Scope

- Only read relevant files and sections — not entire files for a one-function change
- Review the change as submitted, not aspirational improvements
- Unrelated changes mixed in → request split into separate commits
