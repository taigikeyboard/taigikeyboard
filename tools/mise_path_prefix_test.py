#!/usr/bin/env python3
"""Tests for tools/mise-path-prefix.sh, driven with a fake mise on PATH."""

from __future__ import annotations

import os
import stat
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "tools/mise-path-prefix.sh"


class MisePathPrefixTests(unittest.TestCase):
    def setUp(self) -> None:
        home = tempfile.TemporaryDirectory()
        self.addCleanup(home.cleanup)
        self.home = Path(home.name)
        self.bin = self.home / "bin"
        self.bin.mkdir()

    def fake_mise(self, directory: Path, body: str) -> None:
        directory.mkdir(parents=True, exist_ok=True)
        mise = directory / "mise"
        mise.write_text(f"#!/bin/sh\n{body}\n")
        mise.chmod(mise.stat().st_mode | stat.S_IEXEC)

    def run_script(self) -> subprocess.CompletedProcess[str]:
        env = {"HOME": str(self.home), "PATH": f"{self.bin}{os.pathsep}/usr/bin:/bin"}
        return subprocess.run(
            ["sh", str(SCRIPT)], env=env, capture_output=True, text=True, check=False
        )

    def test_without_mise_prints_nothing(self) -> None:
        finished = self.run_script()
        self.assertEqual(
            (finished.returncode, finished.stdout, finished.stderr), (0, "", "")
        )

    def test_names_exactly_the_tools_in_mise_toml(self) -> None:
        # An unscoped call would bring in the contributor's global tools.
        self.fake_mise(self.bin, 'echo "$*"')
        pinned = tomllib.loads((ROOT / "mise.toml").read_text())["tools"]
        self.assertEqual(self.run_script().stdout, f"bin-paths {' '.join(pinned)}:")

    def test_each_directory_is_followed_by_a_colon(self) -> None:
        self.fake_mise(self.bin, "echo /pins/a; echo /pins/b")
        self.assertEqual(self.run_script().stdout, "/pins/a:/pins/b:")

    def test_finds_mise_in_its_installer_directory(self) -> None:
        self.fake_mise(self.home / ".local/bin", "echo /pins/a")
        self.assertEqual(self.run_script().stdout, "/pins/a:")

    def test_no_installed_tool_never_prints_a_lone_colon(self) -> None:
        self.fake_mise(self.bin, "exit 0")
        self.assertEqual(self.run_script().stdout, "")

    def test_mise_failure_warns_and_prints_nothing(self) -> None:
        self.fake_mise(self.bin, "echo /partial; exit 1")
        finished = self.run_script()
        self.assertEqual((finished.returncode, finished.stdout), (0, ""))
        self.assertIn("mise trust", finished.stderr)


if __name__ == "__main__":
    unittest.main()
