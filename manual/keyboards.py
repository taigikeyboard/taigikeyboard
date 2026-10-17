#!/usr/bin/env python3
"""Draws the keyboard pictures of the manual: one SVG per topic and platform,
written to the directory given as the first argument. render.sh turns them
into keyboards/*.png.

Key defaults come from desktop/crates/taigi-desktop-core/src/keys/
(action.rs, shortcut_actions.rs, intent.rs, slot_key_set.rs,
telex_guide_rows.rs) and policies/full_width.rs. Update this file when one of
them changes.
"""

import sys
from html import escape
from pathlib import Path

UNIT = 62          # key pitch
GAP = 6            # space between keys
ROW = 62           # row pitch
FN_ROW = 36        # pitch of the function row
PAD = 22           # plate padding
MARGIN = 20        # canvas margin around the plate

INK = "#1d1d1f"
DIM = "#a1a1a6"
PLATE = "#f5f5f7"
KEY_STROKE = "#d2d2d7"
FONT = "-apple-system, 'SF Pro Text', 'PingFang TC', 'Helvetica Neue', sans-serif"

# style name -> (fill, stroke, label colour, caption colour)
STYLES = {
    "plain": ("#ffffff", KEY_STROKE, DIM, DIM),
    "blue": ("#e5f1fd", "#8fc0f2", INK, "#0060c0"),
    "blue-solid": ("#0071e3", "#0071e3", "#ffffff", "#ffffff"),
    "orange": ("#fdeee0", "#f0b98a", INK, "#a84600"),
    "orange-solid": ("#c85a00", "#c85a00", "#ffffff", "#ffffff"),
}


def key(key_id, label, width=1.0, sub=None):
    return {"id": key_id, "label": label, "w": width, "sub": sub}


def gap(width):
    return {"gap": width}


def letters(chars):
    return [key(c, c.upper() if c.isalpha() else c) for c in chars]


NUMBER_ROW = letters("`1234567890-=")
QWERTY_ROW = letters("qwertyuiop[]")
HOME_ROW = letters("asdfghjkl;'")
BOTTOM_ROW = letters("zxcvbnm,./")

MAC = {
    "fn_row": [key("esc", "esc", 1.5)] + [key(f"f{n}", f"F{n}") for n in range(1, 13)] + [key("power", "")],
    "rows": [
        NUMBER_ROW + [key("backspace", "delete", 1.5)],
        [key("tab", "tab", 1.5)] + QWERTY_ROW + [key("\\", "\\")],
        [key("caps", "caps lock", 1.75)] + HOME_ROW + [key("enter", "return", 1.75)],
        [key("shift", "⇧", 2.25, "shift")] + BOTTOM_ROW + [key("rshift", "⇧", 2.25, "shift")],
        [
            key("fn", "fn"),
            key("ctrl", "⌃", 1, "control"),
            key("opt", "⌥", 1, "option"),
            key("cmd", "⌘", 1.25, "command"),
            key("space", "", 5),
            key("rcmd", "⌘", 1.25, "command"),
            key("ropt", "⌥", 1, "option"),
            "arrows",
        ],
    ],
    "cluster": None,
}

WINDOWS = {
    "fn_row": [key("esc", "Esc"), gap(1)]
    + [key(f"f{n}", f"F{n}") for n in range(1, 5)] + [gap(0.5)]
    + [key(f"f{n}", f"F{n}") for n in range(5, 9)] + [gap(0.5)]
    + [key(f"f{n}", f"F{n}") for n in range(9, 13)],
    "rows": [
        NUMBER_ROW + [key("backspace", "Backspace", 2)],
        [key("tab", "Tab", 1.5)] + QWERTY_ROW + [key("\\", "\\", 1.5)],
        [key("caps", "Caps Lock", 1.75)] + HOME_ROW + [key("enter", "Enter", 2.25)],
        [key("shift", "Shift", 2.25)] + BOTTOM_ROW + [key("rshift", "Shift", 2.75)],
        [
            key("ctrl", "Ctrl", 1.25),
            key("win", "Win", 1.25),
            key("alt", "Alt", 1.25),
            key("space", "", 6.25),
            key("ralt", "Alt", 1.25),
            key("rwin", "Win", 1.25),
            key("menu", "Menu", 1.25),
            key("rctrl", "Ctrl", 1.25),
        ],
    ],
    # Navigation block right of the main block: (row, column) -> key.
    "cluster": {
        (0, 0): key("ins", "Ins"), (0, 1): key("home", "Home"), (0, 2): key("pgup", "PgUp"),
        (1, 0): key("del", "Del"), (1, 1): key("end", "End"), (1, 2): key("pgdn", "PgDn"),
        (3, 1): key("up", "↑"),
        (4, 0): key("left", "←"), (4, 1): key("down", "↓"), (4, 2): key("right", "→"),
    },
}

