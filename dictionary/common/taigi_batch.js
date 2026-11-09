/**
 * Persistent Node.js subprocess for taigi-converter bridge.
 *
 * Protocol: reads one JSON line from stdin, writes one JSON line to stdout.
 * Operations: convert, toToneNumber, toToneNumberAscii, validate
 *
 * Single form: {"op", "text", ...} → {"result"} or {"result": null, "error"}.
 * List form:   {"op", "texts": [...], ...} → {"results": [...]}, one reply per
 * text, each shaped exactly as the single form would answer that text.
 */

import { convert, toToneNumber, toToneNumberAscii } from "../../taigi-converter/src/converter.js";
import { parseSyllable } from "../../taigi-converter/src/phonetics.js";
import { createInterface } from "readline";

const rl = createInterface({ input: process.stdin });

function run(req, text) {
  switch (req.op) {
    case "convert":
      return convert(text, req.source, req.target);
    case "toToneNumber":
      return toToneNumber(text);
    case "toToneNumberAscii":
      return toToneNumberAscii(text);
    case "validate": {
      const syllables = text.split(/[\s-]+/).filter(Boolean);
      let valid = syllables.length > 0;
      for (const s of syllables) {
        try {
          parseSyllable(s);
        } catch {
          valid = false;
          break;
        }
      }
      return valid;
    }
    default:
      return text;
  }
}

function reply(req, text) {
  try {
    return { result: run(req, text) };
  } catch (e) {
    return { result: null, error: e.message };
  }
}

rl.on("line", (line) => {
  let out;
  try {
    const req = JSON.parse(line);
    out = Array.isArray(req.texts)
      ? { results: req.texts.map((text) => reply(req, text)) }
      : reply(req, req.text);
  } catch (e) {
    out = { result: null, error: e.message };
  }
  process.stdout.write(JSON.stringify(out) + "\n");
});
