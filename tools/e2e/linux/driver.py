"""Linux e2e driver: types every scenario into a real GTK text field through
the installed test-mode IME (Fcitx5 or IBus) and records what happened.

Runs on a Linux host with the test-mode build installed into a prefix of
its own (`make -C linux install E2E=1 PREFIX=<prefix>`; the environment
below points both frameworks at it, so a system install is never touched),
inside an X display and a D-Bus session:

  xvfb-run -a dbus-run-session -- python3 tools/e2e/linux/driver.py \\
      --framework fcitx5 --prefix <prefix> --out <run-dir>

Writes <run-dir>/linux-<framework>/<scenario>/{result.json, trace.jsonl,
framework.log} — the run layout of docs/architecture/e2e-trace-schema.md.
Every key goes through xdotool → X server → the framework → the IME; no
step ever sets text directly.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import analyze

HOST_SCRIPT = Path(__file__).with_name("host.py")
TRACE_RELATIVE_PATH = Path("taigikeyboard") / "e2e-trace.jsonl"
INPUT_METHOD_NAME = "taigikeyboard"
# X keysym names for the scenario's platform-neutral key names.
KEYSYMS = {"enter": "Return", "space": "space", "backspace": "BackSpace", "escape": "Escape", "capslock": "Caps_Lock"}
# IBus / Fcitx5 release bit (`linux/fcitx5/src/engine.cpp` statesFor); a
# release is traced as a `key` event too, and only presses are counted.
RELEASE_MASK = 1 << 30
# The first consumed key loads the lexicon synchronously (runtime.rs
# `prepare_for_first_key`), seconds on a slow VM.
KEYS_SETTLE_TIMEOUT_S = 60
STARTUP_TIMEOUT_S = 30
XDOTOOL_TYPE_DELAY_MS = "40"


class ScenarioError(Exception):
    """The scenario could not be driven to its end; recorded as `error`."""


class UnsupportedScenario(Exception):
    """The scenario asks for something this platform does not have; recorded as `skipped`."""


def settings_document(settings: dict) -> dict:
    """The desktop `settings.json` for a scenario's intent-level settings
    (desktop/crates/taigi-desktop-core/src/settings/keys.rs)."""
    values: dict = {}
    romanization = settings.get("romanization", "tl")
    if romanization not in ("tl", "poj"):
        raise UnsupportedScenario(f"romanization {romanization!r} has no desktop input mode")
    values["inputMode"] = romanization
    if settings.get("continuous_input") is False:
        raise UnsupportedScenario("desktop continuous input is always on")
    output = settings.get("output")
    if output is not None:
        values["isTranslateSwapped"] = output == "hanji"
    return {"revision": 1, "values": values}


def prefix_environment(prefix: Path) -> dict[str, str]:
    """Point Fcitx5, IBus and the engine at the test-mode install in `prefix`.

    `FCITX_ADDON_DIRS` replaces Fcitx5's addon search path, so the system
    directories (keyboard, xim, the frontends) are listed after ours."""
    addon_dirs = [str(p) for p in sorted(prefix.glob("lib*/**/fcitx5")) if p.is_dir()]
    addon_dirs += [
        str(p)
        for pattern in ("/usr/lib/*/fcitx5", "/usr/lib64/fcitx5", "/usr/lib/fcitx5")
        for p in sorted(Path("/").glob(pattern.lstrip("/")))
        if p.is_dir() and str(p) not in addon_dirs
    ]
    return {
        "FCITX_ADDON_DIRS": ":".join(addon_dirs),
        "XDG_DATA_DIRS": f"{prefix / 'share'}:/usr/local/share:/usr/share",
        "IBUS_COMPONENT_PATH": str(prefix / "share" / "ibus" / "component"),
        "TAIGIKEYBOARD_DATA_DIR": str(prefix / "share" / "taigikeyboard"),
    }


def fcitx5_profile() -> str:
    return (
        "[Groups/0]\nName=Default\nDefault Layout=us\nDefaultIM=taigikeyboard\n"
        "[Groups/0/Items/0]\nName=keyboard-us\nLayout=\n"
        "[Groups/0/Items/1]\nName=taigikeyboard\nLayout=\n"
        "[GroupOrder]\n0=Default\n"
    )


class TraceReader:
    """The live trace, read incrementally: each call parses only the lines
    appended since the last one, and never a half-written last line."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.offset = 0
        self.events: list[dict] = []

    def refresh(self) -> list[dict]:
        if self.path.exists():
            with self.path.open("rb") as trace:
                trace.seek(self.offset)
                chunk = trace.read()
            complete = chunk[: chunk.rfind(b"\n") + 1]
            self.offset += len(complete)
            self.events += [json.loads(line) for line in complete.decode("utf-8").splitlines() if line]
        return self.events

    def pressed_key_count(self) -> int:
        return sum(1 for e in self.refresh() if e.get("event") == "key" and not e.get("state", 0) & RELEASE_MASK)


def wait_until(predicate, timeout_s: float, what: str) -> None:
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise ScenarioError(f"timed out waiting for {what}")


