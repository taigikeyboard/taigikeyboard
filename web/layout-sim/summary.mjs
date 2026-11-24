// Global teardown of the layout simulation: folds every `report-*.json` the
// spec wrote into `summary.json` — browser → combination → issue kind → count,
// `{}` for a clean combination, `incomplete: 1` for one that stopped early —
// and prints the combinations with issues.

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * @typedef {object} Report
 * @property {string} browser
 * @property {string} view
 * @property {string} layout
 * @property {string} size
 * @property {boolean} complete false when a step threw before the last sentence
 * @property {{ issue: string }[]} issues
 */

/** @param {import("@playwright/test").FullConfig} config */
export default function summarize(config) {
  // Every project shares the one outputDir.
  const out = config.projects[0].outputDir;
  /** @type {Record<string, Record<string, Record<string, number>>>} */
  const summary = {};
  const reports = readdirSync(out).filter(
    (name) => name.startsWith("report-") && name.endsWith(".json"),
  );
  for (const name of reports.sort()) {
    const report = /** @type {Report} */ (
      JSON.parse(readFileSync(join(out, name), "utf8"))
    );
    /** @type {Record<string, number>} */
    const kinds = {};
    if (!report.complete) {
      kinds.incomplete = 1;
    }
    for (const { issue } of report.issues) {
      const kind = issue.split(":")[0];
      kinds[kind] = (kinds[kind] ?? 0) + 1;
    }
    summary[report.browser] ??= {};
    summary[report.browser][`${report.view} ${report.layout} ${report.size}`] =
      kinds;
  }
  writeFileSync(
    join(out, "summary.json"),
    `${JSON.stringify(summary, null, 2)}\n`,
  );
  console.log(
    `\nlayout-sim: ${reports.length} combinations → ${join(out, "summary.json")}`,
  );
  for (const [browser, combinations] of Object.entries(summary)) {
    for (const [combination, kinds] of Object.entries(combinations)) {
      const counts = Object.entries(kinds).map(([kind, n]) => `${kind} ×${n}`);
      if (counts.length > 0) {
        console.log(`  ${browser} ${combination}: ${counts.join(", ")}`);
      }
    }
  }
}
