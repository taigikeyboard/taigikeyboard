---
name: deps
description: Evaluate which third-party dependencies can be upgraded — Rust crates (engine / desktop / windows / linux workspaces), Android Gradle deps + plugins + wrapper, Swift packages (macOS + iOS), Python (dictionary pipeline, taigi-emojis), mise tools, open Dependabot PRs/alerts. Classifies every candidate SAFE / REVIEW / MAJOR / BLOCKED / USER-ONLY against the repo's standing pins and proposes PR groupings. Use when asked "what can we upgrade", "check outdated packages", "dependency audit". Read-only — never edits manifests or lockfiles, never builds, branches or opens a round. Args: scope `rust` | `android` | `apple` | `python` | `tools` | `all` (default all). App-version upgrade compatibility is /upgrade-check.
disable-model-invocation: false
---

# Dependency upgrade evaluation

Answer one question: **which dependencies can move today, what does each move cost, and which are held on purpose?**

Output is a report; the USER decides which upgrade rounds open (`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope). Never write "deferred", "post-vX" or "known limitation".

**Read-only.** Only dry-run / query commands. Never run `cargo update` without `--dry-run`, `mise upgrade`, `uv lock`, or edit any manifest. `ios/TaigiKeyboard.xcodeproj/**` (incl. its `Package.resolved`) is USER-only — report, never touch.

## Classification

| Tier | Meaning |
|---|---|
| **SAFE** | Semver-compatible lockfile bump inside current requirements (`cargo update` would take it), or a patch release; no pin touched |
| **REVIEW** | Minor bump that changes the manifest requirement, or a toolchain / plugin / wrapper bump; read the changelog, list what could break |
| **MAJOR** | Breaking version; name the API break from the changelog and the call sites it hits (`file:line`) |
| **BLOCKED** | Held by a recorded constraint (table below). Re-verify the constraint still holds; if it no longer does, re-tier it and say why |
| **USER-ONLY** | Only the USER can apply it (Xcode project packages) |

## 1. Standing pins — load first

Every pin carries its reason next to the version. Read these before classifying; a candidate that hits one is BLOCKED, not MAJOR.

| Source | What it holds |
|---|---|
| `.github/dependabot.yml` `ignore:` blocks | windows-rs sub-crate majors (`windows-core` / `windows-numerics` / `windows-sys`) — family moves together with the `windows-reactor` git rev in `windows/Cargo.toml`; `sha2` 0.10 |
| `android/app/build.gradle.kts` comments above `dependencies` | `core-ktx`, `lifecycle` group, `compose-bom` capped by `minCompileSdk=37` while `compileSdk = 36`; `protobuf-javalite` tied to protoc |
| `android/app/build.gradle.kts` `ktlint("…")` | ktlint frozen at 1.5.0 (USER 2026-08-18) |
| `mise.toml` comments | `protoc` = the libprotoc matching `protobuf-javalite`; `gitleaks` = `.gitleaks-scanned`; `swiftformat` = `.github/workflows/checks.yml` |
| `macos/Package.swift` | `KeyboardShortcuts` `exact:` pin |
| project memory `project_dependency_upgrade_audit.md` | history + traps (lifecycle whole-group rule, KK binary target) |

Grep for new pins too, manifests only (source and test comments say "pinned" constantly): `git ls-files '*Cargo.toml' '*.gradle' '*.kts' 'mise.toml' '*Package.swift' '*requirements*.txt' '*pyproject.toml' .github/dependabot.yml | xargs grep -n -iE 'capped|held at|pinned|exact pin|do not (bump|upgrade)'`

## 2. Scope → procedure

Run independent ecosystems in parallel (one message, several Bash calls). Network is required.

### rust

Each tracked `Cargo.lock` is its own workspace: `git ls-files '*Cargo.lock'` (engine, desktop, windows, linux).

1. Compatible bumps: `cargo update --manifest-path <ws>/Cargo.toml --dry-run` → every `Updating a vX -> vY` line is SAFE (group per workspace; list only direct deps by name, count transitive).
2. Majors / out-of-range for **direct** deps: `cargo metadata --manifest-path <ws>/Cargo.toml --format-version 1 --no-deps` → each `dependencies[].req` from crates.io; latest via `curl -s -H 'User-Agent: taigikeyboard-deps' https://crates.io/api/v1/crates/<name> | jq -r .crate.max_stable_version`. Latest outside `req` → REVIEW (0.x minor = breaking in Cargo → MAJOR) / MAJOR.
3. Same crate in several workspaces → report once with every workspace it lives in; a bump should land in all of them together.
4. Git deps (`git =` / `rev =`): `git ls-remote <url> HEAD` vs pinned rev; REVIEW.
5. Toolchains: every `rust-toolchain.toml` is `channel = "stable"` → nothing to evaluate unless a file pins a number.

### android

1. Query repository metadata directly — `dependencyUpdates` does not exist (the ben-manes plugin is only version-declared in `android/settings.gradle` `pluginManagement`, never applied, and this skill does not edit build files).
   - Libraries: every `"g:a:v"` string in `android/app/build.gradle.kts` → `https://dl.google.com/android/maven2/<g path>/<a>/maven-metadata.xml`, else `https://repo1.maven.org/maven2/…/maven-metadata.xml`; highest version without `alpha|beta|rc|dev|-M<n>|eap|snapshot`.
   - Plugins in `android/build.gradle`: AGP → Google Maven `com.android.tools.build:gradle`; Kotlin compose plugin → Maven Central `org.jetbrains.kotlin:compose-compiler-gradle-plugin` (its Plugin Portal marker is stale at 2.0.0); others → `https://plugins.gradle.org/m2/<id path>/<id>.gradle.plugin/maven-metadata.xml`.
   - Gradle wrapper: `curl -s https://services.gradle.org/versions/current | jq -r .version` vs `android/gradle/wrapper/gradle-wrapper.properties`.
   - ktlint: Maven Central `com.pinterest.ktlint:ktlint-cli`.
2. **minCompileSdk gate** for every androidx / Compose / Material candidate — the POM does not show it. Fetch the aar from `https://dl.google.com/android/maven2/<group path>/<artifact>/<ver>/<artifact>-<ver>.aar`, `unzip -p … META-INF/com/android/build/gradle/aar-metadata.properties`, read `minCompileSdk=`. Above `compileSdk` → BLOCKED.
3. **Whole-group rule**: androidx publishes same-version constraints per group. Check every artifact of the group (read the `-android` variants — a KMP root such as `lifecycle-runtime-ktx` ships no aar-metadata), the group is capped by its strictest member. For `compose-bom` check the `ui` version the BOM resolves.
4. AGP ↔ Gradle ↔ Kotlin compose plugin: a plugin bump that needs a newer Gradle or compileSdk is REVIEW with the coupled bumps named.
5. `protobuf-javalite`: BLOCKED unless the same bump moves `mise.toml` `protoc` and regenerates protos (`engine/scripts/gen-platform-protos.sh`).

### apple

1. macOS: `macos/Package.swift` requirements + `macos/Package.resolved` versions.
2. iOS: `ios/TaigiKeyboard.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved` → every row USER-ONLY (still state SAFE/MAJOR underneath).
3. Latest per package: `git ls-remote --tags --refs <repositoryURL> | awk -F/ '{print $NF}' | sed 's/^v//' | awk '!/-/' | sort -V | tail -3` (strip `v` first — repos mix `v1.x` and `3.x` tags, and `sort -V` ranks `v…` above digits).
4. KeyboardKit ≥ 10.9 is a closed binary target — changelog from the release notes (`gh release view -R KeyboardKit/KeyboardKit <tag>`), API check per `.claude/rules/doc-lookup.md`. Any KK bump needs spacebar-drag + autocap + settings dogfood (memory).
5. `ios/Vendor/ISEmojiView` is vendored source — report its upstream latest only; upgrading is a manual port.

### python

1. `dictionary/requirements.txt` (exact `==` pins): `curl -s https://pypi.org/pypi/<pkg>/json | jq -r .info.version`. A bump means `make dict` + `make build` + the Python CI (`.github/workflows/python.yml`) — REVIEW at minimum.
2. `taigi-emojis`: `uv tree --outdated --depth 1 --directory taigi-emojis`.

### tools

1. Per tool in `mise.toml`: `mise latest <tool>` (`mise outdated` reports an exact pin as its own latest). Every pinned row is BLOCKED by its `mise.toml` comment unless the coupled file moves too. `[MISSING]` = not installed on this host (`mise install`), not an upgrade finding.
2. GitHub Actions: covered by Dependabot — step 3 only.

### Dependabot (always, any scope)

- Open PRs: `gh pr list -R taigikeyboard/taigikeyboard --label dependencies --state open`.
- Alerts: `gh api repos/taigikeyboard/taigikeyboard/dependabot/alerts --jq '.[] | select(.state=="open") | [.security_advisory.severity, .dependency.package.name, .security_vulnerability.first_patched_version.identifier] | @tsv'`. A security alert outranks every other finding; if it hits a BLOCKED pin, say so explicitly.

## 3. Evidence per MAJOR / REVIEW row

- Changelog / release notes link or quoted breaking line — never from model memory.
- Call sites of the changed API (`file:line`, LSP references preferred over grep).
- Build cost: which stale-artifact gate the bump triggers (`CLAUDE.md` § Build & Test: `engine/` → `make build`, `dictionary/` → `make dict` + `make build`) and which platforms' post-PR verification it needs.

## 4. Report

Print in the terminal (offer an artifact only if the USER wants to share it):

1. **One line**: counts per tier + any open security alert.
2. **Table per ecosystem**, most actionable first: `| Package | Current | Latest | Tier | Why / blocker | Touches |`.
3. **Re-verified pins**: each BLOCKED row → still holds / no longer holds (with the evidence).
4. **Suggested PR grouping** — one PR per ecosystem-and-risk (e.g. "rust SAFE lock refresh, 4 workspaces", "android plugin + wrapper", one PR per MAJOR). Size and build gates per group; no release assignment.
5. **Next action**: the single group to start with and the cue to open it.

If a finding changes a standing pin's status, suggest updating `project_dependency_upgrade_audit.md` — do not write memory from inside this skill.
