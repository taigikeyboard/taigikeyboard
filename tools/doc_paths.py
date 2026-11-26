#!/usr/bin/env python3
"""Every repo path a living doc names in a code span or link still exists.

Agents trust a path a doc names and go read it; a path that was moved or
deleted sends them to nothing, or to a guess. Living docs are the root docs,
`docs/**`, `.claude/**` and every `README.md` / `AGENTS.md`. Skipped: dated
snapshots (`docs/reports/`, `docs/releases/`), `docs/references/` (paths
written relative to the cloned external repos), vendored code, and
`docs/architecture/*-roadmap*.md` design records (files as they were, or were
planned, when the phases ran).

A code span is a path when its first segment is a tracked top-level directory
and it holds no glob, placeholder or space. It resolves from the repo root,
from the doc's directory, or as the tail of exactly one tracked path
(`engine/PhoneticsBridge.kt` for the Android file). A link resolves from the
doc's directory (a leading `/` from the repo root), never by tail. Submodule
and gitignored paths pass — neither exists on CI. Name another repository's
file by URL, not in a code span.

Usage: python3 tools/doc_paths.py   (exit 1 lists every dead path)
"""

from __future__ import annotations

import posixpath
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path, PurePosixPath
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
ROOT_DOCS = ("CLAUDE.md", "REVIEW.md", "CONTRIBUTING.md")
DOC_DIRS = ("docs/", ".claude/")
DOC_NAMES = ("README.md", "AGENTS.md")
SKIPPED = ("docs/reports/", "docs/releases/", "docs/references/")
VENDORED = "/Vendor/"
DESIGN_RECORD = re.compile(r"^docs/architecture/[^/]+-roadmap[^/]*\.md$")

