/**
 * Persistent Node.js subprocess for taigi-converter bridge.
 *
 * Protocol: reads one JSON line from stdin, writes one JSON line to stdout.
 * Operations: convert, toToneNumber
 */

const { convert, toToneNumber } = require("../../taigi-converter/src/index.js");
const readline = require("readline");

const rl = readline.createInterface({ input: process.stdin });

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