class Session:
    """One scenario: a fresh XDG tree, framework, host window and trace."""

    def __init__(self, framework: str, prefix: Path, scenario: dict, work: Path, env: dict[str, str] | None = None) -> None:
        """`env` replaces the Xvfb session's environment built here."""
        self.framework = framework
        self.work = work
        self.trace = work / "data" / TRACE_RELATIVE_PATH
        self.host_out = work / "host-text.txt"
        self.framework_log = work / "framework.log"
        self.reader = TraceReader(self.trace)
        self.keys_sent = 0
        self.caps_lock_on = False
        self.processes: list[subprocess.Popen] = []
        self.host: subprocess.Popen | None = None
        self.last_ibus_listing = ""
        (work / "config" / "taigikeyboard").mkdir(parents=True)
        (work / "config" / "taigikeyboard" / "settings.json").write_text(
            json.dumps(settings_document(scenario.get("settings", {}))), encoding="utf-8"
        )
        module = "fcitx" if framework == "fcitx5" else "ibus"
        self.env = env or {
            **os.environ,
            **prefix_environment(prefix),
            "HOME": str(work),
            "XDG_CONFIG_HOME": str(work / "config"),
            "XDG_DATA_HOME": str(work / "data"),
            "GTK_IM_MODULE": module,
            "XMODIFIERS": f"@im={module}",
            "NO_AT_BRIDGE": "1",
            "GTK_USE_PORTAL": "0",
            "GDK_BACKEND": "x11",
        }

    def run(self, command: list[str], **kwargs) -> subprocess.CompletedProcess:
        return subprocess.run(command, env=self.env, capture_output=True, text=True, timeout=STARTUP_TIMEOUT_S, check=False, **kwargs)

    def spawn(self, command: list[str], env: dict[str, str] | None = None) -> subprocess.Popen:
        log = self.framework_log.open("a", encoding="utf-8")
        process = subprocess.Popen(command, env=env or self.env, stdout=log, stderr=subprocess.STDOUT)
        self.processes.append(process)
        return process

    def start(self) -> None:
        if self.framework == "fcitx5":
            (self.work / "config" / "fcitx5").mkdir(parents=True)
            (self.work / "config" / "fcitx5" / "profile").write_text(fcitx5_profile(), encoding="utf-8")
            self.spawn(["fcitx5", "--replace"])
            # Only NameHasOwner until our fcitx5 owns the name: any call to
            # the unowned name makes dbus-daemon auto-start a second fcitx5
            # with the daemon's environment, not this scenario's.
            wait_until(self.fcitx5_owns_its_name, STARTUP_TIMEOUT_S, "fcitx5 to own org.fcitx.Fcitx5")
        else:
            self.spawn(["ibus-daemon", "--replace", "--panel", "disable", "--config", "disable"])
            try:
                wait_until(self.ibus_lists_engine, STARTUP_TIMEOUT_S, "ibus-daemon listing the engine")
            except ScenarioError as timeout:
                raise ScenarioError(f"{timeout}; last `ibus list-engine`: {self.last_ibus_listing!r}") from None
        self.host = subprocess.Popen([sys.executable, str(HOST_SCRIPT), str(self.host_out)], env=self.env)
        self.processes.append(self.host)
        search = self.run(["xdotool", "search", "--sync", "--name", "e2ehost"])
        window = search.stdout.split()
        if not window:
            raise ScenarioError("the host window never mapped")
        self.run(["xdotool", "windowfocus", "--sync", window[0]])
        wait_until(self.activate, STARTUP_TIMEOUT_S, "the input method to be active on the focused field")

    def ibus_lists_engine(self) -> bool:
        listing = self.run(["ibus", "list-engine"])
        if INPUT_METHOD_NAME in listing.stdout:
            return True
        # Kept for the timeout's reason: why the daemon never listed it.
        self.last_ibus_listing = (listing.stdout + listing.stderr).strip()[-400:]
        return False

    def fcitx5_owns_its_name(self) -> bool:
        owner = self.run([
            "dbus-send", "--session", "--print-reply", "--dest=org.freedesktop.DBus", "/",
            "org.freedesktop.DBus.NameHasOwner", "string:org.fcitx.Fcitx5",
        ])
        return "boolean true" in owner.stdout

    def activate(self) -> bool:
        """Switch the focused field to this IME and confirm it took: the
        switch succeeds before the field's input context exists, and keys
        typed then bypass the IME."""
        if self.framework == "fcitx5":
            self.run(["fcitx5-remote", "-s", INPUT_METHOD_NAME])
            return self.run(["fcitx5-remote", "-n"]).stdout.strip() == INPUT_METHOD_NAME
        self.run(["ibus", "engine", INPUT_METHOD_NAME])
        return self.run(["ibus", "engine"]).stdout.strip() == INPUT_METHOD_NAME

    def send_text(self, text: str) -> None:
        self.run(["xdotool", "type", "--delay", XDOTOOL_TYPE_DELAY_MS, text])
        self.keys_sent += len(text)

    def send_key(self, name: str) -> None:
        self.run(["xdotool", "key", KEYSYMS.get(name, name)])
        self.keys_sent += 1

    def settle(self) -> None:
        wait_until(
            lambda: self.reader.pressed_key_count() >= self.keys_sent,
            KEYS_SETTLE_TIMEOUT_S,
            f"the IME to answer {self.keys_sent} keys",
        )

    def pick(self, wanted: dict) -> None:
        """Tab to the cell `analyze.matches_cell` accepts, then Enter
        (`confirmHighlighted`, desktop keys/action.rs)."""
        self.settle()
        lists = [e for e in self.reader.refresh() if e.get("event") == "candidates"]
        if not lists:
            raise ScenarioError("no candidate list to pick from")
        items = lists[-1].get("items", [])
        index = next((i for i, c in enumerate(items) if analyze.matches_cell(c, wanted)), None)
        if index is None:
            offered = [(c.get("hanji"), c.get("tl")) for c in items[:6]]
            raise ScenarioError(f"{wanted.get('hanji', '(any hanji)')} {wanted['tl']} not offered; first cells {offered}")
        for _ in range(index):
            self.send_key("Tab")
        self.send_key("enter")

    def checkpoint(self, step_index: int) -> None:
        """After each step; the desktop driver keeps a screenshot here."""

    def finish(self) -> str:
        self.settle()
        self.host.send_signal(signal.SIGUSR1)
        self.host.wait(timeout=STARTUP_TIMEOUT_S)
        return self.host_out.read_text(encoding="utf-8") if self.host_out.exists() else ""

    def stop(self) -> None:
        # The display's lock state outlives the scenario; the next one starts unlocked.
        if self.caps_lock_on:
            with contextlib.suppress(ScenarioError, subprocess.SubprocessError, OSError):
                self.send_key("capslock")
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()