CODE_SPAN = re.compile(r"`([^`\n]+)`")
INLINE_LINK = re.compile(r"\]\(\s*(?:<([^>\n]+)>|([^)\s]+))(?:\s+\"[^\"]*\")?\s*\)")
REFERENCE_DEFINITION = re.compile(r"^ {0,3}\[[^\]]+\]:\s*<?([^>\s]+)>?")
# A link to a URL, text included: its text names a file in another repository.
URL_LINK = re.compile(r"\[[^\]\n]*\]\(\s*<?[a-z][a-z0-9+.-]*://[^)\n]*\)")
# CommonMark fence: up to three spaces of indent, three or more backticks or
# tildes, closed by a run of the same character at least as long.
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})[^\n]*\n.*?^ {0,3}\1[`~]*[ \t]*$", re.S | re.M)
# Glob, shell and template characters, or a version placeholder
# (`MAJOR.MINOR.PATCH`, `YYYY.M.P`): the span is a pattern, not a path.
NOT_A_PATH = re.compile(r"[*?{}<>$\[\]|\s,=()]|\.\.\.|…|[A-Z]+\.[A-Z]+\.[A-Z]+")
# `:12`, `:L12`, `:12-30`, `::symbol`, `#anchor` after the path.
SUFFIX = re.compile(r"(::.*|:L?\d.*|#.*)$")


def git(*args: str, stdin: str = "", repo: Path = ROOT) -> list[str]:
    """NUL-separated git output, so non-ASCII names come back unquoted."""
    result = subprocess.run(
        ["git", "-C", str(repo), "-c", "core.excludesFile=/dev/null", *args, "-z"],
        input=stdin,
        capture_output=True,
        text=True,
        check=False,
    )
    # check-ignore exits 1 when no path is ignored.
    allowed = (0, 1) if args[0] == "check-ignore" else (0,)
    if result.returncode not in allowed:
        raise RuntimeError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return [entry for entry in result.stdout.split("\0") if entry]


def is_living_doc(path: str) -> bool:
    if not path.endswith(".md") or path.startswith(SKIPPED) or VENDORED in f"/{path}":
        return False
    return (
        path in ROOT_DOCS
        or path.rsplit("/", 1)[-1] in DOC_NAMES
        or (path.startswith(DOC_DIRS) and not DESIGN_RECORD.match(path))
    )


def link_target(doc_dir: PurePosixPath, raw: str) -> str | None:
    """Repo path a link points at; None for an external, anchor-only or root link.

    A link out of the repository keeps its leading `..`, which matches nothing.
    """
    if "://" in raw or raw.startswith(("mailto:", "#")):
        return None
    target = unquote(SUFFIX.sub("", raw))
    if target.strip("/") == "":
        return None
    base = "" if target.startswith("/") else str(doc_dir)
    return posixpath.normpath(posixpath.join(base, target.lstrip("/")))


def references(doc: str, text: str, top_dirs: set[str]) -> list[tuple[int, list[str], bool]]:
    """(line number, candidate repo paths, tail allowed) for every path-like span or link.

    The reference resolves when any candidate exists; a span's candidates are
    its root and doc-relative readings.
    """
    text = FENCE.sub(lambda match: "\n" * match.group(0).count("\n"), text)
    doc_dir = PurePosixPath(doc).parent
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        line = URL_LINK.sub("", line)
        for raw in CODE_SPAN.findall(line):
            path = SUFFIX.sub("", raw).rstrip("/")
            if "/" in path and not NOT_A_PATH.search(path) and path.split("/")[0] in top_dirs:
                found.append((number, [path, posixpath.normpath(doc_dir / path)], True))
        without_spans = CODE_SPAN.sub("", line)
        raws = [a or b for a, b in INLINE_LINK.findall(without_spans)]
        raws += REFERENCE_DEFINITION.findall(without_spans)
        for raw in raws:
            if (target := link_target(doc_dir, raw)) is not None:
                found.append((number, [target], False))
    return found


def dead_paths(
    doc_refs: list[tuple[str, list[tuple[int, list[str], bool]]]],
    tracked: set[str],
    ignored: set[str],
    submodules: set[str],
) -> list[str]:
    existing = tracked | {str(parent) for path in tracked for parent in PurePosixPath(path).parents}
    tail_owners = Counter(
        "/".join(parts[start:])
        for path in existing
        for parts in [path.split("/")]
        for start in range(1, len(parts))
    )

    def resolves(path: str, tail_allowed: bool) -> bool:
        if path in existing or path in ignored:
            return True
        if any(path == sub or path.startswith(f"{sub}/") for sub in submodules):
            return True
        return tail_allowed and tail_owners[path] == 1

    return [
        f"{doc}:{number}: `{candidates[0]}` does not exist"
        for doc, refs in doc_refs
        for number, candidates, tail_allowed in refs
        if not any(resolves(path, tail_allowed) for path in candidates)
    ]


def ignored_paths(paths: set[str], repo: Path = ROOT) -> set[str]:
    """The paths a .gitignore rule matches, whether or not they exist on disk.

    A directory-only rule (`build/out/`) matches a bare path only when that
    directory exists, so a built tree and a clean CI checkout disagree; each
    path is also asked with a trailing `/`.
    """
    if not paths:
        return set()
    queries = "\0".join(f"{path}\0{path}/" for path in paths)
    return {hit.rstrip("/") for hit in git("check-ignore", "--no-index", "--stdin", stdin=queries, repo=repo)}


def _jj_tracked_entries(repo: Path) -> list[tuple[str, str]] | None:
    if not (repo / ".jj").is_dir():
        return None
    try:
        result = subprocess.run(
            [
                "jj",
                "--no-pager",
                "--color=never",
                "--quiet",
                "-R",
                str(repo),
                "file",
                "list",
                "-T",
                r'file_type ++ "\t" ++ path ++ "\0"',
            ],
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        return None
    if result.returncode != 0:
        raise RuntimeError(f"jj file list failed: {result.stderr.strip()}")
    entries: list[tuple[str, str]] = []
    for entry in result.stdout.split("\0"):
        if entry:
            kind, path = entry.split("\t", 1)
            entries.append((kind, path))
    return entries


def working_tree_paths(repo: Path = ROOT) -> tuple[set[str], set[str]]:
    """(existing tracked paths, submodule paths) in `repo`."""
    jj_entries = _jj_tracked_entries(repo)
    if jj_entries is not None:
        submodules = {path for kind, path in jj_entries if kind == "git-submodule"}
        tracked = {
            path
            for kind, path in jj_entries
            if kind == "git-submodule"
            or (repo / path).is_file()
            or (repo / path).is_symlink()
        }
        return tracked, submodules
    # `<mode> <object> <stage>\t<path>`; mode 160000 is a submodule.
    entries = [entry.split("\t", 1) for entry in git("ls-files", "--stage", repo=repo)]
    submodules = {path for meta, path in entries if meta.startswith("160000 ")}
    tracked = {
        path
        for _, path in entries
        if path in submodules or (repo / path).is_file() or (repo / path).is_symlink()
    }
    return tracked, submodules


def main() -> int:
    tracked, submodules = working_tree_paths(ROOT)
    top_dirs = {path.split("/")[0] for path in tracked if "/" in path}
    doc_refs = []
    for doc in sorted(filter(is_living_doc, tracked)):
        file = ROOT / doc
        if file.is_file():
            doc_refs.append((doc, references(doc, file.read_text(encoding="utf-8"), top_dirs)))
    unresolved = {path for _, refs in doc_refs for _, paths, _ in refs for path in paths} - tracked
    ignored = ignored_paths(unresolved)
    failures = dead_paths(doc_refs, tracked, ignored, submodules)
    if not failures:
        print(f"✓ {sum(len(refs) for _, refs in doc_refs)} doc paths exist")
        return 0
    print("\n".join(failures), file=sys.stderr)
    print(
        f"doc-paths: {len(failures)} dead path(s) — point the doc at the file's new"
        " location, drop the reference if the file is gone, or `git add` a new file;"
        " name another repository's file by URL.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
