"""Fixtures for source-data semantics and mocked issue publication."""

import argparse
import json
import re
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter
from pathlib import Path
from unittest.mock import Mock

import kautian_watch as watch
import pytest


def ods(path, sheets):
    document = ET.Element(watch.OFFICE + "document-content")
    for name, rows in sheets.items():
        table = ET.SubElement(document, watch.TABLE + "table", {watch.TABLE + "name": name})
        for values in rows:
            row = ET.SubElement(table, watch.TABLE + "table-row")
            for value in values:
                cell = ET.SubElement(row, watch.TABLE + "table-cell")
                if isinstance(value, int):
                    cell.set(watch.OFFICE + "value-type", "float")
                    cell.set(watch.OFFICE + "value", str(value))
                else:
                    ET.SubElement(cell, watch.TEXT + "p").text = value
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("content.xml", ET.tostring(document))
    return path


def test_source_rows_counts_ids_and_modifications(tmp_path):
    columns = ["詞目id", "漢字", "羅馬字"]
    old = ods(
        tmp_path / "old.ods",
        {"詞目": [columns, [1, "重", "tîng"], [2, "重", "tāng"], [3, "古", "kóo"]]},
    )
    new = ods(
        tmp_path / "new.ods",
        {"詞目": [columns, [1, "重", "tîng"], [2, "重", "tāng-á"], [4, "早", "tsá"]]},
    )
    result = watch.compare(watch.read_ods(old), watch.read_ods(new))[0]
    assert (result.total_before, result.total_after) == (3, 3)
    assert result.added == [watch.RowChange(("4", "早", "tsá"), 1)]
    assert result.removed == [watch.RowChange(("3", "古", "kóo"), 1)]
    assert result.modified == [watch.Modification(("2", "重", "tāng"), ("2", "重", "tāng-á"))]


def test_duplicates_disable_pairing_across_complete_sheet():
    old = watch.Sheet(("詞目id", "漢字"), Counter({("1", "甲"): 2, ("2", "乙"): 1}))
    new = watch.Sheet(old.columns, Counter({("1", "甲"): 2, ("2", "丙"): 1}))
    result = watch.compare({"詞目": old}, {"詞目": new})[0]
    assert result.modified == []
    assert result.added == [watch.RowChange(("2", "丙"), 1)]
    result = watch.compare(
        {"姓": old}, {"姓": watch.Sheet(old.columns, Counter({("1", "甲"): 1}))}
    )[0]
    assert sum(row.count for row in result.removed) == 2


def test_order_and_packaging_do_not_change_semantics(tmp_path):
    old = watch.read_ods(
        ods(
            tmp_path / "old.ods",
            {"名": [["漢字", "羅馬字"], ["九", "kiú"], ["九", "káu"]]},
        )
    )
    new = watch.read_ods(
        ods(
            tmp_path / "new.ods",
            {"名": [["漢字", "羅馬字"], ["九", "káu"], ["九", "kiú"]]},
        )
    )
    assert watch.fingerprint(old) == watch.fingerprint(new)
    assert not any(watch.changed(sheet) for sheet in watch.compare(old, new))


def test_ods_whitespace_values_repetitions_and_blank_tails(tmp_path):
    row = ET.Element(watch.TABLE + "table-row")
    cell = ET.SubElement(row, watch.TABLE + "table-cell")
    paragraph = ET.SubElement(cell, watch.TEXT + "p")
    paragraph.text = "A"
    ET.SubElement(paragraph, watch.TEXT + "s", {watch.TEXT + "c": "2"}).tail = "B"
    ET.SubElement(paragraph, watch.TEXT + "tab").tail = "C"
    ET.SubElement(paragraph, watch.TEXT + "line-break").tail = "D"
    ET.SubElement(cell, watch.TEXT + "p").text = "E"
    ET.SubElement(
        row,
        watch.TABLE + "table-cell",
        {watch.TABLE + "number-columns-repeated": "16383"},
    )
    assert watch.row_values(row) == ("A  B\tC\nD\nE",)
    assert (
        watch.cell_value(
            ET.Element(
                "cell",
                {watch.OFFICE + "value-type": "float", watch.OFFICE + "value": "1.0"},
            )
        )
        == "1"
    )
    assert (
        watch.cell_value(
            ET.Element(
                "cell",
                {
                    watch.OFFICE + "value-type": "boolean",
                    watch.OFFICE + "boolean-value": "true",
                },
            )
        )
        == "true"
    )
    path = ods(tmp_path / "repeat.ods", {"名": [["漢字"], ["九"]]})
    with zipfile.ZipFile(path) as archive:
        root = ET.fromstring(archive.read("content.xml"))
    table = root.find(watch.TABLE + "table")
    table[1].set(watch.TABLE + "number-rows-repeated", "3")
    blank = ET.SubElement(
        table,
        watch.TABLE + "table-row",
        {watch.TABLE + "number-rows-repeated": "1048576"},
    )
    ET.SubElement(blank, watch.TABLE + "table-cell")
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("content.xml", ET.tostring(root))
    assert watch.read_ods(path)["名"].rows == Counter({("九",): 3})