def drive(new_session, scenario: dict, out: Path) -> dict:
    """One scenario through `new_session(work, out)` — a `Session` (or a
    subclass) over a fresh work directory."""
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="taigi-e2e-") as temporary:
        work = Path(temporary)
        try:
            session = new_session(work, out)
        except UnsupportedScenario as reason:
            return {"status": "skipped", "reason": str(reason)}
        try:
            session.start()
            for index, step in enumerate(scenario.get("steps", [])):
                if step["type"] == "text":
                    session.send_text(step["value"])
                elif step["type"] == "key":
                    session.send_key(step["value"])
                    if step["value"] == "capslock":
                        session.caps_lock_on = not session.caps_lock_on
                elif step["type"] == "pick":
                    session.pick(step)
                session.checkpoint(index)
            result = {"status": "ran", "observed_text": session.finish()}
        except (ScenarioError, subprocess.SubprocessError, OSError) as error:
            result = {"status": "error", "reason": str(error)}
        finally:
            session.stop()
            for source, name in ((session.trace, "trace.jsonl"), (session.framework_log, "framework.log")):
                if source.exists():
                    shutil.copyfile(source, out / name)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--framework", choices=("fcitx5", "ibus"), required=True)
    parser.add_argument("--prefix", type=Path, help="where `make install E2E=1 PREFIX=` put the test-mode build")
    parser.add_argument("--out", type=Path, required=True, help="run dir; results go to <out>/linux-<framework>/")
    parser.add_argument("--scenarios", type=Path, default=analyze.REPO_ROOT / "e2e" / "scenarios")
    parser.add_argument("--only", help="run just this scenario id")
    parser.add_argument("--skip", metavar="REASON", help="drive nothing; record every scenario as skipped")
    args = parser.parse_args(argv)
    if not args.skip and args.prefix is None:
        parser.error("--prefix is required unless --skip")

    if args.skip:
        run_scenarios(f"linux-{args.framework}", args.scenarios, args.only, args.out, skip=args.skip)
        return 0
    prefix = args.prefix.resolve()
    run_scenarios(
        f"linux-{args.framework}", args.scenarios, args.only, args.out,
        drive_one=lambda scenario, out: drive(
            lambda work, _out: Session(args.framework, prefix, scenario, work), scenario, out
        ),
    )
    return 0


def run_scenarios(platform: str, scenarios: Path, only: str | None, out_root: Path, drive_one=None, skip: str | None = None) -> None:
    """Write <out_root>/<platform>/<scenario>/result.json for every scenario
    (just `only` when given): `drive_one(scenario, out)`, or `skipped` with
    the `skip` reason."""
    for scenario_id, scenario in analyze.load_scenarios(scenarios).items():
        if only and scenario_id != only:
            continue
        out = out_root / platform / scenario_id
        if skip:
            out.mkdir(parents=True, exist_ok=True)
            result = {"status": "skipped", "reason": skip}
        else:
            result = drive_one(scenario, out)
        (out / "result.json").write_text(json.dumps(result, ensure_ascii=False), encoding="utf-8")
        print(f"{platform} {scenario_id}: {result['status']} {result.get('reason', result.get('observed_text', ''))}")


if __name__ == "__main__":
    sys.exit(main())