# The four typing rows only, for pictures that are the same on every desktop.
LETTERS_ONLY = {
    "fn_row": None,
    "rows": [
        NUMBER_ROW,
        [gap(0.5)] + QWERTY_ROW + [key("\\", "\\")],
        [gap(0.75)] + HOME_ROW,
        [gap(1.25)] + BOTTOM_ROW,
    ],
    "cluster": None,
}

# policies/full_width.rs MAP, by key: (typed bare, written bare, typed with Shift, written with Shift).
PUNCTUATION = {
    "`": ("`", None, "~", "～"),
    "1": ("1", None, "!", "！"), "2": ("2", None, "@", "＠"), "3": ("3", None, "#", "＃"),
    "4": ("4", None, "$", "＄"), "5": ("5", None, "%", "％"), "6": ("6", None, "^", "＾"),
    "7": ("7", None, "&", "＆"), "8": ("8", None, "*", "＊"), "9": ("9", None, "(", "（"),
    "0": ("0", None, ")", "）"), "-": ("-", None, "_", "＿"), "=": ("=", None, "+", "＋"),
    "[": ("[", "「", "{", "『"), "]": ("]", "」", "}", "』"),
    "\\": ("\\", "、", "|", None),
    ";": (";", "；", ":", "："), "'": ("'", "、", '"', None),
    ",": (",", "，", "<", "《"), ".": (".", "。", ">", "》"), "/": ("/", None, "?", "？"),
}