def test_sheet_and_header_changes_are_explicit():
    old = {
        "keep": watch.Sheet(("a",), Counter()),
        "removed": watch.Sheet(("x",), Counter({("1",): 1})),
    }
    new = {
        "keep": watch.Sheet(("b",), Counter()),
        "added": watch.Sheet(("x",), Counter()),
    }
    result = watch.compare(old, new)
    assert all(watch.changed(sheet) for sheet in result)
    assert "@@ 欄位名（null 是無這个資料表）@@" in watch.render(result, brief=True)


@pytest.mark.parametrize("content", [b"bad xml", b"<!DOCTYPE x><x/>", b"<x/>"])
def test_malformed_exports_fail(tmp_path, content):
    path = tmp_path / "bad.ods"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("content.xml", content)
    with pytest.raises((ValueError, ET.ParseError)):
        watch.read_ods(path)


def test_reports_are_bounded_escaped_and_lossless():
    value = "@everyone <script> | [link](url) `code`" + "字" * 30000
    sheet = watch.SheetDiff(
        "名", ("漢字",), ("漢字",), 0, 50, [watch.RowChange((value,), 50)], [], []
    )
    summary = watch.render([sheet], brief=True)
    assert len(summary) < 24050
    assert "摘要傷長" in summary
    assert value not in summary
    assert "字" * 30000 in watch.render([sheet], brief=False)


def test_escape_entities_and_keep_modification_pairs_complete():
    assert watch.safe("@<>&'|#") == "&#64;&lt;&gt;&amp;&#x27;&#124;&#35;"
    sheet = watch.SheetDiff(
        "名",
        ("漢字",),
        ("漢字",),
        1,
        4,
        [watch.RowChange((str(i),), 1) for i in range(3)],
        [],
        [watch.Modification(("九",), ("八",))],
    )
    summary = watch.render([sheet], brief=True)
    assert "- 漢字: 九\n+ 漢字: 八" in summary
    sheet.modified = [watch.Modification(("字" * 1000,), ("語" * 1000,))] * 20
    summary = watch.render([sheet] * 30, brief=True)
    assert len(summary) < 24050
    assert summary.count("- 漢字:") == summary.count("+ 漢字:")
    assert summary.count("```diff") == len(re.findall(r"^```$", summary, re.MULTILINE))
    assert summary.count("<details>") == summary.count("</details>")


def test_approved_layout_shows_all_modifications_and_changed_fields_only():
    columns = ("詞目id", "漢字", "羅馬字")
    modifications = [
        watch.Modification((str(i), "水秤", "suí-tshìn"), (str(i), "水秤", "tsuí-tshìn"))
        for i in range(10)
    ]
    sheet = watch.SheetDiff("詞目", columns, columns, 29592, 29592, [], [], modifications)
    summary = watch.render([sheet], brief=True)
    assert "**增加 0 · 刪掉 0 · 改 10**" in summary
    assert "<summary>全部 1 个資料表</summary>" in summary
    assert "29,592" in summary
    assert summary.count("@@ 詞目id=") == 10
    assert "- 羅馬字: suí-tshìn\n+ 羅馬字: tsuí-tshìn" in summary
    assert "漢字: 水秤" not in summary
    assert "Before columns" not in summary
    assert "  漢字: 水秤" in watch.render([sheet], brief=False)


def test_duplicate_blank_and_overflow_column_values_keep_positions():
    columns = ("漢字", "漢字", "")
    sheet = watch.SheetDiff(
        "名", columns, columns, 0, 2, [watch.RowChange(("九", "八", "七", "六"), 2)], [], []
    )
    summary = watch.render([sheet], brief=True)
    for line in (
        "+ 漢字 [第 1 欄]: 九",
        "+ 漢字 [第 2 欄]: 八",
        "+ 第 3 欄: 七",
        "+ 第 4 欄: 六",
    ):
        assert line in summary
    assert "×2" in summary


