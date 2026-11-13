#!/usr/bin/env python3
"""Compare MoE ODS data and maintain the generated section of its update issue."""

from __future__ import annotations

import argparse
import hashlib
import html
import io
import json
import re
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter
from collections.abc import Iterable
from dataclasses import asdict, dataclass
from decimal import Decimal
from itertools import zip_longest
from pathlib import Path

TABLE = "{urn:oasis:names:tc:opendocument:xmlns:table:1.0}"
OFFICE = "{urn:oasis:names:tc:opendocument:xmlns:office:1.0}"
TEXT = "{urn:oasis:names:tc:opendocument:xmlns:text:1.0}"
TITLE = "chore(dictionary): MoE kautian.ods has new data"
START, END = "<!-- kautian-watch:start -->", "<!-- kautian-watch:end -->"
# Issue text is Taigi Hanji, worded after sutian.moe.edu.tw/und-hani (maintainer request).
FORMAT_MARKER = "<!-- kautian-watch:format:6 -->"
# Sheets the dictionary build reads, with their reading columns
# (dictionary/sources/official/kautian/config.yaml select.sheets + dialect_sheet;
# kautian_watch_test keeps the two in sync).
READING_SHEETS = {
    **dict.fromkeys(("詞目", "又唸作", "合音唸作", "俗唸作", "詞彙比較", "名", "姓"), ("羅馬字",)),
    "語音差異": (
        "鹿港偏泉腔",
        "三峽偏泉腔",
        "臺北偏泉腔",
        "宜蘭偏漳腔",
        "臺南混合腔",
        "高雄混合腔",
        "金門偏泉腔",
        "馬公偏泉腔",
        "新竹偏泉腔",
        "臺中偏漳腔",
    ),
}
# Same separators as dictionary/common/expand.py DEFAULT_SEPARATORS.
SEPARATORS = re.compile(r"[/,]")
# Keeps the issue body (summary tables + 24,000-char diff) under GitHub's limit.
SUMMARY_TABLE_CHARS = 4000
# Plain-language names for sheets that do not reach the dictionary.
SHEET_TOPICS = {
    # Dropped at merge; the variants stage reads the frozen variants.csv instead.
    "異用字": "異用字",
    "義項": "釋義",
    "例句": "例句",
    "羅馬字清單": "羅馬字清單",
    "漢字羅馬字對應": "漢字羅馬字對應",
}
# These are source-record identifiers, not dictionary word/deduplication keys.
SOURCE_KEYS = {
    "詞目": ("詞目id",),
    "義項": ("詞目id", "義項id"),
    "例句": ("詞目id", "義項id", "例句順序"),
}
Row = tuple[str, ...]


@dataclass
class Sheet:
    columns: Row
    rows: Counter[Row]


@dataclass
class RowChange:
    values: Row
    count: int


@dataclass
class Modification:
    before: Row
    after: Row


@dataclass
class SheetDiff:
    name: str
    columns_before: Row | None
    columns_after: Row | None
    total_before: int
    total_after: int
    added: list[RowChange]
    removed: list[RowChange]
    modified: list[Modification]


def repeat(element: ET.Element, attribute: str, maximum: int) -> int:
    count = int(element.get(attribute, "1"))
    if not 1 <= count <= maximum:
        raise ValueError(f"Invalid ODS repetition: {attribute}={count}")
    return count


def text_content(element: ET.Element) -> str:
    special = {TEXT + "tab": "\t", TEXT + "line-break": "\n"}
    if element.tag == TEXT + "s":
        return " " * repeat(element, TEXT + "c", 32767)
    if element.tag in special:
        return special[element.tag]
    return (element.text or "") + "".join(
        text_content(child) + (child.tail or "") for child in element
    )


