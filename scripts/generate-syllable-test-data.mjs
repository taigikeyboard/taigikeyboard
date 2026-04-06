import { readFileSync, writeFileSync } from "node:fs";
import { parseSyllable, stripToneMark } from "../taigi-converter/src/phonetics.js";
import { toZhuyin } from "../taigi-converter/src/zhuyin.js";
import { toPoj } from "../taigi-converter/src/poj.js";

const DICT_PATH = new URL("../dictionary/aiongtaigi-dictionary.csv", import.meta.url);
const OUT_PATH = new URL("../android/app/src/test/resources/syllable-test-data.csv", import.meta.url);

const csv = readFileSync(DICT_PATH, "utf-8");
const lines = csv.split("\n").slice(1); // skip header

const uniqueSyllables = new Set();
const skipped = [];

for (const line of lines) {
  if (!line.trim()) continue;
  const match = line.match(/^"([^"]+)"/);
  if (!match) continue;
  // Strip control characters (U+0003, U+0006, etc.) and trailing whitespace
  const roman = match[1].replace(/[\x00-\x1f\x7f]/g, "").trim();

  // Split word into syllables (space = word boundary, hyphen = syllable boundary)
  const words = roman.split(/\s+/);
  for (const word of words) {
    const syllables = word.split("-");
    for (const syl of syllables) {
      if (!syl) continue;
      try {
        const [initial, final, tone] = parseSyllable(syl);
        const tlNumeric = initial + final + tone;
        uniqueSyllables.add(tlNumeric);
      } catch {
        skipped.push(syl);
      }
    }
  }
}

// Deduplicate skipped
const uniqueSkipped = [...new Set(skipped)];
if (uniqueSkipped.length > 0) {
  console.log(`Skipped ${uniqueSkipped.length} unparseable syllables:`);
  console.log(uniqueSkipped.slice(0, 20).join(", ") + (uniqueSkipped.length > 20 ? " ..." : ""));
}

// Generate reference data
const rows = [];
const errors = [];

for (const tlNumeric of [...uniqueSyllables].sort()) {
  try {
    const [initial, final, tone] = parseSyllable(tlNumeric);

    // Skip tone 1 (TPS) and tone 4 (POJ): both have no explicit tone marker,
    // pipeline produces toneless form which works via trie prefix search.
    // Tone 4 IS testable for TPS (entering-tone symbols encode it), so we
    // keep tone 4 but mark POJ as untestable via a flag.
    if (tone === "1") continue;
    const pojTestable = tone !== "4";

    const tps = toZhuyin(tlNumeric, { encodeSafe: true }).trimEnd();
    if (!tps) { errors.push(`${tlNumeric}: empty TPS`); continue; }

    const pojDisplay = toPoj(initial, final, tone);

    // Generate poj_numeric: simulate InputNormalizer.normalize(pojDisplay, POJ)
    // Order must match Kotlin: ⁿ→nn, NFD, U+0358→"o", extract tone, strip marks
    const pojLower = pojDisplay.toLowerCase()
      .replaceAll("\u207f", "nn")
      .replaceAll("\u1d3a", "nn");
    const pojNfd = pojLower.normalize("NFD")
      .replaceAll("\u0358", "o"); // POJ o͘ dot → extra "o" (BEFORE stripping marks)

    // Extract tone mark and strip all combining marks
    const toneMap = { '\u0301': '2', '\u0300': '3', '\u0302': '5', '\u030c': '6', '\u0304': '7', '\u030d': '8', '\u0306': '9' };
    let pojTone = "";
    let pojBare = "";
    for (const ch of pojNfd) {
      if (toneMap[ch]) { pojTone = toneMap[ch]; }
      else if (ch.charCodeAt(0) >= 0x0300 && ch.charCodeAt(0) <= 0x036f) { /* skip other combining */ }
      else { pojBare += ch; }
    }
    const pojNumeric = pojTestable ? (pojBare + pojTone) : "";

    rows.push(`${tlNumeric},${tps},${pojDisplay},${pojNumeric}`);
  } catch (e) {
    errors.push(`${tlNumeric}: ${e.message}`);
  }
}

if (errors.length > 0) {
  console.log(`\n${errors.length} generation errors:`);
  errors.forEach(e => console.log("  " + e));
}

const output = "tl_numeric,tps,poj_display,poj_numeric\n" + rows.join("\n") + "\n";
writeFileSync(OUT_PATH, output, "utf-8");

console.log(`\nGenerated ${rows.size ?? rows.length} syllable test rows -> ${OUT_PATH.pathname}`);
