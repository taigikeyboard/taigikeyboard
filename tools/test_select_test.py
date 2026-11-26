"""Unit tests for the changed-files test selector (run against the real engine graph)."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_TOOLS_DIR = str(Path(__file__).resolve().parent)
if _TOOLS_DIR not in sys.path:
    sys.path.insert(0, _TOOLS_DIR)

import test_select
from test_select import DOCS_CHECKS, ROOT, Command

DOC_PATHS_CHECK = DOCS_CHECKS["doc-paths"]


class SelectorTestCase(unittest.TestCase):
    graph: test_select.EngineGraph

    @classmethod
    def setUpClass(cls) -> None:
        cls.graph = test_select.load_engine_graph()

    def select(self, *paths: str) -> test_select.Selection:
        return test_select.select(paths, self.graph)

    def runs(self, selection: test_select.Selection, platform: str) -> list[str]:
        return [command.run for command in selection.commands.get(platform, [])]


class EngineGraphTests(SelectorTestCase):
    def test_crate_of_uses_the_deepest_manifest_dir(self) -> None:
        self.assertEqual(self.graph.crate_of("engine/phonetics/src/tl.rs"), "phonetics")
        self.assertEqual(
            self.graph.crate_of("engine/build-helpers/fst-builder/src/main.rs"),
            "fst-builder",
        )
        self.assertIsNone(self.graph.crate_of("engine/Cargo.toml"))

    def test_leaf_crate_reaches_its_real_dependents(self) -> None:
        affected = self.graph.affected_by({"phonetics"})

        self.assertTrue(
            {"phonetics", "nextword", "userdata", "lexicon", "composing", "dispatch"}
            <= affected
        )
        self.assertTrue({"swift-ffi", "android-jni", "fst-builder"} <= affected)
        self.assertNotIn("ranking", affected)
        self.assertNotIn("mmap-host", affected)

    def test_top_crate_reaches_only_its_bindings(self) -> None:
        self.assertEqual(
            self.graph.affected_by({"dispatch"}),
            {"dispatch", "swift-ffi", "android-jni"},
        )

    def test_dev_dependents_are_one_hop(self) -> None:
        graph = test_select.EngineGraph(
            ("a", "b", "c"),
            {"a": "engine/a", "b": "engine/b", "c": "engine/c"},
            dependents={"b": {"c"}},
            dev_dependents={"a": {"b"}},
        )

        self.assertEqual(graph.affected_by({"a"}), {"a", "b"})


class EngineSelectionTests(SelectorTestCase):
    def test_leaf_source_change_selects_closure_and_desktop_shells(self) -> None:
        # dispatch reaches the Windows / Linux shells, swift-ffi the macOS
        # workspace's native gate; neither is a wire-surface change.
        selection = self.select("engine/phonetics/src/tl.rs")

        self.assertIn("dispatch", selection.engine_crates)
        test_run = self.runs(selection, "engine")[0]
        self.assertTrue(
            test_run.startswith(
                "cargo test --manifest-path engine/Cargo.toml -p phonetics"
            )
        )
        self.assertIn("-p dispatch", test_run)
        self.assertIn(
            "cargo test --manifest-path engine/Cargo.toml -p dispatch --features e2e-trace",
            self.runs(selection, "engine"),
        )
        self.assertEqual(
            set(selection.commands),
            {"engine", "desktop", "windows", "linux", "web", "macos-rust", "docs-checks"},
        )
        self.assertEqual(selection.prerequisites, [])

    def test_crate_without_dispatch_in_closure_selects_engine_only(self) -> None:
        selection = self.select("engine/build-helpers/fst-builder/src/main.rs")

        self.assertEqual(selection.engine_crates, ["fst-builder"])
        self.assertEqual(set(selection.commands), {"engine", "docs-checks"})

    def test_tests_only_change_selects_that_crate_alone(self) -> None:
        selection = self.select("engine/phonetics/tests/op_coverage.rs")

        self.assertEqual(selection.engine_crates, ["phonetics"])
        self.assertNotIn("desktop", selection.commands)

    def test_dispatch_tests_only_change_skips_shells_and_mobile(self) -> None:
        selection = self.select("engine/dispatch/tests/common/mod.rs")

        self.assertEqual(selection.engine_crates, ["dispatch"])
        self.assertEqual(set(selection.commands), {"engine", "docs-checks"})

    def test_protos_change_selects_every_crate_and_mobile(self) -> None:
        selection = self.select("engine/protos/proto/envelope.proto")

        self.assertEqual(selection.engine_crates, list(self.graph.crates))
        for platform in ("desktop", "windows", "linux", "ios", "android", "macos"):
            self.assertIn(platform, selection.commands)
        self.assertEqual(selection.prerequisites, [test_select.MAKE_BUILD_PREREQUISITE])
        self.assertTrue(any("every engine crate" in note for note in selection.notes))

    def test_lockfile_selects_every_crate_without_mobile(self) -> None:
        selection = self.select("engine/Cargo.lock")

        self.assertEqual(selection.engine_crates, list(self.graph.crates))
        self.assertNotIn("ios", selection.commands)

    def test_wire_surface_crate_selects_mobile(self) -> None:
        selection = self.select("engine/swift-ffi/src/lib.rs")

        self.assertEqual(selection.engine_crates, ["swift-ffi"])
        self.assertEqual(
            set(selection.commands),
            {"engine", "macos-rust", "macos", "ios", "android", "docs-checks"},
        )

    def test_engine_build_script_selects_mobile_only(self) -> None:
        selection = self.select("engine/scripts/build-xcframework.sh")

        self.assertEqual(set(selection.commands), {"macos", "ios", "android"})
        self.assertEqual(selection.prerequisites, [test_select.MAKE_BUILD_PREREQUISITE])


class PlatformSelectionTests(SelectorTestCase):
    def test_ios_swift_change(self) -> None:
        selection = self.select("ios/Sources/TaigiKeyboard/Keyboard/A.swift")

        self.assertEqual(
            self.runs(selection, "ios"),
            ["swiftformat --lint ios", test_select.IOS_TEST],
        )
        self.assertEqual(set(selection.commands), {"ios", "docs-checks"})

    def test_android_change(self) -> None:
        selection = self.select("android/app/src/main/java/A.kt")

        self.assertEqual(
            self.runs(selection, "android"),
            ["android/gradlew -p android :app:spotlessCheck :app:testDebugUnitTest"],
        )

    def test_desktop_change_selects_every_desktop_shell(self) -> None:
        selection = self.select("desktop/crates/taigi-desktop-core/src/lib.rs")

        self.assertEqual(
            set(selection.commands),
            {"desktop", "windows", "linux", "web", "macos-rust", "macos", "docs-checks"},
        )
        self.assertEqual(self.runs(selection, "desktop"), ["make desktop-check"])
        # The macOS archive links desktop/: the Swift suite needs a rebuild.
        self.assertEqual(selection.prerequisites, [test_select.MAKE_BUILD_PREREQUISITE])

    def test_macos_rust_change_selects_both_macos_gates(self) -> None:
        for path in (
            "macos/crates/taigi-macos-ffi/src/lib.rs",
            "macos/crates/taigi-macos-ffi/proto/desktop_shell.proto",
            "macos/Cargo.toml",
            "macos/Cargo.lock",
            "macos/rust-toolchain.toml",
        ):
            with self.subTest(path=path):
                selection = self.select(path)

                # The Rust gate runs first: it needs no rebuilt archive.
                self.assertEqual(
                    [p for p in selection.commands if p != "docs-checks"],
                    ["macos-rust", "macos"],
                )
                self.assertEqual(
                    self.runs(selection, "macos-rust"), ["make macos-rust-check"]
                )
                self.assertEqual(self.runs(selection, "macos"), ["make -C macos test"])
                self.assertEqual(
                    selection.prerequisites, [test_select.MAKE_BUILD_PREREQUISITE]
                )

    def test_macos_swift_change_skips_the_rust_gate(self) -> None:
        selection = self.select("macos/Sources/TaigiInputMethodCore/A.swift")

        self.assertNotIn("macos-rust", selection.commands)
        self.assertEqual(
            self.runs(selection, "macos"),
            ["swiftformat --lint macos", "make -C macos test"],
        )
        self.assertEqual(selection.prerequisites, [])

    def test_i18n_source_selects_check_and_mobile_macos(self) -> None:
        selection = self.select("i18n/settings.json")

        self.assertEqual(set(selection.commands), {"i18n", "ios", "android", "macos", "web"})
        self.assertIn("python3 tools/i18n/check.py", self.runs(selection, "i18n"))

    def test_dictionaries_select_data_crates_and_desktop(self) -> None:
        selection = self.select("assets/dictionaries/dictionary.bin")

        self.assertEqual(selection.engine_crates, ["composing", "lexicon", "dispatch"])
        self.assertEqual(set(selection.commands), {"engine", "desktop", "web"})

    def test_web_paths_select_the_web_gate(self) -> None:
        for path in ("web/Makefile", "web/tests/smoke.test.mjs", "mise.toml"):
            selection = self.select(path)

            self.assertEqual(set(selection.commands), {"web"}, path)
            self.assertEqual(
                self.runs(selection, "web"), ["make web-check", "make web-e2e"]
            )

    def test_engine_crate_web_links_selects_the_web_gate(self) -> None:
        selection = self.select("engine/mmap-host/src/registry.rs")

        self.assertIn("web", selection.commands)

    def test_dictionary_output_selects_composing_and_lexicon(self) -> None:
        selection = self.select("dictionary/output/dictionary.csv")

        self.assertEqual(selection.engine_crates, ["composing", "lexicon"])

    def test_dictionary_python_runs_pipeline_tests_from_dictionary(self) -> None:
        selection = self.select("dictionary/build/merge_csv.py")

        self.assertEqual(
            selection.commands["python"],
            [Command("python3 -m pytest tests -q", "dictionary")],
        )

    def test_tools_python_and_converter(self) -> None:
        selection = self.select("tools/release_notes.py", "taigi-converter")

        self.assertEqual(set(selection.commands), {"python", "converter"})
        self.assertEqual(len(selection.commands["python"]), 2)

    def test_tools_shell_scripts_have_no_gate(self) -> None:
        selection = self.select(
            "tools/release/stage-desktop.sh", "tools/secret-scan/gitleaks-scan.sh"
        )

        self.assertEqual(selection.commands, {})
        self.assertEqual(selection.unmapped, [])

    def test_kautian_watch_selects_pytest_suite(self) -> None:
        selection = self.select("tools/kautian_watch.py", "tools/kautian_watch_test.py")
        self.assertEqual(
            selection.commands["python"],
            [Command("python3 -m pytest tools/kautian_watch_test.py -q")],
        )

    def test_e2e_analyzer_runs_its_tests_in_place(self) -> None:
        selection = self.select("e2e/analyzer/analyze.py")

        self.assertEqual(
            selection.commands["python"],
            [Command("python3 -m unittest analyze_test", "e2e/analyzer")],
        )

    def test_docs_only_is_admin_lane_plus_doc_gates(self) -> None:
        selection = self.select(
            "docs/architecture/behavioral-invariants.md", "README.md"
        )

        self.assertEqual(
            selection.commands,
            {
                "docs-checks": [
                    Command("python3 tools/invariant_labels.py"),
                    DOC_PATHS_CHECK,
                ]
            },
        )
        self.assertIn(test_select.ADMIN_LANE_NOTE, selection.notes)

    def test_rule_pointers_and_contributing_guides_run_the_path_gates(self) -> None:
        self.assertEqual(
            self.select(".claude/rules/i18n.md").commands,
            {
                "docs-checks": [
                    Command("python3 tools/agent_rule_paths.py"),
                    DOC_PATHS_CHECK,
                ]
            },
        )
        self.assertEqual(
            self.select("docs/contributing/README.md").commands,
            {
                "docs-checks": [
                    Command("python3 tools/invariant_labels.py"),
                    Command("python3 tools/agent_rule_paths.py"),
                    DOC_PATHS_CHECK,
                ]
            },
        )

    def test_frozen_snapshot_skips_the_doc_paths_gate(self) -> None:
        self.assertEqual(
            self.select("docs/reports/2026-11-13-audit-all.md").commands,
            {"docs-checks": [Command("python3 tools/invariant_labels.py")]},
        )

    def test_changelog_plus_claude_md_runs_only_doc_paths(self) -> None:
        selection = self.select("changelog/3.6.10.md", "CLAUDE.md")

        self.assertEqual(
            selection.commands,
            {"docs-checks": [DOC_PATHS_CHECK]},
        )
        self.assertIn(test_select.ADMIN_LANE_NOTE, selection.notes)

    def test_unknown_top_level_dir_is_unmapped(self) -> None:
        selection = self.select("newdir/thing.txt")

        self.assertEqual(selection.unmapped, ["newdir/thing.txt"])


class DefaultBaseTests(unittest.TestCase):
    def _git(self, repo: Path, *args: str) -> str:
        return subprocess.run(
            [
                "git",
                "-C",
                str(repo),
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                *args,
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

    def _init_repo(self, repo: Path) -> str:
        self._git(repo, "init", "-b", "main")
        self._git(repo, "commit", "--allow-empty", "-m", "root")
        return self._git(repo, "rev-parse", "HEAD")

    def _commit(self, repo: Path, message: str) -> str:
        self._git(repo, "commit", "--allow-empty", "-m", message)
        return self._git(repo, "rev-parse", "HEAD")

    def test_plain_git_repo_uses_origin_main(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            origin_main = self._init_repo(repo)
            self._git(repo, "update-ref", "refs/remotes/origin/main", origin_main)
            self._commit(repo, "branch")

            self.assertEqual(test_select.default_base(repo), origin_main)

    def test_fork_prefers_upstream_main_over_stale_origin_main(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            stale_origin = self._init_repo(repo)
            upstream_main = self._commit(repo, "upstream")
            self._git(repo, "update-ref", "refs/remotes/origin/main", stale_origin)
            self._git(repo, "update-ref", "refs/remotes/upstream/main", upstream_main)
            self._commit(repo, "branch")

            self.assertEqual(test_select.default_base(repo), upstream_main)

    def test_upstream_remote_without_upstream_main_falls_back_to_origin_main(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            origin_main = self._init_repo(repo)
            self._git(
                repo, "remote", "add", "upstream", "https://example.com/upstream.git"
            )
            self._git(repo, "update-ref", "refs/remotes/origin/main", origin_main)
            self._commit(repo, "branch")

            self.assertEqual(test_select.default_base(repo), origin_main)

    def test_jj_repo_uses_jj_trunk_over_older_git_refs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / ".jj").mkdir()
            stale_origin = self._init_repo(repo)
            older_upstream = self._commit(repo, "upstream")
            jj_trunk = self._commit(repo, "trunk")
            self._git(repo, "update-ref", "refs/remotes/origin/main", stale_origin)
            self._git(repo, "update-ref", "refs/remotes/upstream/main", older_upstream)
            self._commit(repo, "branch")

            real_run = subprocess.run

            def fake_run(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    self.assertEqual(
                        cmd,
                        [
                            "jj",
                            "--ignore-working-copy",
                            "-R",
                            str(repo),
                            "log",
                            "-r",
                            "trunk()",
                            "--no-graph",
                            "-T",
                            r'commit_id ++ "\n"',
                        ],
                    )
                    return subprocess.CompletedProcess(
                        cmd, 0, stdout=f"{jj_trunk}\n", stderr=""
                    )
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(test_select.subprocess, "run", side_effect=fake_run):
                self.assertEqual(test_select.default_base(repo), jj_trunk)

    def test_jj_fallback_when_jj_fails_or_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / ".jj").mkdir()
            stale_origin = self._init_repo(repo)
            upstream_main = self._commit(repo, "upstream")
            self._git(repo, "update-ref", "refs/remotes/origin/main", stale_origin)
            self._git(repo, "update-ref", "refs/remotes/upstream/main", upstream_main)
            self._commit(repo, "branch")

            real_run = subprocess.run

            def fail_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    return subprocess.CompletedProcess(
                        cmd, 1, stdout="", stderr="error"
                    )
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(test_select.subprocess, "run", side_effect=fail_jj):
                self.assertEqual(test_select.default_base(repo), upstream_main)

            def missing_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    raise FileNotFoundError("jj")
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(
                test_select.subprocess, "run", side_effect=missing_jj
            ):
                self.assertEqual(test_select.default_base(repo), upstream_main)

    def test_repo_without_jj_dir_does_not_invoke_jj(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            origin_main = self._init_repo(repo)
            self._git(repo, "update-ref", "refs/remotes/origin/main", origin_main)
            self._commit(repo, "branch")

            real_run = subprocess.run
            calls = []

            def spy_run(cmd, *args, **kwargs):
                calls.append(cmd[0])
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(test_select.subprocess, "run", side_effect=spy_run):
                self.assertEqual(test_select.default_base(repo), origin_main)

            self.assertNotIn("jj", calls)


def _working_tree_paths(root: Path) -> list[str]:
    listed = subprocess.run(
        [
            "git",
            "-C",
            str(root),
            "-c",
            "core.quotepath=false",
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return sorted(
        {
            path
            for path in listed.split("\0")
            if path and ((root / path).exists() or (root / path).is_symlink())
        }
    )


class CoverageGuardTests(SelectorTestCase):
    def test_every_tracked_file_has_a_rule(self) -> None:
        paths = _working_tree_paths(ROOT)

        effects = test_select.effects_of(paths, self.graph)

        self.assertEqual(
            effects.unmapped,
            [],
            "give each new top-level directory a rule in tools/test_select.py "
            "(effects_of, or NO_GATE_DIRS when it needs no gate)",
        )

    def test_working_tree_paths_tracks_new_files_and_skips_deleted_or_ignored(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            subprocess.run(["git", "init", "-q", tmp], check=True)
            (repo / ".gitignore").write_text("ignored_dir/\n*.log\n", encoding="utf-8")
            (repo / ".jj").mkdir()
            (repo / ".jj" / ".gitignore").write_text("/*\n", encoding="utf-8")
            (repo / ".jj" / "state.txt").write_text("jj\n", encoding="utf-8")
            (repo / "kept.txt").write_text("kept\n", encoding="utf-8")
            (repo / "deleted.txt").write_text("gone\n", encoding="utf-8")
            subprocess.run(
                ["git", "-C", tmp, "add", ".gitignore", "kept.txt", "deleted.txt"],
                check=True,
            )
            subprocess.run(
                [
                    "git",
                    "-C",
                    tmp,
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "-q",
                    "-m",
                    "init",
                ],
                check=True,
            )

            (repo / "deleted.txt").unlink()
            (repo / "newdir").mkdir()
            (repo / "newdir" / "thing.txt").write_text("new\n", encoding="utf-8")
            (repo / "ignored_dir").mkdir()
            (repo / "ignored_dir" / "skip.txt").write_text("ignored\n", encoding="utf-8")
            (repo / "skip.log").write_text("ignored\n", encoding="utf-8")

            self.assertEqual(
                _working_tree_paths(repo),
                [".gitignore", "kept.txt", "newdir/thing.txt"],
            )


if __name__ == "__main__":
    unittest.main()