def cell_value(cell: ET.Element) -> str:
    if cell.get(TABLE + "formula"):
        raise ValueError("Unexpected formula in source export")
    kind = cell.get(OFFICE + "value-type", "string")
    if kind in {"float", "currency", "percentage"}:
        return format(Decimal(cell.attrib[OFFICE + "value"]).normalize(), "f")
    for attribute in ("boolean-value", "date-value", "time-value", "string-value"):
        if OFFICE + attribute in cell.attrib:
            return cell.attrib[OFFICE + attribute]
    if kind not in {"string", "void"}:
        raise ValueError(f"Unsupported ODS cell type: {kind}")
    return "\n".join(text_content(p) for p in cell.findall(TEXT + "p"))


def row_values(element: ET.Element) -> Row:
    values: list[str] = []
    pending_empty = 0
    for cell in element:
        if cell.tag not in {TABLE + "table-cell", TABLE + "covered-table-cell"}:
            continue
        count = repeat(cell, TABLE + "number-columns-repeated", 16384)
        value = cell_value(cell)
        if not value:
            pending_empty += count
            continue
        if len(values) + pending_empty + count > 256:
            raise ValueError("Source export exceeds 256 populated columns")
        values.extend([""] * pending_empty + [value] * count)
        pending_empty = 0
    return tuple(values)


def read_ods(path: Path) -> dict[str, Sheet]:
    with zipfile.ZipFile(path) as archive:
        if archive.getinfo("content.xml").file_size > 128 * 1024 * 1024:
            raise ValueError("ODS content.xml exceeds 128 MiB")
        content = archive.read("content.xml")
    if b"<!DOCTYPE" in content or b"<!ENTITY" in content:
        raise ValueError("Source export must not contain XML entity declarations")
    sheets: dict[str, Sheet] = {}
    current: Sheet | None = None
    for event, element in ET.iterparse(io.BytesIO(content), events=("start", "end")):
        if element.tag == TABLE + "table":
            if event == "start":
                name = element.attrib[TABLE + "name"]
                if current is not None or name in sheets:
                    raise ValueError("Nested or duplicate ODS table")
                current = Sheet((), Counter())
                sheets[name] = current
            else:
                if current is None or not current.columns:
                    raise ValueError("Source sheet has no column headers")
                current = None
                element.clear()
        elif event == "end" and element.tag == TABLE + "table-row":
            if current is None:
                raise ValueError("ODS row outside a sheet")
            values = row_values(element)
            count = repeat(element, TABLE + "number-rows-repeated", 1048576)
            if values:
                if not current.columns:
                    current.columns = values
                    count -= 1
                if count:
                    current.rows[values] += count
            element.clear()
    if not sheets:
        raise ValueError("Source export contains no sheets")
    return sheets


def fingerprint(sheets: dict[str, Sheet]) -> str:
    data = [
        (name, sheet.columns, sorted(sheet.rows.items())) for name, sheet in sorted(sheets.items())
    ]
    return hashlib.sha256(json.dumps(data, ensure_ascii=False).encode()).hexdigest()


def unique_rows(sheet: Sheet, columns: tuple[str, ...]) -> dict[Row, Row]:
    if not all(column in sheet.columns for column in columns):
        return {}
    indices = [sheet.columns.index(column) for column in columns]
    indexed: dict[Row, Row] = {}
    for row, count in sheet.rows.items():
        key = tuple(row[index] if index < len(row) else "" for index in indices)
        if count != 1 or not all(key) or key in indexed:
            return {}
        indexed[key] = row
    return indexed


def compare(before: dict[str, Sheet], after: dict[str, Sheet]) -> list[SheetDiff]:
    differences = []
    for name in sorted(before.keys() | after.keys()):
        old, new = before.get(name), after.get(name)
        old_rows = old.rows if old else Counter()
        new_rows = new.rows if new else Counter()
        removed, added = old_rows - new_rows, new_rows - old_rows
        modified = []
        if old and new and old.columns == new.columns and name in SOURCE_KEYS:
            old_index = unique_rows(old, SOURCE_KEYS[name])
            new_index = unique_rows(new, SOURCE_KEYS[name])
            for key in sorted(old_index.keys() & new_index.keys()):
                left, right = old_index[key], new_index[key]
                if removed[left] and added[right]:
                    modified.append(Modification(left, right))
                    removed[left] -= 1
                    added[right] -= 1
        differences.append(
            SheetDiff(
                name,
                old.columns if old else None,
                new.columns if new else None,
                sum(old_rows.values()),
                sum(new_rows.values()),
                [RowChange(row, count) for row, count in sorted(added.items()) if count],
                [RowChange(row, count) for row, count in sorted(removed.items()) if count],
                modified,
            )
        )
    return differences