def test_fenced_values_cannot_close_code_block_or_inject_unsigned_lines():
    value = "@everyone <script>\n````\n- forged\r\t"
    sheet = watch.SheetDiff(
        "名<script>|", ("漢字",), ("漢字",), 0, 1, [watch.RowChange((value,), 1)], [], []
    )
    summary = watch.render([sheet], brief=True)
    assert "`````diff\n" in summary
    assert "\n`````\n" in summary
    assert "+ 漢字: @everyone \\u003cscript>\\n````\\n- forged\\r\\t" in summary
    assert "\n- forged" not in summary
    assert "### 名&lt;script&gt;&#124;" in summary


def test_source_marker_text_cannot_break_generated_section():
    columns = (watch.START, watch.END)
    sheet = watch.SheetDiff(
        "名", None, columns, 0, 1, [watch.RowChange((watch.START, watch.END), 1)], [], []
    )
    for brief in (True, False):
        rendered = watch.render([sheet], brief=brief)
        assert watch.START not in rendered and watch.END not in rendered
        body = f"{watch.START}\n{rendered}\n{watch.END}"
        assert watch.generated_section(body) == body
        assert "\\u003c!-- kautian-watch:start -->" in rendered


def test_oversized_metadata_and_missing_name_are_bounded():
    sheet = watch.SheetDiff(
        "名",
        ("詞目id", "漢字"),
        ("詞目id", "漢字"),
        1,
        1,
        [],
        [],
        [watch.Modification(("1", "九"), ("1",))],
    )
    assert "+ 漢字: （空的）" in watch.render([sheet], brief=True)
    summary = watch.render([sheet] * 1000, brief=True)
    assert len(summary) <= 24000
    assert summary.count("<details>") == summary.count("</details>")
    assert "摘要傷長" in summary


def test_replace_only_generated_section_and_reject_bad_markers():
    section = f"{watch.START}\nnew\n{watch.END}"
    assert watch.replace_section("human", section) == "human\n\n" + section
    body = f"before\n{watch.START}\nold\n{watch.END}\nafter"
    assert watch.replace_section(body, section) == "before\n" + section + "\nafter"
    for body in (
        watch.START,
        watch.END,
        watch.END + watch.START,
        watch.START * 2 + watch.END,
    ):
        with pytest.raises(ValueError):
            watch.replace_section(body, section)


@pytest.fixture
def report(tmp_path, monkeypatch):
    before = ods(tmp_path / "old.ods", {"名": [["漢字"], ["九"]]})
    after = ods(tmp_path / "new.ods", {"名": [["漢字"], ["八"]]})
    output = tmp_path / "report"
    watch.generate(
        argparse.Namespace(
            committed=before, upstream=after, baseline_commit="abc123", output=output
        )
    )
    assert json.loads((output / "report.json").read_text())["sheets"][0]["added"][0]["values"] == [
        "八"
    ]
    return argparse.Namespace(
        repo="example/repo",
        report=output,
        artifact_url="https://example.org/artifact",
        run_url="https://example.org/run",
        last_modified="today",
    )


def issue(number=534, body="human refresh instructions", **extra):
    return {
        "number": number,
        "title": watch.TITLE,
        "user": {"login": "github-actions[bot]"},
        "body": body,
        **extra,
    }


def test_publish_create_existing_update_and_no_duplicate(report, monkeypatch):
    api = Mock(side_effect=[[[]], issue()])
    monkeypatch.setattr(watch, "github", api)
    watch.publish(report)
    assert api.call_args.kwargs["payload"]["title"] == watch.TITLE
    api.reset_mock(side_effect=True)
    # A title/author lookalike and a pull request must not be selected.
    lookalike = issue(1, user={"login": "human"})
    api.side_effect = [[[lookalike], [issue(), issue(2, pull_request={})]], issue(), {}]
    watch.publish(report)
    assert api.call_args.args[0].endswith("/534")
    body = api.call_args.kwargs["payload"]["body"]
    assert body.startswith("human refresh instructions\n")
    assert watch.FORMAT_MARKER in body
    assert "<summary>比對資料</summary>" in body
    assert "--paginate" in api.call_args_list[0].args
    api.reset_mock(side_effect=True)
    api.side_effect = [[[issue(body=body)]], issue(body=body)]
    watch.publish(report)
    assert api.call_count == 2


