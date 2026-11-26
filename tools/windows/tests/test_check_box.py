"""Tests for windows/scripts/check-box.sh commit resolution under Git and Jujutsu."""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

CHECK_BOX = Path(__file__).resolve().parents[3] / "windows" / "scripts" / "check-box.sh"


class CheckBoxTests(unittest.TestCase):
    def temporary_directory(self) -> Path:
        holder = tempfile.TemporaryDirectory()
        self.addCleanup(holder.cleanup)
        return Path(holder.name)

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

    def _init_git_repo(self, repo: Path) -> str:
        self._git(repo, "init", "-q", "-b", "main")
        (repo / ".gitignore").write_text("*.ignored\n", encoding="utf-8")
        (repo / "tracked.txt").write_text("initial\n", encoding="utf-8")
        self._git(repo, "add", ".gitignore", "tracked.txt")
        self._git(repo, "commit", "-q", "-m", "initial")
        return self._git(repo, "rev-parse", "HEAD")

    def _run_print_sha(
        self, repo: Path, env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(CHECK_BOX), "--print-sha", str(repo)],
            capture_output=True,
            text=True,
            env=env,
        )

    def _write_fake_jj(
        self,
        bin_dir: Path,
        *,
        at_line: str,
        parent_line: str = "",
    ) -> None:
        script = bin_dir / "jj"
        script.write_text(
            f"""#!/usr/bin/env python3
import sys

args = sys.argv[1:]
if "--no-pager" not in args or "--color=never" not in args:
    print(f"missing --no-pager or --color=never in jj args: {{args}}", file=sys.stderr)
    sys.exit(2)
if "-r" in args:
    rev = args[args.index("-r") + 1]
    if rev == "@":
        print({at_line!r})
        sys.exit(0)
    if rev == "@-":
        if {parent_line!r}:
            print({parent_line!r})
            sys.exit(0)
        print("missing @-", file=sys.stderr)
        sys.exit(1)
print(f"unexpected jj args: {{args}}", file=sys.stderr)
sys.exit(2)
""",
            encoding="utf-8",
        )
        script.chmod(script.stat().st_mode | stat.S_IEXEC)

    def _env_with_bin(self, bin_dir: Path) -> dict[str, str]:
        env = dict(os.environ)
        env["PATH"] = f"{bin_dir}:{env.get('PATH', '')}"
        return env

    def _env_without_jj(self) -> dict[str, str]:
        bin_dir = self.temporary_directory() / "no-jj-bin"
        bin_dir.mkdir()
        for tool in ("bash", "dirname", "git"):
            resolved = shutil.which(tool)
            self.assertIsNotNone(resolved, f"{tool} must exist on PATH")
            (bin_dir / tool).symlink_to(resolved)
        env = dict(os.environ)
        env["PATH"] = str(bin_dir)
        return env

    def _env_with_jj_config(self, toml_text: str = "") -> dict[str, str]:
        cfg = self.temporary_directory() / "jj-config.toml"
        cfg.write_text(toml_text, encoding="utf-8")
        env = dict(os.environ)
        env["JJ_CONFIG"] = str(cfg)
        return env

    def test_plain_git_clean_returns_head_and_ignores_ignored_and_untracked_files(
        self,
    ) -> None:
        repo = self.temporary_directory()
        head = self._init_git_repo(repo)
        (repo / "scratch.ignored").write_text("ignored\n", encoding="utf-8")
        (repo / "notes.txt").write_text("untracked\n", encoding="utf-8")

        result = self._run_print_sha(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), head)
        self.assertEqual(result.stderr, "")

    def test_plain_git_dirty_warns_and_still_returns_head(self) -> None:
        repo = self.temporary_directory()
        head = self._init_git_repo(repo)

        (repo / "tracked.txt").write_text("modified\n", encoding="utf-8")
        res_mod = self._run_print_sha(repo)
        self.assertEqual(res_mod.returncode, 0, res_mod.stderr)
        self.assertEqual(res_mod.stdout.strip(), head)
        self.assertIn("warning — the working tree is dirty", res_mod.stderr)

        self._git(repo, "add", "tracked.txt")
        res_staged = self._run_print_sha(repo)
        self.assertEqual(res_staged.returncode, 0, res_staged.stderr)
        self.assertEqual(res_staged.stdout.strip(), head)
        self.assertIn("warning — the working tree is dirty", res_staged.stderr)

    def test_jj_stub_non_empty_at_resolves_to_at_even_with_git_diff(self) -> None:
        repo = self.temporary_directory()
        self._init_git_repo(repo)
        (repo / ".jj").mkdir()
        (repo / "tracked.txt").write_text("edited in @\n", encoding="utf-8")
        bin_dir = self.temporary_directory() / "bin"
        bin_dir.mkdir()
        at_sha = "a" * 40
        parent_sha = "b" * 40
        self._write_fake_jj(
            bin_dir,
            at_line=f"{at_sha} false 1 0 false",
            parent_line=f"{parent_sha} false",
        )

        result = self._run_print_sha(repo, env=self._env_with_bin(bin_dir))

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), at_sha)

    def test_jj_stub_empty_unbookmarked_single_parent_resolves_to_parent(self) -> None:
        repo = self.temporary_directory()
        self._init_git_repo(repo)
        (repo / ".jj").mkdir()
        bin_dir = self.temporary_directory() / "bin"
        bin_dir.mkdir()
        at_sha = "a" * 40
        parent_sha = "b" * 40
        self._write_fake_jj(
            bin_dir,
            at_line=f"{at_sha} true 1 0 false",
            parent_line=f"{parent_sha} false",
        )

        result = self._run_print_sha(repo, env=self._env_with_bin(bin_dir))

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), parent_sha)

    def test_jj_stub_empty_bookmarked_or_merge_at_resolves_to_at(self) -> None:
        repo = self.temporary_directory()
        self._init_git_repo(repo)
        (repo / ".jj").mkdir()
        bin_dir = self.temporary_directory() / "bin"
        bin_dir.mkdir()
        at_sha = "a" * 40
        parent_sha = "b" * 40

        for at_line in (
            f"{at_sha} true 1 1 false",
            f"{at_sha} true 2 0 false",
        ):
            with self.subTest(at_line=at_line):
                self._write_fake_jj(
                    bin_dir,
                    at_line=at_line,
                    parent_line=f"{parent_sha} false",
                )
                result = self._run_print_sha(repo, env=self._env_with_bin(bin_dir))
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), at_sha)

    def test_jj_stub_rejects_conflicted_at_and_conflicted_parent(self) -> None:
        repo = self.temporary_directory()
        self._init_git_repo(repo)
        (repo / ".jj").mkdir()
        bin_dir = self.temporary_directory() / "bin"
        bin_dir.mkdir()
        at_sha = "a123456" + "0" * 33
        parent_sha = "b123456" + "0" * 33

        self._write_fake_jj(
            bin_dir,
            at_line=f"{at_sha} false 1 0 true",
            parent_line=f"{parent_sha} false",
        )
        res_at = self._run_print_sha(repo, env=self._env_with_bin(bin_dir))
        self.assertNotEqual(res_at.returncode, 0)
        self.assertIn("unresolved jj conflicts", res_at.stderr)

        self._write_fake_jj(
            bin_dir,
            at_line=f"{at_sha} true 1 0 false",
            parent_line=f"{parent_sha} true",
        )
        res_parent = self._run_print_sha(repo, env=self._env_with_bin(bin_dir))
        self.assertNotEqual(res_parent.returncode, 0)
        self.assertIn("unresolved jj conflicts", res_parent.stderr)

    def test_jj_dir_without_jj_binary_falls_back_to_plain_git_behavior(self) -> None:
        repo = self.temporary_directory()
        head = self._init_git_repo(repo)
        (repo / ".jj").mkdir()
        env = self._env_without_jj()

        clean = self._run_print_sha(repo, env=env)
        self.assertEqual(clean.returncode, 0, clean.stderr)
        self.assertEqual(clean.stdout.strip(), head)
        self.assertEqual(clean.stderr, "")

        (repo / "tracked.txt").write_text("dirty\n", encoding="utf-8")
        dirty = self._run_print_sha(repo, env=env)
        self.assertEqual(dirty.returncode, 0, dirty.stderr)
        self.assertEqual(dirty.stdout.strip(), head)
        self.assertIn("warning — the working tree is dirty", dirty.stderr)

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_colocated_jj_workspace_resolution(self) -> None:
        repo = self.temporary_directory()
        env = self._env_with_jj_config("")
        subprocess.run(
            [
                "jj",
                "git",
                "init",
                "--colocate",
                "--config",
                "user.name=Test",
                "--config",
                "user.email=test@example.com",
                str(repo),
            ],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )
        (repo / "tracked.txt").write_text("v1\n", encoding="utf-8")
        subprocess.run(
            [
                "jj",
                "-R",
                str(repo),
                "--config",
                "user.name=Test",
                "--config",
                "user.email=test@example.com",
                "commit",
                "-m",
                "base",
            ],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )

        def jj_id(rev: str) -> str:
            return subprocess.run(
                [
                    "jj",
                    "--no-pager",
                    "--color=never",
                    "-R",
                    str(repo),
                    "log",
                    "-r",
                    rev,
                    "--no-graph",
                    "-T",
                    'commit_id ++ "\\n"',
                ],
                check=True,
                capture_output=True,
                text=True,
                env=env,
            ).stdout.strip()

        # Clean empty @ resolves to @- (== git HEAD).
        res_empty = self._run_print_sha(repo, env=env)
        self.assertEqual(res_empty.returncode, 0, res_empty.stderr)
        self.assertEqual(res_empty.stdout.strip(), jj_id("@-"))
        self.assertEqual(res_empty.stdout.strip(), self._git(repo, "rev-parse", "HEAD"))

        # Editing a tracked file in @ resolves to @'s commit_id (different from git HEAD).
        (repo / "tracked.txt").write_text("v2\n", encoding="utf-8")
        res_edit = self._run_print_sha(repo, env=env)
        self.assertEqual(res_edit.returncode, 0, res_edit.stderr)
        edit_sha = res_edit.stdout.strip()
        self.assertEqual(edit_sha, jj_id("@"))
        self.assertNotEqual(edit_sha, self._git(repo, "rev-parse", "HEAD"))

        # Adding a brand-new untracked file in @ snapshots and updates @'s commit_id.
        (repo / "added.txt").write_text("new\n", encoding="utf-8")
        res_added = self._run_print_sha(repo, env=env)
        self.assertEqual(res_added.returncode, 0, res_added.stderr)
        added_sha = res_added.stdout.strip()
        self.assertEqual(added_sha, jj_id("@"))
        self.assertNotEqual(added_sha, edit_sha)
        self.assertNotEqual(added_sha, self._git(repo, "rev-parse", "HEAD"))

        # After `jj new`, clean empty @ resolves to @- (which now equals git HEAD).
        subprocess.run(
            [
                "jj",
                "-R",
                str(repo),
                "--config",
                "user.name=Test",
                "--config",
                "user.email=test@example.com",
                "new",
            ],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )
        res_after_new = self._run_print_sha(repo, env=env)
        self.assertEqual(res_after_new.returncode, 0, res_after_new.stderr)
        self.assertEqual(res_after_new.stdout.strip(), added_sha)
        self.assertEqual(res_after_new.stdout.strip(), jj_id("@-"))
        self.assertEqual(
            res_after_new.stdout.strip(), self._git(repo, "rev-parse", "HEAD")
        )

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_colocated_jj_ignores_user_color_and_pager_config(self) -> None:
        repo = self.temporary_directory()
        env = self._env_with_jj_config(
            '[user]\nname = "Test"\nemail = "test@example.com"\n'
            '[ui]\ncolor = "always"\npaginate = "auto"\n'
        )
        subprocess.run(
            ["jj", "git", "init", "--colocate", str(repo)],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )
        (repo / "tracked.txt").write_text("v1\n", encoding="utf-8")
        subprocess.run(
            ["jj", "-R", str(repo), "commit", "-m", "base"],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )

        def clean_id(rev: str) -> str:
            return subprocess.run(
                [
                    "jj",
                    "--no-pager",
                    "--color=never",
                    "-R",
                    str(repo),
                    "log",
                    "-r",
                    rev,
                    "--no-graph",
                    "-T",
                    'commit_id ++ "\\n"',
                ],
                check=True,
                capture_output=True,
                text=True,
                env=env,
            ).stdout.strip()

        res_empty = self._run_print_sha(repo, env=env)
        self.assertEqual(res_empty.returncode, 0, res_empty.stderr)
        empty_sha = res_empty.stdout.strip()
        self.assertRegex(empty_sha, r"^[0-9a-f]{40}$")
        self.assertEqual(empty_sha, clean_id("@-"))

        (repo / "tracked.txt").write_text("v2\n", encoding="utf-8")
        res_edit = self._run_print_sha(repo, env=env)
        self.assertEqual(res_edit.returncode, 0, res_edit.stderr)
        edit_sha = res_edit.stdout.strip()
        self.assertRegex(edit_sha, r"^[0-9a-f]{40}$")
        self.assertEqual(edit_sha, clean_id("@"))

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_colocated_jj_respects_auto_track_none_for_untracked_files(self) -> None:
        repo = self.temporary_directory()
        cfg = self.temporary_directory() / "jj-config.toml"
        cfg.write_text(
            '[user]\nname = "Test"\nemail = "test@example.com"\n',
            encoding="utf-8",
        )
        env = dict(os.environ)
        env["JJ_CONFIG"] = str(cfg)
        subprocess.run(
            ["jj", "git", "init", "--colocate", str(repo)],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )
        (repo / "tracked.txt").write_text("v1\n", encoding="utf-8")
        subprocess.run(
            ["jj", "-R", str(repo), "commit", "-m", "base"],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )

        cfg.write_text(
            '[user]\nname = "Test"\nemail = "test@example.com"\n'
            '[snapshot]\nauto-track = "none()"\n',
            encoding="utf-8",
        )
        (repo / "notes.txt").write_text("untracked\n", encoding="utf-8")
        res = self._run_print_sha(repo, env=env)
        self.assertEqual(res.returncode, 0, res.stderr)
        self.assertEqual(res.stdout.strip(), self._git(repo, "rev-parse", "HEAD"))


if __name__ == "__main__":
    unittest.main()