def changed(sheet: SheetDiff) -> bool:
    return bool(
        sheet.added
        or sheet.removed
        or sheet.modified
        or sheet.columns_before != sheet.columns_after
    )


def safe(value: str, limit: int | None = None) -> str:
    if limit is not None and len(value) > limit:
        value = value[:limit] + "…"
    # Escape Markdown/HTML and mentions before embedding upstream text in issues.
    replacements = {c: f"&#{ord(c)};" for c in "\\`*_{}[]()#+-.!|~@\t"}
    replacements["\n"] = "<br>"
    return "".join(replacements.get(char, html.escape(char)) for char in value)


def counts(sheet: SheetDiff) -> tuple[int, int, int]:
    return (
        sum(row.count for row in sheet.added),
        sum(row.count for row in sheet.removed),
        len(sheet.modified),
    )


def summary_table(sheets: list[SheetDiff], *, brief: bool) -> str:
    lines = [
        "| 資料表 | 進前 | 了後 | 增加 | 刪掉 | 改 |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for sheet in sheets:
        totals = (sheet.total_before, sheet.total_after, *counts(sheet))
        lines.append(
            f"| {safe(sheet.name, 80 if brief else None)} | "
            + " | ".join(f"{number:,}" for number in totals)
            + " |"
        )
    return "\n".join(lines)


def code_text(value: str) -> str:
    # Fenced code preserves Unicode; JSON escaping keeps controls on one signed line.
    # Marker-like source text must not confuse the generated-section parser.
    return json.dumps(value, ensure_ascii=False)[1:-1].replace("<", "\\u003c")


def column_label(columns: Row, index: int) -> str:
    if index >= len(columns) or not columns[index]:
        return f"第 {index + 1} 欄"
    name = columns[index]
    return name if columns.count(name) == 1 else f"{name} [第 {index + 1} 欄]"


def row_context(columns: Row, row: Row) -> str:
    fields = dict(zip(columns, row))
    identifiers = [
        f"{column}={value}" for column, value in zip(columns, row) if column.endswith("id")
    ]
    name = fields.get("漢字", fields.get("詞目漢字", fields.get("羅馬字", "")))
    if target := fields.get("對應詞目漢字"):
        name += f" → {target}"
    return code_text(" · ".join([*identifiers, name] if name else identifiers))


def diff_hunks(sheet: SheetDiff, *, brief: bool):
    if not brief or sheet.columns_before != sheet.columns_after:
        yield "\n".join(
            [
                "@@ 欄位名（null 是無這个資料表）@@",
                "- " + json.dumps(sheet.columns_before, ensure_ascii=False).replace("<", "\\u003c"),
                "+ " + json.dumps(sheet.columns_after, ensure_ascii=False).replace("<", "\\u003c"),
            ]
        )
    for modification in sheet.modified:
        columns = sheet.columns_before or ()
        context = row_context(columns, modification.before)
        name_index = columns.index("漢字") if "漢字" in columns else None
        if name_index is not None:
            old_name = (
                modification.before[name_index] if name_index < len(modification.before) else ""
            )
            new_name = (
                modification.after[name_index] if name_index < len(modification.after) else ""
            )
            if old_name != new_name:
                context += " → " + code_text(new_name)
        lines = [f"@@ {context} @@"]
        for index in range(max(len(modification.before), len(modification.after))):
            before = modification.before[index] if index < len(modification.before) else None
            after = modification.after[index] if index < len(modification.after) else None
            label = code_text(column_label(columns, index))
            if before != after:
                for sign, value in (("-", before), ("+", after)):
                    display = code_text(value) if value else "（空的）"
                    lines.append(f"{sign} {label}: {display}")
            elif not brief:
                lines.append(f"  {label}: {code_text(before) if before else '（空的）'}")
        yield "\n".join(lines)
    for sign, rows, columns in (
        ("+", sheet.added, sheet.columns_after),
        ("-", sheet.removed, sheet.columns_before),
    ):
        for row in rows:
            columns = columns or ()
            context = row_context(columns, row.values)
            suffix = f" · ×{row.count}" if row.count > 1 else ""
            lines = [f"@@ {context}{suffix} @@"]
            for index, value in enumerate(row.values):
                lines.append(
                    f"{sign} {code_text(column_label(columns, index))}: "
                    + (code_text(value) or "（空的）")
                )
            yield "\n".join(lines)


def render(differences: list[SheetDiff], *, brief: bool) -> str:
    active = [sheet for sheet in differences if changed(sheet)]
    totals = [sum(counts(sheet)[index] for sheet in active) for index in range(3)]
    blocks = [
        "## 原始資料差異",
        (f"**增加 {totals[0]} · 刪掉 {totals[1]} · 改 {totals[2]}**，佇 {len(active)} 个資料表。"),
        "數量算原始資料的列；改的列另外算。",
    ]
    if not active:
        blocks.append("佮已經存的版本無差異。")
    tables = [summary_table(active, brief=brief)] if active else []
    tables.append(
        f"<details>\n<summary>全部 {len(differences)} 个資料表</summary>\n\n"
        + summary_table(differences, brief=brief)
        + "\n\n</details>"
    )
    omitted = "摘要傷長，賰的請看完整報告。"
    limit = 24000 if brief else None
    for table in tables:
        if limit and len("\n\n".join([*blocks, table, omitted])) > limit:
            return "\n\n".join([*blocks, omitted]) + "\n"
        blocks.append(table)
    for sheet in active:
        labels = ", ".join(
            f"{label} {number}"
            for number, label in zip(counts(sheet), ("增加", "刪掉", "改"))
            if number
        )
        title = f"### {safe(sheet.name, 80 if brief else None)} — " + (labels or "欄位名有改")
        hunks, fence_length, body_length = [], 3, 0
        truncated = False
        remaining = limit - len("\n\n".join([*blocks, title, omitted])) - 24 if limit else None
        for hunk in diff_hunks(sheet, brief=brief):
            candidate_fence = max(
                fence_length, max((len(m[0]) + 1 for m in re.finditer(r"`+", hunk)), default=3)
            )
            candidate_length = body_length + len(hunk) + 2
            if remaining is not None and candidate_length + 2 * candidate_fence > remaining:
                truncated = True
                break
            hunks.append(hunk)
            body_length, fence_length = candidate_length, candidate_fence
        if hunks:
            fence = "`" * fence_length
            blocks.append(title + f"\n\n{fence}diff\n" + "\n\n".join(hunks) + f"\n{fence}")
        if truncated:
            blocks.append(omitted)
            break
    return "\n\n".join(blocks) + "\n"


def parts(value: str) -> list[str]:
    return [part.strip() for part in SEPARATORS.split(value) if part.strip()]


def split_pairs(hanjis: list[str], reading: str) -> list[tuple[str, str]]:
    # Mirrors dictionary/common/expand.py expand_dataframe pairing and padding.
    readings = parts(reading)
    if not readings:
        return []
    return [
        (
            hanjis[index] if index < len(hanjis) else (hanjis[0] if hanjis else ""),
            readings[index] if index < len(readings) else readings[0],
        )
        for index in range(max(len(readings), len(hanjis)))
    ]


def readings(sheets: dict[str, Sheet]) -> dict[tuple[str, str], set[str]]:
    """(Hanji, romanization) pairs the dictionary build sees → sheets / dialects holding them."""
    found: dict[tuple[str, str], set[str]] = {}
    for name, reading_columns in READING_SHEETS.items():
        sheet = sheets.get(name)
        if not sheet or "漢字" not in sheet.columns:
            continue
        hanji_index = sheet.columns.index("漢字")
        # 語音差異 holds one reading per dialect column; other sheets have one 羅馬字.
        holders = [
            (sheet.columns.index(column), column if len(reading_columns) > 1 else name)
            for column in reading_columns
            if column in sheet.columns
        ]
        for row in sheet.rows:
            hanjis = parts(row[hanji_index]) if hanji_index < len(row) else []
            for index, holder in holders:
                for pair in split_pairs(hanjis, row[index] if index < len(row) else ""):
                    found.setdefault(pair, set()).add(holder)
    return found


def sheet_topic(name: str) -> str:
    if "近義" in name:
        return "近義"
    if "反義" in name:
        return "反義"
    return SHEET_TOPICS.get(name) or safe(name, 80)


def bounded(lines: Iterable[str], total: int, *, brief: bool) -> list[str]:
    shown, length = [], 0
    for line in lines:
        length += len(line) + 1
        if brief and length > SUMMARY_TABLE_CHARS:
            return shown + ["", f"…猶有 {total - len(shown)} 項，請看完整報告。"]
        shown.append(line)
    return shown


def pair_table(
    title: str, rows: list[Row], *, header: Row = ("漢字", "音讀", "資料表"), brief: bool
) -> str:
    body = (
        f"| {' | '.join(safe(value, 80 if brief else None) for value in row)} |" for row in rows
    )
    head = [f"**{title}（{len(rows)}）**", "", f"| {' | '.join(header)} |", "| --- | --- | --- |"]
    return "\n".join(head + bounded(body, len(rows), brief=brief))


def link_kind(sheet: SheetDiff) -> str | None:
    """Synonym / antonym link sheets with a stable layout; None for any other sheet."""
    columns = sheet.columns_before or ()
    if sheet.columns_before != sheet.columns_after or not {"詞目漢字", "對應詞目漢字"} <= set(
        columns
    ):
        return None
    return "近義" if "近義" in sheet.name else "反義" if "反義" in sheet.name else None


def link_changes(differences: list[SheetDiff], *, brief: bool) -> list[str]:
    """Synonym / antonym links per (word, linked word), across headword and sense sheets."""
    links: dict[tuple[str, str, str], dict[str, list[str]]] = {}
    for sheet in differences:
        if not (kind := link_kind(sheet)):
            continue
        for side, rows in (("removed", sheet.removed), ("added", sheet.added)):
            for row in rows:
                fields = dict(zip(sheet.columns_before or (), row.values))
                # 義項 sheets link one sense of the word; 詞目 sheets the whole word.
                scope = f"釋義「{fields['解說']}」" if fields.get("解說") else "規个詞目"
                key = (kind, fields.get("詞目漢字", ""), fields.get("對應詞目漢字", ""))
                links.setdefault(key, {}).setdefault(side, []).append(scope)
    tables: dict[str, list[Row]] = {}
    for (kind, word, target), sides in sorted(links.items()):
        removed, added = "; ".join(sides.get("removed", [])), "; ".join(sides.get("added", []))
        change = (
            f"{removed} → {added}" if removed and added else f"增加：{added}" if added else "刪掉"
        )
        tables.setdefault(kind, []).append((word, target, change))
    return [
        pair_table(kind, rows, header=("詞目", kind, "改變"), brief=brief)
        for kind, rows in tables.items()
    ]


def side_effects(differences: list[SheetDiff], *, brief: bool) -> list[str]:
    """Changed source rows that alter no (Hanji, romanization) pair."""
    edits: dict[str, Counter[str]] = {}
    topics: dict[str, Counter[str]] = {}
    for sheet in differences:
        if not changed(sheet):
            continue
        if sheet.name in READING_SHEETS:
            columns = sheet.columns_before or ()
            typing_columns = {"漢字", *READING_SHEETS[sheet.name]}
            for modification in sheet.modified:
                touched = {
                    column_label(columns, index)
                    for index, (old, new) in enumerate(
                        zip_longest(modification.before, modification.after)
                    )
                    if old != new
                }
                if not typing_columns & touched:
                    edits.setdefault(sheet.name, Counter()).update(touched)
            continue
        if link_kind(sheet):
            continue
        totals = topics.setdefault(sheet_topic(sheet.name), Counter())
        totals.update(dict(zip(("增加", "刪掉", "改"), counts(sheet))))
    lines = [
        f"- {safe(name)}：{safe(column, 80)}改 {number} 項"
        for name, fields in edits.items()
        for column, number in sorted(fields.items())
    ]
    for topic, totals in topics.items():
        labels = "、".join(f"{label} {number}" for label, number in totals.items() if number)
        lines.append(f"- {topic}：{labels or '欄位名有改'}")
    return bounded(lines, len(lines), brief=brief)


def typing_summary(
    old: dict[tuple[str, str], set[str]],
    new: dict[tuple[str, str], set[str]],
    differences: list[SheetDiff],
    *,
    brief: bool,
) -> str:
    """Takes readings() of the committed and upstream exports."""

    def holders(pair: tuple[str, str], side: dict[tuple[str, str], set[str]]) -> str:
        return ", ".join(sorted(side[pair]))

    tables = [
        ("增加的音讀", [(*pair, holders(pair, new)) for pair in new.keys() - old.keys()]),
        ("刪掉的音讀", [(*pair, holders(pair, old)) for pair in old.keys() - new.keys()]),
        # Sheet / dialect membership feeds the Kautian subtag and accent mask.
        (
            "徙位的音讀",
            [
                (*pair, f"{holders(pair, old)} → {holders(pair, new)}")
                for pair in old.keys() & new.keys()
                if old[pair] != new[pair]
            ],
        ),
    ]
    blocks = ["## 這擺教典的改變"]
    if headers := [
        safe(sheet.name, 80)
        for sheet in differences
        if sheet.name in READING_SHEETS and sheet.columns_before != sheet.columns_after
    ]:
        blocks.append(f"⚠ 欄位名有改：{'、'.join(headers)}。下面的清單無準。")
    blocks += [pair_table(title, sorted(rows), brief=brief) for title, rows in tables if rows]
    if not any(rows for _, rows in tables):
        blocks.append("音讀無改變。")
    others = side_effects(differences, brief=brief)
    parts = (["\n".join(others)] if others else []) + link_changes(differences, brief=brief)
    if parts:
        blocks.append("\n\n".join(["**其他改變**", *parts]))
    return "\n\n".join(blocks) + "\n\n"


def generate(arguments: argparse.Namespace) -> None:
    before, after = read_ods(arguments.committed), read_ods(arguments.upstream)
    differences = compare(before, after)
    old_readings, new_readings = readings(before), readings(after)
    data = {
        "baseline_commit": arguments.baseline_commit,
        "baseline_data_sha256": fingerprint(before),
        "upstream_data_sha256": fingerprint(after),
        "upstream_file_sha256": hashlib.sha256(arguments.upstream.read_bytes()).hexdigest(),
        "sheets": [asdict(sheet) for sheet in differences],
    }
    identity = hashlib.sha256(
        (data["baseline_data_sha256"] + data["upstream_data_sha256"]).encode()
    ).hexdigest()
    arguments.output.mkdir(parents=True, exist_ok=True)
    (arguments.output / "fingerprint.txt").write_text(identity, encoding="utf-8")
    (arguments.output / "report.json").write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    for filename, brief in (("summary.md", True), ("diff.md", False)):
        (arguments.output / filename).write_text(
            ("" if brief else f"比對基準 commit：`{arguments.baseline_commit}`\n\n")
            + typing_summary(old_readings, new_readings, differences, brief=brief)
            + render(differences, brief=brief),
            encoding="utf-8",
        )


def generated_section(body: str) -> str | None:
    if START not in body and END not in body:
        return None
    if body.count(START) != 1 or body.count(END) != 1 or body.index(START) > body.index(END):
        raise ValueError("Issue has malformed generated-section markers; preserve it unchanged")
    return body[body.index(START) : body.index(END) + len(END)]


def replace_section(body: str, section: str) -> str:
    old = generated_section(body)
    result = body.replace(old, section, 1) if old is not None else body.rstrip() + "\n\n" + section
    if len(result) > 60000:
        raise ValueError("Issue body too large; preserve human text unchanged")
    return result


def github(*arguments: str, payload: dict[str, str] | None = None):
    command = ["gh", "api", *arguments]
    if payload is not None:
        command += ["--input", "-"]
    result = subprocess.run(
        command,
        input=json.dumps(payload) if payload else None,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def publish(arguments: argparse.Namespace) -> None:
    report = json.loads((arguments.report / "report.json").read_text(encoding="utf-8"))
    if report["baseline_data_sha256"] == report["upstream_data_sha256"]:
        print("No data changes; preserve the latest issue diff")
        return
    endpoint = f"repos/{arguments.repo}/issues"
    pages = github(
        f"{endpoint}?state=open&creator=github-actions%5Bbot%5D&per_page=100",
        "--paginate",
        "--slurp",
    )
    matches = [
        issue
        for page in pages
        for issue in page
        if issue["title"] == TITLE
        and issue["user"]["login"] == "github-actions[bot]"
        and "pull_request" not in issue
    ]
    if len(matches) > 1:
        raise ValueError("Multiple matching watch issues; refusing ambiguous update")
    identity = (arguments.report / "fingerprint.txt").read_text(encoding="utf-8")
    marker = f"<!-- kautian-watch:data:{identity} -->"
    current = github(f"{endpoint}/{matches[0]['number']}") if matches else None
    existing = generated_section(current["body"] or "") if current else None
    if existing and marker in existing and FORMAT_MARKER in existing:
        print("Issue already has this data diff")
        return
    summary = (arguments.report / "summary.md").read_text(encoding="utf-8")
    section = (
        f"{START}\n{marker}\n{FORMAT_MARKER}\n{summary}\n"
        f"[完整報告（Markdown + JSON）]({arguments.artifact_url}) · [檢查記錄]({arguments.run_url})\n\n"
        "<details>\n<summary>比對資料</summary>\n\n"
        f"- 比對基準 commit：`{report['baseline_commit']}`\n"
        f"- 教典檔案上尾改的時間：{safe(arguments.last_modified, 200)}\n"
        f"- 教典檔案 SHA-256：`{report['upstream_file_sha256']}`\n"
        "- 完整報告保留 90 工，愛登入 GitHub。\n"
        "- 改的列是用兩爿攏有、無重複的 id 配對；其他的算增加抑是刪掉。\n\n</details>\n"
        f"{END}"
    )
    if current:
        github(
            f"{endpoint}/{current['number']}",
            "--method",
            "PATCH",
            payload={"body": replace_section(current["body"] or "", section)},
        )
        print(f"Updated issue #{current['number']}")
    else:
        body = "教育部公佈新版的教典資料：https://sutian.moe.edu.tw/media/senn/ods/kautian.ods"
        issue = github(
            endpoint,
            "--method",
            "POST",
            payload={"title": TITLE, "body": replace_section(body, section)},
        )
        print(f"Created issue #{issue['number']}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    comparison = commands.add_parser("compare")
    comparison.add_argument("committed", type=Path)
    comparison.add_argument("upstream", type=Path)
    comparison.add_argument("--baseline-commit", required=True)
    comparison.add_argument("--output", type=Path, required=True)
    publication = commands.add_parser("publish")
    publication.add_argument("--repo", required=True)
    publication.add_argument("--report", type=Path, required=True)
    publication.add_argument("--artifact-url", required=True)
    publication.add_argument("--run-url", required=True)
    publication.add_argument("--last-modified", default="unknown")
    arguments = parser.parse_args()
    (generate if arguments.command == "compare" else publish)(arguments)


if __name__ == "__main__":
    main()