def test_old_layout_or_markers_outside_generated_section_refresh_once(report, monkeypatch):
    identity = (report.report / "fingerprint.txt").read_text()
    marker = f"<!-- kautian-watch:data:{identity} -->"
    for body in (
        f"{watch.START}\n{marker}\nold layout\n{watch.END}",
        f"{marker}\n{watch.FORMAT_MARKER}\nHuman text",
    ):
        api = Mock(side_effect=[[[issue(body=body)]], issue(body=body), {}])
        monkeypatch.setattr(watch, "github", api)
        watch.publish(report)
        assert api.call_count == 3
        updated = api.call_args.kwargs["payload"]["body"]
        assert watch.FORMAT_MARKER in watch.generated_section(updated)


def test_no_change_preserves_latest_issue_without_any_api_mutation(report, monkeypatch):
    data_path = report.report / "report.json"
    data = json.loads(data_path.read_text())
    data["upstream_data_sha256"] = data["baseline_data_sha256"]
    data_path.write_text(json.dumps(data))
    (report.report / "fingerprint.txt").write_text("new identity")
    (report.report / "summary.md").write_text("No data differences from the committed capture.")
    api = Mock(return_value=[[]])
    monkeypatch.setattr(watch, "github", api)
    watch.publish(report)
    api.assert_not_called()


def test_github_errors_propagate_without_mutation(report, monkeypatch):
    api = Mock(side_effect=RuntimeError("API failed"))
    monkeypatch.setattr(watch, "github", api)
    with pytest.raises(RuntimeError, match="API failed"):
        watch.publish(report)
    assert api.call_count == 1


def test_reading_sheets_match_dictionary_config():
    yaml = pytest.importorskip("yaml")
    config = Path(__file__).parents[1] / "dictionary/sources/official/kautian/config.yaml"
    options = yaml.safe_load(config.read_text(encoding="utf-8"))["stage_options"]
    select = options["select"]
    sheets = {name.removesuffix(".csv"): spec["columns"] for name, spec in select["sheets"].items()}
    # 異用字 is dropped at merge, so it lists under 袂影響拍字.
    assert "異用字.csv" in options["merge"]["exclude"] and "異用字" in watch.SHEET_TOPICS
    del sheets["異用字"]
    assert sheets == {
        name: ["漢字", *columns]
        for name, columns in watch.READING_SHEETS.items()
        if name != select["dialect_sheet"].removesuffix(".csv")
    }
    assert tuple(select["dialect_columns"]) == watch.READING_SHEETS["語音差異"]


def summarize(before, after, *, brief=True):
    differences = watch.compare(before, after)
    return watch.typing_summary(
        watch.readings(before), watch.readings(after), differences, brief=brief
    )


def test_split_pairs_matches_expand_padding():
    assert watch.split_pairs(watch.parts("甲/乙"), "a/b") == [("甲", "a"), ("乙", "b")]
    assert watch.split_pairs(watch.parts("甲"), "a/b") == [("甲", "a"), ("甲", "b")]
    # trace: expand_dataframe pads the shorter multi-value side with its first entry.
    assert watch.split_pairs(watch.parts("甲/乙/丙"), "a/b") == [
        ("甲", "a"),
        ("乙", "b"),
        ("丙", "a"),
    ]
    assert watch.split_pairs(watch.parts(""), "a") == [("", "a")]
    assert watch.split_pairs(watch.parts("甲"), "") == []


