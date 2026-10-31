#!/usr/bin/env python3
"""Select the build / test commands a change needs — "test only what changed".

Changed files come from `--base <ref>` (`git diff <base>...HEAD` plus staged,
unstaged and untracked files; default base = merge-base with origin/main),
`--range A..B` (a finished range, e.g. `<merge>^1..<merge>`), or `--paths`.
Each path maps to commands grouped by platform (PLATFORMS below).

Engine: a path under an engine crate selects that crate and every workspace
crate that depends on it through normal / build dependencies (transitively),
plus crates with a dev-dependency on any of those (one hop: a dev-dependency
reaches the dependent's tests, not the dependent's own dependents). A change
only under a crate's `tests/` or `benches/` selects that crate alone. protos,
the workspace manifest, lockfile or toolchain select every crate. Shipped
dispatch code reaching the selection also selects desktop + windows + linux,
which link dispatch; a direct change to the FFI wire surface (protos, dispatch,
swift-ffi, android-jni, engine/scripts) also selects ios + android + macos
behind the stale-artifact prerequisite `make build`.

macOS has two gates: `macos-rust` (the macOS Cargo workspace's native tests +
clippy + fmt) and `macos` (the Swift suite). An input of the archive the app
links — `macos/crates/**`, the workspace manifest / lockfile / toolchain, and
`desktop/**` — selects both, behind `make build`; an engine change reaching
swift-ffi selects `macos-rust`, as one reaching dispatch selects the desktop
shells.

A path no rule covers is reported as unmapped (exit 2): add its rule here.

Usage:
  python3 tools/test_select.py [--base REF | --range A..B | --paths P ...]
                               [--platform engine,macos] [--json] [--run]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

PLATFORMS = (
    "engine",
    "desktop",
    "windows",
    "linux",
    "macos-rust",
    "macos",
    "ios",
    "android",
    "python",
    "i18n",
    "converter",
    "docs-checks",
)

ENGINE_MANIFEST = "engine/Cargo.toml"
ENGINE_WHOLE_WORKSPACE_FILES = {
    "engine/Cargo.toml",
    "engine/Cargo.lock",
    "engine/rust-toolchain.toml",
}
# Crates whose change is a wire-surface change for the platforms that link the
# engine binary (the committed bindings / xcframework / jniLibs).
WIRE_SURFACE_CRATES = {"protos", "dispatch", "swift-ffi", "android-jni"}
# Crates a desktop shell links directly (desktop/, windows/, linux/ Cargo.toml).
DESKTOP_LINKED_CRATE = "dispatch"
LEAF_ONLY_SUBDIRS = ("tests/", "benches/")

# Destination by device name, not a simulator UUID: a UUID is per-machine and changes whenever the
# simulator is recreated (OS update), which turned the gate red with "Unable to find a device".
IOS_TEST = (
    "xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests "
    "-destination 'platform=iOS Simulator,name=iPhone 17' test"
)
MAKE_BUILD_PREREQUISITE = "run `make build` first (stale-artifact gate) — ios / android / macos link the engine binary"
# The macOS Cargo workspace: the `taigi-macos-ffi` archive the Swift app links.
MACOS_RUST_PATHS = (
    "macos/crates/",
    "macos/Cargo.toml",
    "macos/Cargo.lock",
    "macos/rust-toolchain.toml",
)
# The engine crate whose closure the macOS archive links (with dispatch and
# protos, both below it).
MACOS_LINKED_CRATE = "swift-ffi"
ADMIN_LANE_NOTE = "no build/test gate (admin lane)"

# Top-level directories whose files need no build or test gate.
NO_GATE_DIRS = {
    ".claude",
    ".githooks",
    ".github",
    "changelog",
    "corpus",
    "docs",
    "media",
}
INVARIANT_PREFIXES = ("docs/", "tools/invariant_labels")
INVARIANT_SUFFIXES = (".rs", ".kt", ".swift")


@dataclass(frozen=True)
class Command:
    run: str  # shell command, run from `cwd` (repo-relative)
    cwd: str = "."

    def display(self) -> str:
        return self.run if self.cwd == "." else f"(cd {self.cwd} && {self.run})"


@dataclass(frozen=True)
class EngineGraph:
    crates: tuple[str, ...]  # workspace order
    crate_dirs: dict[str, str]  # crate → repo-relative dir ("engine/phonetics")
    dependents: dict[str, set[str]]  # crate → crates with a normal/build dep on it
    dev_dependents: dict[str, set[str]]  # crate → crates with a dev-dep on it

    def crate_of(self, path: str) -> str | None:
        best = None
        for crate, directory in self.crate_dirs.items():
            if path.startswith(directory + "/") and (
                best is None or len(directory) > len(self.crate_dirs[best])
            ):
                best = crate
        return best

    def affected_by(self, changed: Iterable[str]) -> set[str]:
        """Changed crates + their transitive normal/build dependents + one dev-dep hop."""
        closure = set(changed)
        frontier = list(closure)
        while frontier:
            for dependent in self.dependents.get(frontier.pop(), ()):
                if dependent not in closure:
                    closure.add(dependent)
                    frontier.append(dependent)
        dev = {d for crate in closure for d in self.dev_dependents.get(crate, ())}
        return closure | dev


def load_engine_graph(root: Path = ROOT) -> EngineGraph:
    metadata = json.loads(
        subprocess.run(
            [
                "cargo",
                "metadata",
                "--no-deps",
                "--offline",
                "--format-version",
                "1",
                "--manifest-path",
                str(root / ENGINE_MANIFEST),
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    names = {p["name"] for p in packages}
    crate_dirs = {
        p["name"]: Path(p["manifest_path"]).parent.relative_to(root).as_posix()
        for p in packages
    }
    dependents: dict[str, set[str]] = {}
    dev_dependents: dict[str, set[str]] = {}
    for package in packages:
        for dependency in package["dependencies"]:
            if dependency["name"] not in names:
                continue
            target = dev_dependents if dependency.get("kind") == "dev" else dependents
            target.setdefault(dependency["name"], set()).add(package["name"])
    return EngineGraph(
        tuple(p["name"] for p in packages), crate_dirs, dependents, dev_dependents
    )


@dataclass
class Effects:
    engine_crates: set[str] = field(default_factory=set)  # with reverse closure
    engine_leaf_crates: set[str] = field(default_factory=set)  # that crate only
    engine_whole: bool = False
    wire_surface: bool = False
    macos_archive: bool = False  # an input of the archive the Swift app links
    platforms: set[str] = field(default_factory=set)  # whole-platform gates
    python: set[str] = field(default_factory=set)  # PYTHON_SUITES keys
    swift_dirs: set[str] = field(default_factory=set)  # "ios" / "macos"
    invariant_labels: bool = False
    notes: list[str] = field(default_factory=list)
    unmapped: list[str] = field(default_factory=list)


PYTHON_SUITES = {
    "dictionary": Command("python3 -m pytest tests -q", "dictionary"),
    "tools": Command("python3 -m unittest discover -s tools -p '*_test.py'"),
    "kautian-watch": Command("python3 -m pytest tools/kautian_watch_test.py -q"),
    "tools-windows": Command("python3 -m pytest tools/windows/tests -q"),
    "e2e": Command("python3 -m unittest analyze_test", "e2e/analyzer"),
    "emoji": Command("python3 -m pytest -q", "emoji"),
}


def add_engine_path(path: str, graph: EngineGraph, effects: Effects) -> None:
    if path in ENGINE_WHOLE_WORKSPACE_FILES:
        effects.engine_whole = True
        return
    if path.startswith("engine/scripts/"):
        # Binding / xcframework / jniLibs build scripts: no Rust test reads them.
        effects.wire_surface = True
        return
    if path == "engine/deny.toml":
        effects.notes.append("engine/deny.toml: cargo-deny runs in CI (security.yml)")
        return
    crate = graph.crate_of(path)
    if crate is None:
        effects.engine_whole = True
        effects.notes.append(f"{path}: in no engine crate — selecting every crate")
        return
    if crate == "protos":
        effects.engine_whole = True
        effects.wire_surface = True
        return
    inside = path[len(graph.crate_dirs[crate]) + 1 :]
    if inside.startswith(LEAF_ONLY_SUBDIRS):
        effects.engine_leaf_crates.add(crate)
        return
    effects.engine_crates.add(crate)
    if crate in WIRE_SURFACE_CRATES:
        effects.wire_surface = True


def add_dictionary_path(path: str, effects: Effects) -> None:
    if path.startswith("dictionary/output/"):
        # Test inputs of the composing / lexicon suites, not shipped data.
        effects.engine_leaf_crates |= {"composing", "lexicon"}
        return
    effects.python.add("dictionary")
    if not (path.endswith(".py") or path.startswith("dictionary/tests/")):
        effects.notes.append(
            f"{path}: dictionary source — rebuild with `make dict` then `make build`"
        )


def add_tools_path(path: str, effects: Effects) -> None:
    if path in {"tools/kautian_watch.py", "tools/kautian_watch_test.py"}:
        effects.python.add("kautian-watch")
    elif path.startswith("tools/i18n/"):
        effects.platforms.add("i18n")
    elif path.startswith("tools/windows/"):
        effects.python.add("tools-windows")
    elif path.startswith("tools/desktop/"):
        effects.notes.append(f"{path}: icon tool — no test gate")
    elif path.endswith(".sh"):
        # Release, secret-scan and reference-sync scripts have no test suite.
        pass
    elif path.count("/") == 1:
        effects.python.add("tools")
    else:
        effects.unmapped.append(path)


def add_assets_path(path: str, effects: Effects) -> None:
    if path.startswith("assets/dictionaries/"):
        effects.engine_leaf_crates |= {"composing", "lexicon", "dispatch"}
        effects.platforms.add("desktop")
    elif path.startswith("assets/symbols/"):
        effects.platforms |= {"desktop", "windows", "linux", "macos"}
    elif path.startswith("assets/fonts/"):
        # Typefaces ship as-is: no build or test gate.
        pass
    else:
        effects.unmapped.append(path)


def effects_of(paths: Iterable[str], graph: EngineGraph) -> Effects:
    effects = Effects()
    for path in paths:
        if path.endswith(INVARIANT_SUFFIXES) or path.startswith(INVARIANT_PREFIXES):
            effects.invariant_labels = True
        if path.endswith(".md"):
            continue
        top = path.split("/", 1)[0]
        if path.endswith(".swift") and top in ("ios", "macos"):
            effects.swift_dirs.add(top)
        if "/" not in path:
            if path == ".swiftformat":
                effects.swift_dirs |= {"ios", "macos"}
            elif path == "taigi-converter":
                effects.platforms.add("converter")
                effects.python.add("dictionary")
            continue
        if top in NO_GATE_DIRS:
            continue
        if top == "engine":
            add_engine_path(path, graph, effects)
        elif top == "dictionary":
            add_dictionary_path(path, effects)
        elif top == "assets":
            add_assets_path(path, effects)
        elif top == "tools":
            add_tools_path(path, effects)
        elif top == "i18n":
            effects.platforms |= {"i18n", "ios", "android", "macos"}
        elif top == "desktop":
            effects.platforms |= {"desktop", "windows", "linux", "macos-rust", "macos"}
            effects.macos_archive = True
        elif top == "e2e":
            effects.python.add("e2e")
        elif top == "emoji":
            effects.python.add("emoji")
        elif top in ("ios", "macos", "android", "windows", "linux"):
            effects.platforms.add(top)
            if path.startswith(MACOS_RUST_PATHS):
                effects.platforms.add("macos-rust")
                effects.macos_archive = True
        else:
            effects.unmapped.append(path)
    return effects


@dataclass
class Selection:
    commands: dict[str, list[Command]]  # platform → commands, PLATFORMS order
    notes: list[str]
    prerequisites: list[str]
    unmapped: list[str]
    engine_crates: list[str]


def engine_commands(crates: list[str], select_dispatch_trace: bool) -> list[Command]:
    packages = " ".join(f"-p {crate}" for crate in crates)
    manifest = f"--manifest-path {ENGINE_MANIFEST}"
    commands = [Command(f"cargo test {manifest} {packages}")]
    if select_dispatch_trace:
        commands.append(
            Command(f"cargo test {manifest} -p dispatch --features e2e-trace")
        )
    commands.append(
        Command(
            f"cargo clippy {manifest} {packages} --all-targets --locked -- -D warnings"
        )
    )
    commands.append(Command(f"cargo fmt --all --check {manifest}"))
    return commands


PLATFORM_COMMANDS = {
    "desktop": [Command("make desktop-check")],
    "windows": [Command("make windows-check")],
    "linux": [Command("make linux-check")],
    "macos-rust": [Command("make macos-rust-check")],
    "macos": [Command("make -C macos test")],
    "ios": [Command(IOS_TEST)],
    "android": [
        Command("android/gradlew -p android :app:spotlessCheck :app:testDebugUnitTest")
    ],
    "i18n": [
        Command("python3 tools/i18n/test_i18n.py"),
        Command("python3 tools/i18n/check.py"),
    ],
    "converter": [Command("npm test", "taigi-converter")],
}


def select(paths: Iterable[str], graph: EngineGraph) -> Selection:
    effects = effects_of(paths, graph)
    notes = list(effects.notes)
    platforms = set(effects.platforms)

    shipped = (
        set(graph.crates)
        if effects.engine_whole
        else graph.affected_by(effects.engine_crates)
    )
    selected = shipped | effects.engine_leaf_crates
    if DESKTOP_LINKED_CRATE in shipped:
        platforms |= {"desktop", "windows", "linux"}
    if MACOS_LINKED_CRATE in shipped:
        platforms.add("macos-rust")
    if effects.wire_surface:
        platforms |= {"ios", "android", "macos"}
    prerequisites = []
    if effects.wire_surface or effects.macos_archive:
        prerequisites.append(MAKE_BUILD_PREREQUISITE)

    commands: dict[str, list[Command]] = {}
    engine_crates = [crate for crate in graph.crates if crate in selected]
    if engine_crates:
        commands["engine"] = engine_commands(engine_crates, "dispatch" in selected)
        if len(engine_crates) == len(graph.crates):
            notes.append(
                "every engine crate selected — the rule's outcome, same scope as `cargo test --workspace`"
            )
    for platform in PLATFORMS:
        platform_commands = []
        if platform in effects.swift_dirs:
            platform_commands.append(Command(f"swiftformat --lint {platform}"))
        if platform in platforms:
            platform_commands += PLATFORM_COMMANDS[platform]
        if platform_commands:
            commands[platform] = platform_commands
    if effects.python:
        commands["python"] = [
            PYTHON_SUITES[k] for k in PYTHON_SUITES if k in effects.python
        ]
    if effects.invariant_labels:
        commands["docs-checks"] = [Command("python3 tools/invariant_labels.py")]
    if set(commands) <= {"docs-checks"} and not effects.unmapped:
        notes.append(ADMIN_LANE_NOTE)

    ordered = {p: commands[p] for p in PLATFORMS if p in commands}
    return Selection(ordered, notes, prerequisites, effects.unmapped, engine_crates)


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(ROOT), "-c", "core.quotepath=false", *args],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def split_z(output: str) -> list[str]:
    return [path for path in output.split("\0") if path]


def changed_since(base: str) -> list[str]:
    paths = split_z(git("diff", "--name-only", "-z", f"{base}...HEAD"))
    paths += split_z(git("diff", "--name-only", "-z", "HEAD"))
    paths += split_z(git("ls-files", "--others", "--exclude-standard", "-z"))
    return sorted(set(paths))


def changed_in_range(range_spec: str) -> list[str]:
    start, _, end = range_spec.partition("..")
    if not start or not end:
        raise SystemExit(f"--range needs A..B, got {range_spec!r}")
    return sorted(set(split_z(git("diff", "--name-only", "-z", start, end))))


def default_base() -> str:
    return git("merge-base", "origin/main", "HEAD").strip()


def print_selection(selection: Selection, changed: list[str]) -> None:
    print(f"{len(changed)} changed file(s)")
    if selection.engine_crates:
        print(f"engine crates: {', '.join(selection.engine_crates)}")
    for prerequisite in selection.prerequisites:
        print(f"⚠ prerequisite: {prerequisite}")
    for note in selection.notes:
        print(f"• {note}")
    for path in selection.unmapped:
        print(f"✗ unmapped: {path} — add a rule to tools/test_select.py")
    for platform, commands in selection.commands.items():
        print(f"[{platform}]")
        for command in commands:
            print(f"  {command.display()}")


def run_selection(selection: Selection) -> int:
    for prerequisite in selection.prerequisites:
        print(f"⚠ not run by --run: {prerequisite}")
    results: dict[str, str] = {}
    for platform, commands in selection.commands.items():
        results[platform] = "pass"
        for command in commands:
            print(f"==> [{platform}] {command.display()}", flush=True)
            finished = subprocess.run(
                command.run, shell=True, cwd=ROOT / command.cwd, check=False
            )
            if finished.returncode:
                results[platform] = f"FAIL: {command.display()}"
                break
    print("\nSummary")
    for platform, result in results.items():
        print(f"  {platform:12} {result}")
    return 1 if any(result != "pass" for result in results.values()) else 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    source = parser.add_mutually_exclusive_group()
    source.add_argument(
        "--base",
        help="diff <base>...HEAD + working tree (default: merge-base with origin/main)",
    )
    source.add_argument("--range", help="A..B: the files changed between two commits")
    source.add_argument("--paths", nargs="+", help="explicit repo-relative paths")
    parser.add_argument(
        "--platform", help=f"comma-separated subset of: {','.join(PLATFORMS)}"
    )
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    parser.add_argument("--run", action="store_true", help="execute the commands")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.paths:
        changed = sorted(set(args.paths))
    elif args.range:
        changed = changed_in_range(args.range)
    else:
        changed = changed_since(args.base or default_base())
    selection = select(changed, load_engine_graph())
    if args.platform:
        wanted = set(args.platform.split(","))
        if unknown := wanted - set(PLATFORMS):
            raise SystemExit(f"unknown platform(s): {', '.join(sorted(unknown))}")
        selection.commands = {
            p: c for p, c in selection.commands.items() if p in wanted
        }
    if args.json:
        print(
            json.dumps(
                {
                    "changed": changed,
                    "engine_crates": selection.engine_crates,
                    "commands": {
                        p: [{"run": c.run, "cwd": c.cwd} for c in cs]
                        for p, cs in selection.commands.items()
                    },
                    "prerequisites": selection.prerequisites,
                    "notes": selection.notes,
                    "unmapped": selection.unmapped,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        print_selection(selection, changed)
    if selection.unmapped:
        return 2
    return run_selection(selection) if args.run else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
