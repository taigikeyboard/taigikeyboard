/**
 * Persistent Node.js subprocess for taigi-converter bridge.
 *
 * Protocol: reads one JSON line from stdin, writes one JSON line to stdout.
 * Operations: convert, toToneNumber, validate
 */

import { convert, toToneNumber } from "../../taigi-converter/src/converter.js";
import { parseSyllable } from "../../taigi-converter/src/phonetics.js";
import { createInterface } from "readline";

const rl = createInterface({ input: process.stdin });

rl.on("line", (line) => {
  try {
    const req = JSON.parse(line);
    let result;
    switch (req.op) {
      case "convert":
        result = convert(req.text, req.source, req.target);
        break;
      case "toToneNumber":
        result = toToneNumber(req.text, req.system || "tl");
        break;
      case "validate": {
        const syllables = req.text.split(/[\s-]+/).filter(Boolean);
        let valid = syllables.length > 0;
        for (const s of syllables) {
          try {
            parseSyllable(s);
          } catch {
            valid = false;
            break;
          }
        }
        result = valid;
        break;
      }
      default:
        result = req.text;
    }
    process.stdout.write(JSON.stringify({ result }) + "\n");
  } catch (e) {
    process.stdout.write(
      JSON.stringify({ result: null, error: e.message }) + "\n",
    );
  }
});