class Picture:
    def __init__(self, layout, marks, legend, punctuation=False):
        self.layout = layout
        self.marks = marks
        self.legend = legend
        self.punctuation = punctuation
        self.parts = []

    def text(self, x, y, content, size, colour, anchor="start", weight=400):
        self.parts.append(
            f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" fill="{colour}" '
            f'text-anchor="{anchor}" font-weight="{weight}">{escape(content)}</text>'
        )

    def draw_key(self, spec, x, y, width, height):
        mark = self.marks.get(spec["id"], {})
        fill, stroke, label_colour, caption_colour = STYLES[mark.get("style", "plain")]
        self.parts.append(
            f'<rect x="{x:.1f}" y="{y:.1f}" width="{width:.1f}" height="{height:.1f}" rx="7" '
            f'fill="{fill}" stroke="{stroke}"/>'
        )
        if self.punctuation and spec["id"] in PUNCTUATION:
            self.draw_punctuation(spec["id"], x, y, width)
            return
        small = height < 40
        if small:
            self.text(x + width / 2, y + height / 2 + 4, spec["label"], 10, label_colour, "middle")
            return
        short = len(spec["label"]) <= 2
        self.text(x + 9, y + 21, spec["label"], 15 if short else 11.5, label_colour, weight=500 if short else 400)
        lines = mark.get("caption", spec["sub"] or "").split("\n")
        colour = caption_colour if "caption" in mark else label_colour
        for index, line in enumerate(reversed(lines)):
            if line:
                self.text(x + 9, y + height - 9 - index * 13, line, 10.5, colour, weight=500 if "caption" in mark else 400)

    def draw_punctuation(self, key_id, x, y, width):
        bare, bare_out, shifted, shifted_out = PUNCTUATION[key_id]
        for typed, written, baseline in ((shifted, shifted_out, y + 22), (bare, bare_out, y + 44)):
            self.text(x + 9, baseline, typed, 13, INK if written else DIM)
            if written:
                self.text(x + width - 8, baseline, written, 15, "#0060c0", "end", 500)

    def draw_arrows(self, x, y):
        """The MacBook arrow block: half-height keys, ↑ above ↓."""
        half = (ROW - GAP) / 2 - 2
        low = y + ROW - GAP - half
        width = UNIT - GAP
        self.draw_key(key("left", "←"), x, low, width, half)
        self.draw_key(key("up", "↑"), x + UNIT, y, width, half)
        self.draw_key(key("down", "↓"), x + UNIT, low, width, half)
        self.draw_key(key("right", "→"), x + 2 * UNIT, low, width, half)

    def draw_row(self, row, x, y, height):
        for spec in row:
            if spec == "arrows":
                self.draw_arrows(x, y)
                x += 3 * UNIT
            elif "gap" in spec:
                x += spec["gap"] * UNIT
            else:
                self.draw_key(spec, x, y, spec["w"] * UNIT - GAP, height)
                x += spec["w"] * UNIT
        return x

    def draw_legend(self, x, y, max_width):
        """Swatch + text items, wrapped to the plate's width."""
        cursor, line = x, y
        for style, content in self.legend:
            # CJK glyphs are one em wide, the rest about half.
            width = sum(13 if ord(c) > 0x2E7F else 7.2 for c in content) + 22 + 26
            if cursor > x and cursor + width > x + max_width:
                cursor, line = x, line + 26
            fill, stroke, _, _ = STYLES[style]
            self.parts.append(
                f'<rect x="{cursor:.1f}" y="{line - 12:.1f}" width="14" height="14" rx="4" '
                f'fill="{fill}" stroke="{stroke}"/>'
            )
            self.text(cursor + 22, line, content, 13, "#424245")
            cursor += width
        return line

    def svg(self):
        layout = self.layout
        main_units = max(
            sum(3.0 if spec == "arrows" else spec.get("w", spec.get("gap", 0)) for spec in row)
            for row in layout["rows"]
        )
        cluster_units = 3.5 if layout["cluster"] else 0
        plate_width = (main_units + cluster_units) * UNIT - GAP + 2 * PAD
        left = MARGIN + PAD
        top = MARGIN + PAD
        y = top
        if layout["fn_row"]:
            self.draw_row(layout["fn_row"], left, y, FN_ROW - GAP)
            y += FN_ROW + 4
        rows_top = y
        for row in layout["rows"]:
            self.draw_row(row, left, y, ROW - GAP)
            y += ROW
        if layout["cluster"]:
            cluster_left = left + (main_units + 0.5) * UNIT
            for (row, column), spec in layout["cluster"].items():
                self.draw_key(spec, cluster_left + column * UNIT, rows_top + row * ROW, UNIT - GAP, ROW - GAP)
        plate_height = y - GAP + PAD - MARGIN
        legend_bottom = self.draw_legend(MARGIN + 4, MARGIN + plate_height + 30, plate_width - 8)
        width = plate_width + 2 * MARGIN
        height = legend_bottom + MARGIN
        plate = (
            f'<rect x="{MARGIN}" y="{MARGIN}" width="{plate_width:.1f}" height="{plate_height:.1f}" '
            f'rx="16" fill="{PLATE}"/>'
        )
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" '
            f'viewBox="0 0 {width:.0f} {height:.0f}" font-family="{FONT}">'
            f'<rect width="100%" height="100%" fill="#ffffff"/>{plate}{"".join(self.parts)}</svg>'
        )


def marks(style, captions):
    return {key_id: ({"style": style, "caption": caption} if caption else {"style": style})
            for key_id, caption in captions.items()}


SLOT_KEYS = dict(zip("qwdfzxvy;", "123456789"))
ARROWS = {"left": None, "right": None, "up": None, "down": None}


