// Part of `make chromeos`: writes the staged options page, the web page's
// settings form inside web/chromeos/options.html.
//   node scripts/stage-chromeos.mjs <web/src/index.html> <web/chromeos/options.html> <out>

import { readFileSync, writeFileSync } from "node:fs";

import { optionsPage } from "../chromeos/options-form.mjs";

const [index, shell, out] = process.argv.slice(2);
writeFileSync(
  out,
  optionsPage(readFileSync(shell, "utf8"), readFileSync(index, "utf8")),
);
