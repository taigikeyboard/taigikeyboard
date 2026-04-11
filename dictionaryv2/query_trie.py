#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
測試 Trie 查詢

用法：
    python query_trie.py <query>
    python query_trie.py ho2
    python query_trie.py gua
    python query_trie.py hb

支援：
- 完全匹配：顯示該 key 對應的所有結果
- 前綴匹配：顯示所有以該前綴開頭的 key
"""

import sys
import sqlite3
import marisa_trie
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
TRIE_FILE = SCRIPT_DIR / "output" / "dictionary.trie"
DB_FILE = SCRIPT_DIR / "output" / "trie.db"


def main():
    if len(sys.argv) < 2:
        print("用法: python query_trie.py <query>")
        print("範例: python query_trie.py ho2")
        return

    query = sys.argv[1].lower()

    # 載入 trie
    try:
        trie = marisa_trie.RecordTrie("<I").mmap(str(TRIE_FILE))
    except Exception as e:
        print(f"Error loading trie: {e}")
        return

    # 連接 SQLite
    try:
        conn = sqlite3.connect(str(DB_FILE))
        cursor = conn.cursor()
    except Exception as e:
        print(f"Error connecting to database: {e}")
        return

    print(f"Query: '{query}'")
    print("=" * 50)

    # 完全匹配
    if query in trie:
        results = trie[query]
        rowids = [r[0] for r in results]
        print(f"\n[完全匹配] {len(rowids)} 筆結果")
        print("-" * 50)

        # 查詢 SQLite
        placeholders = ",".join("?" * len(rowids))
        cursor.execute(f"""
            SELECT id, tl_num, tl_notone, tl_abbrev
            FROM dictionary
            WHERE id IN ({placeholders})
            LIMIT 20
        """, rowids)

        for row in cursor.fetchall():
            id_, tl_num, tl_notone, tl_abbrev = row
            parts = [f"num={tl_num}"]
            if tl_notone:
                parts.append(f"notone={tl_notone}")
            if tl_abbrev:
                parts.append(f"abbrev={tl_abbrev}")
            print(f"  [{id_}] {', '.join(parts)}")

        if len(rowids) > 20:
            print(f"  ... 還有 {len(rowids) - 20} 筆")

    # 前綴匹配
    prefix_keys = trie.keys(query)
    if prefix_keys:
        # 取得所有 rowid
        all_rowids = set()
        for key in prefix_keys:
            for r in trie[key]:
                all_rowids.add(r[0])

        print(f"\n[前綴匹配] {len(prefix_keys)} 個 key, {len(all_rowids)} 筆不重複結果")
        print("-" * 50)

        # 顯示部分 key
        print(f"  Keys: {prefix_keys[:10]}")
        if len(prefix_keys) > 10:
            print(f"  ... 還有 {len(prefix_keys) - 10} 個 key")

        # 查詢 SQLite（限制 20 筆）
        rowids_list = list(all_rowids)[:100]
        placeholders = ",".join("?" * len(rowids_list))
        cursor.execute(f"""
            SELECT id, tl_num, tl_notone, tl_abbrev
            FROM dictionary
            WHERE id IN ({placeholders})
            LIMIT 20
        """, rowids_list)

        print(f"\n  [Top 20]")
        for row in cursor.fetchall():
            id_, tl_num, tl_notone, tl_abbrev = row
            parts = [f"num={tl_num}"]
            if tl_notone:
                parts.append(f"notone={tl_notone}")
            if tl_abbrev:
                parts.append(f"abbrev={tl_abbrev}")
            print(f"    [{id_}] {', '.join(parts)}")

    if query not in trie and not prefix_keys:
        print("  找不到結果")

    conn.close()


if __name__ == "__main__":
    main()