def typing(mac):
    enter = "Return" if mac else "Enter"
    chord = "option" if mac else "Ctrl"
    shift_caption = f"＋ Tab／{enter}／選字" if mac else f"＋ Tab／{enter}／選字\n揤一下：台語 ⇄ 英文"
    blue = marks("blue", {
        "enter": "確定", "space": "確定", "tab": "後一个字",
        "[": "頂一頁", "]": "後一頁", "esc": None, "shift": shift_caption,
        **SLOT_KEYS, **ARROWS,
    })
    caret_keys = {"opt": None, "ropt": None} if mac else {"ctrl": None, "rctrl": None}
    orange = marks("orange-solid", caret_keys)
    legend = [
        ("blue", "輸出佮選字"),
        ("blue", "1–9：直接選第幾个候選詞"),
        ("blue", "Esc：刪除拍的字"),
        ("orange-solid", f"{chord} ＋ ← → ↑ ↓：修正拍的字"),
    ]
    if not mac:
        orange.update(marks("orange", {"home": "上頭前", "end": "上尾"}))
        legend.append(("orange", "Home／End：跳去上頭前／上尾"))
    return {**blue, **orange}, legend


def shortcuts(mac):
    held = {"ctrl": None, "cmd": None} if mac else {"ctrl": None, "alt": None}
    result = marks("blue-solid", held)
    result.update(marks("blue", {
        "s": "設定", "c": "台羅\n白話字", "p": "方音\n符號", "h": "候選詞\n顯示",
        "j": "方音符號\n齒盤", ",": "符號\n選單", "/": "Telex\n說明",
    }))
    result.update(marks("orange", {"`": "漢字\n羅馬字"}))
    legend = [
        ("blue-solid", "control 佮 command 做伙揤牢" if mac else "Ctrl 佮 Alt 做伙揤牢"),
        ("blue", "閣揤這个齒"),
        ("orange", "單獨揤"),
    ]
    if not mac:
        result.update(marks("orange", {"shift": "揤一下：台語 ⇄ 英文"}))
    return result, legend


def punctuation(mac):
    result = marks("blue-solid", {"ctrl": None} if mac else {"ctrl": None, "rctrl": None})
    result.update(marks("orange", {"shift": None, "rshift": None}))
    name = "control" if mac else "Ctrl"
    legend = [
        ("blue", "藍色的字：漢字排頭前的時輸出的全角標點"),
        ("orange", "Shift：頂懸彼逝"),
        ("blue-solid", f"{name} ＋ 標點齒：這擺換做另外一款（全角 ⇄ 半角）"),
    ]
    return result, legend


def tones_numeric():
    tone_keys = dict(zip("12345789", ["a", "á", "à", "ah", "â", "ā", "a̍h", "a̋"]))
    result = marks("blue", tone_keys)
    result.update(marks("orange", SLOT_KEYS))
    return result, [("blue", "聲調：字母後壁拍數字"), ("orange", "直接選第幾个候選詞")]


def tones_telex():
    result = marks("blue", {
        "x": "1／4 調", "v": "2／8 調", "y": "3 調", "d": "5 調", "w": "7 調", "q": "9 調",
        "z": "ts", "f": "連字符 -",
    })
    result.update(marks("orange", {str(n): None for n in range(1, 10)}))
    return result, [("blue", "聲調、ts、連字符"), ("orange", "直接選第幾个候選詞")]


def pictures():
    for name, layout, mac in (("macos", MAC, True), ("windows", WINDOWS, False)):
        yield f"typing-{name}", Picture(layout, *typing(mac))
        yield f"shortcuts-{name}", Picture(layout, *shortcuts(mac))
        yield f"punctuation-{name}", Picture(layout, *punctuation(mac), punctuation=True)
    yield "tones-numeric", Picture(LETTERS_ONLY, *tones_numeric())
    yield "tones-telex", Picture(LETTERS_ONLY, *tones_telex())


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, picture in pictures():
        (out / f"{name}.svg").write_text(picture.svg(), encoding="utf-8")


if __name__ == "__main__":
    main()
