Run all test suites across the project and report results:

1. **Android unit tests**: `cd android && ./gradlew test`
2. **taigi-converter tests**: `cd taigi-converter && node --test tests/`

For each suite, report: pass/fail count and any failure details.
If a suite fails, continue running the remaining suites before reporting.