def test_typing_summary_lists_pairs_moves_and_side_effects():
    headword = ("詞目id", "漢字", "羅馬字", "分類")
    synonym = ("詞目id", "詞目漢字", "對應詞目id", "對應詞目漢字")
    variant = ("詞目id", "漢字", "異用字")
    before = {
        "詞目": watch.Sheet(
            headword,
            Counter(
                {
                    ("1", "鏡仁", "", ""): 1,
                    ("2", "古", "kóo", ""): 1,
                    ("3", "水濺仔", "", ""): 1,
                }
            ),
        ),
        "又唸作": watch.Sheet(("詞目id", "漢字", "羅馬字"), Counter()),
        "異用字": watch.Sheet(variant, Counter()),
        "詞目tuì詞目近義": watch.Sheet(synonym, Counter({("1", "甲", "2", "乙"): 1})),
    }
    after = {
        "詞目": watch.Sheet(
            headword,
            Counter(
                {("1", "鏡仁", "kiànn-jîn/kiànn-lîn", ""): 1, ("3", "水濺仔", "", "生活用品"): 1}
            ),
        ),
        "又唸作": watch.Sheet(("詞目id", "漢字", "羅馬字"), Counter({("2", "古", "kóo"): 1})),
        "異用字": watch.Sheet(variant, Counter({("4", "這", "即"): 1})),
        "詞目tuì詞目近義": watch.Sheet(synonym, Counter()),
    }
    summary = summarize(before, after)
    # trace: "kiànn-jîn/kiànn-lîn" splits on "/" → two (鏡仁, tl) pairs; 古/kóo
    # leaves 詞目 but stays in 又唸作 → moved, neither new nor removed.
    assert "**增加（2）**" in summary
    assert "| 鏡仁 | kiànn&#45;jîn | 詞目 |" in summary
    assert "| 鏡仁 | kiànn&#45;lîn | 詞目 |" in summary
    assert "**刪掉" not in summary
    assert "**徙位（1）**" in summary
    assert "| 古 | kóo | 詞目 → 又唸作 |" in summary
    assert "- 詞目：分類改 1 項" in summary
    assert "**近義（1）**" in summary and "| 甲 | 乙 | 刪掉 |" in summary
    assert "- 異用字（拍字用固定的 variants.csv）：增加 1" in summary
    assert "⚠" not in summary


def test_typing_summary_warns_on_reading_sheet_header_change():
    old = watch.Sheet(("漢字", "羅馬字"), Counter({("甲", "kah"): 1}))
    new = watch.Sheet(("漢字", "台羅"), Counter({("甲", "kah"): 1}))
    before, after = {"詞目": old}, {"詞目": new}
    summary = summarize(before, after)
    assert "⚠ 欄位名有改：詞目。" in summary


def test_typing_summary_without_reading_changes_and_bounded():
    columns = ("漢字", "羅馬字")
    sheet = watch.Sheet(columns, Counter({("甲", "kah"): 1}))
    assert "拍字無改變。" in summarize({"名": sheet}, {"名": sheet})
    many = watch.Sheet(columns, Counter({(f"字{i}", "lī" * 50): 1 for i in range(2000)}))
    summary = summarize({}, {"名": many}, brief=True)
    assert len(summary) < 2 * watch.SUMMARY_TABLE_CHARS
    assert "請看完整報告。" in summary
    assert summarize({}, {"名": many}, brief=False).count("| 字") == 2000
    columns = tuple(f"欄{i}" for i in range(500))
    edit = watch.Modification(tuple("a" * 500), tuple("b" * 500))
    sheet = watch.SheetDiff("詞目", columns, columns, 1, 1, [], [], [edit])
    effects = watch.side_effects([sheet], brief=True)
    assert len("\n".join(effects)) < watch.SUMMARY_TABLE_CHARS + 100
    assert "請看完整報告。" in effects[-1]


def test_link_changes_pair_headword_and_sense_sheets():
    word = ("詞目id", "詞目漢字", "對應詞目id", "對應詞目漢字")
    sense = ("義項id", "詞目漢字", "解說", "對應詞目id", "對應詞目漢字")
    before = {
        "詞目tuì詞目近義": watch.Sheet(
            word, Counter({("1", "目鏡仁", "2", "鏡仁"): 1, ("3", "水濺仔", "4", "漩桶"): 1})
        ),
        "義項tuì詞目近義": watch.Sheet(sense, Counter()),
        "義項tuì詞目反義": watch.Sheet(sense, Counter()),
    }
    after = {
        "詞目tuì詞目近義": watch.Sheet(word, Counter()),
        "義項tuì詞目近義": watch.Sheet(
            sense, Counter({("9", "目鏡仁", "眼鏡的鏡片。", "2", "鏡仁"): 1})
        ),
        "義項tuì詞目反義": watch.Sheet(sense, Counter({("8", "好", "優良。", "5", "䆀"): 1})),
    }
    tables = watch.link_changes(watch.compare(before, after), brief=True)
    # trace: 目鏡仁→鏡仁 leaves 詞目tuì詞目近義 and appears in 義項tuì詞目近義 → one moved row.
    assert "| 目鏡仁 | 鏡仁 | 規个詞目 → 釋義「眼鏡的鏡片。」 |" in tables[1]
    assert "| 水濺仔 | 漩桶 | 刪掉 |" in tables[1]
    assert "**反義（1）**" in tables[0]
    assert "| 好 | 䆀 | 增加：釋義「優良。」 |" in tables[0]
    assert "近義" not in "\n".join(watch.side_effects(watch.compare(before, after), brief=True))
