"""The desktop driver's keyboard: one uinput device that types into whatever
has focus in the logged-in session, the way a USB keyboard would — the
compositor (Wayland or X11) sees a real input device.

Runs as root (`sudo -n python3 uinput.py`; /dev/uinput is root-only). Reads
one command per line on stdin and answers `ok` once the keys are sent:

  text <characters>   US-layout characters, Shift added for the upper row
  key <name>          enter / space / backspace / escape / tab / capslock / 0–9
"""

from __future__ import annotations

import sys
import time

from evdev import UInput, ecodes

# The compositor needs a moment to adopt a new device before its first key.
DEVICE_SETTLE_S = 1.0
KEY_GAP_S = 0.04
NAMED_KEYS = {"enter": "ENTER", "space": "SPACE", "backspace": "BACKSPACE", "escape": "ESC", "tab": "TAB"}
PUNCTUATION = {
    "-": "MINUS", "=": "EQUAL", "[": "LEFTBRACE", "]": "RIGHTBRACE", ";": "SEMICOLON", "'": "APOSTROPHE",
    "`": "GRAVE", "\\": "BACKSLASH", ",": "COMMA", ".": "DOT", "/": "SLASH", " ": "SPACE",
}
SHIFTED = dict(zip('_+{}:"~|<>?!@#$%^&*()', "-=[];'`\\,./1234567890"))


def key_for(character: str) -> tuple[int, bool]:
    """(key code, needs Shift) for one US-layout character."""
    shift = character.isupper() or character in SHIFTED
    base = SHIFTED.get(character, character.lower())
    name = PUNCTUATION.get(base, base.upper())
    return getattr(ecodes, f"KEY_{name}"), shift


def tap(device: UInput, code: int, shift: bool = False) -> None:
    if shift:
        device.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 1)
        device.syn()
    device.write(ecodes.EV_KEY, code, 1)
    device.syn()
    device.write(ecodes.EV_KEY, code, 0)
    device.syn()
    if shift:
        device.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 0)
        device.syn()
    time.sleep(KEY_GAP_S)


def main() -> int:
    with UInput(name="taigi-e2e-keyboard") as device:
        time.sleep(DEVICE_SETTLE_S)
        for line in sys.stdin:
            command, _, argument = line.rstrip("\n").partition(" ")
            if command == "text":
                for character in argument:
                    tap(device, *key_for(character))
            elif command == "key":
                name = argument.lower()
                tap(device, getattr(ecodes, f"KEY_{NAMED_KEYS.get(name, name.upper())}"))
            else:
                print(f"error unknown command {command!r}", flush=True)
                continue
            print("ok", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
