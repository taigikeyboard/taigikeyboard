//! Build subcommand: read stdin TL\tkey\trowid lines, emit byte-sorted fst.
//!
//! Wire format (mirrors `docs/engine/binary-format.md` § dictionary.fst):
//!   key_bytes (UTF-8) || 0xFF || rowid_le_4
//!
//! `0xFF` separates key from rowid because UTF-8 byte classes never produce
//! `0xFF` mid-sequence. Multiple rowids per key appear as multiple entries.

// build 子命令,讀取 stdin 的 TL\tkey\trowid 行,排序去重後輸出 fst 檔。
// 用 0xFF 當分隔位元,因為合法 UTF-8 序列絕不會產生 0xFF。

use std::fs::File;
use std::io::{self, BufRead, BufWriter};

// key 與 rowid 的分隔位元,UTF-8 序列不會出現 0xFF 故可安全當分界。
const SEPARATOR: u8 = 0xFF;

// 建置完成後回報的統計資料,供 main 印出 log 行使用。
pub struct BuildStats {
    pub entries: usize,
    pub distinct_keys: usize,
}

// 主建置流程,讀 stdin 全部行、組成 key+0xFF+rowid 條目、排序去重後寫入 fst。
pub(crate) fn run_build(output_path: &str) -> Result<BuildStats, String> {
    let stdin = io::stdin();
    let mut entries: Vec<Vec<u8>> = Vec::new();
    let mut distinct_keys = std::collections::HashSet::new();
    let mut line_no: usize = 0;

    for line_res in stdin.lock().lines() {
        line_no += 1;
        let line = line_res.map_err(|e| format!("line {}: read error: {}", line_no, e))?;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let marker = fields
            .next()
            .ok_or_else(|| format!("line {}: missing marker field", line_no))?;
        if marker != "TL" {
            return Err(format!(
                "line {}: unsupported marker `{}` (only `TL` accepted in v3.5.6)",
                line_no, marker
            ));
        }
        let key = fields
            .next()
            .ok_or_else(|| format!("line {}: missing key field", line_no))?;
        let rowid_str = fields
            .next()
            .ok_or_else(|| format!("line {}: missing rowid field", line_no))?;
        if fields.next().is_some() {
            return Err(format!("line {}: too many fields", line_no));
        }
        let rowid: u32 = rowid_str
            .parse()
            .map_err(|e| format!("line {}: rowid `{}` parse error: {}", line_no, rowid_str, e))?;

        let key_bytes = key.as_bytes();
        if key_bytes.contains(&SEPARATOR) {
            return Err(format!(
                "line {}: key `{}` contains 0xFF separator byte",
                line_no, key
            ));
        }
        distinct_keys.insert(key.to_string());

        let mut entry = Vec::with_capacity(key_bytes.len() + 1 + 4);
        entry.extend_from_slice(key_bytes);
        entry.push(SEPARATOR);
        entry.extend_from_slice(&rowid.to_le_bytes());
        entries.push(entry);
    }

    if entries.is_empty() {
        return Err("no input pairs received on stdin".to_string());
    }

    entries.sort_unstable();
    entries.dedup();

    let file =
        File::create(output_path).map_err(|e| format!("create output `{}`: {}", output_path, e))?;
    let writer = BufWriter::new(file);
    let mut builder =
        fst::SetBuilder::new(writer).map_err(|e| format!("fst SetBuilder init: {}", e))?;

    for entry in &entries {
        builder
            .insert(entry)
            .map_err(|e| format!("fst insert: {}", e))?;
    }

    builder.finish().map_err(|e| format!("fst finish: {}", e))?;

    Ok(BuildStats {
        entries: entries.len(),
        distinct_keys: distinct_keys.len(),
    })
}
