// Part of `make extension`: writes what the browser extension cuts from the
// web page at build time, so both keep one markup and one stylesheet. A page
// that drifted fails the build rather than shipping a wrong panel.
//   node scripts/stage-extension.mjs <web/src> <web/extension> <out>

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { optionsPage } from "../chromeos/options-form.mjs";

/** @typedef {import("../chromeos/options-form.mjs").OptionsCut} OptionsCut */

/**
 * Rows with no effect in the extension: it has no key panel, and it draws
 * in the site's own fonts.
 *
 * @type {OptionsCut}
 */
export const EXTENSION_CUT = {
  hiddenControls: ["tpsKeyboardShown", "fontType"],
  removedOptions: [],
};

/**
 * The page's composition, candidate panel, ruler and live region, as they
 * stand in `index.html`: the elements `Panel` and the stylesheet name.
 *
 * @param {string} index
 * @returns {string}
 */
export function panelMarkup(index) {
  const lines = index.split("\n");
  /**
   * @param {string} needle
   * @returns {number}
   */
  const only = (needle) => {
    const found = lines.flatMap((line, at) =>
      line.includes(needle) ? [at] : [],
    );
    if (found.length !== 1) {
      throw new Error(
        `panel markup: expected one line with ${needle}, found ${found.length}`,
      );
    }
    return found[0];
  };
  const start = only('<div id="preedit">');
  const end = only('<div id="ruler"');
  const spoken = only('<div id="spoken"');
  if (end <= start) {
    throw new Error("panel markup: #ruler is not after #preedit");
  }
  return [...lines.slice(start, end + 1), lines[spoken]]
    .map((line) => line.trim())
    .join("\n");
}

/**
 * The page's stylesheet for a shadow tree: what the page sets on `:root`,
 * the overlay's host carries.
 *
 * @param {string} css
 * @returns {string}
 */
export function forShadowTree(css) {
  const rooted = /:root((?:\[[^\]]*\]|:not\([^)]*\))*)/g;
  if (!rooted.test(css)) {
    throw new Error("overlay styles: no :root rule in style.css");
  }
  return css.replace(rooted, (_, condition) =>
    condition ? `:host(${condition})` : ":host",
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [source, extension, out] = process.argv.slice(2);
  /**
   * @param {...string} path
   * @returns {string}
   */
  const read = (...path) => readFileSync(join(...path), "utf8");
  const index = read(source, "index.html");

  writeFileSync(
    join(out, "options.html"),
    optionsPage(read(extension, "options.html"), index, EXTENSION_CUT),
  );

  mkdirSync(join(out, "generated"), { recursive: true });
  writeFileSync(
    join(out, "generated", "panel-markup.js"),
    `export const PANEL_MARKUP = ${JSON.stringify(panelMarkup(index))};\n`,
  );

  writeFileSync(
    join(out, "overlay.css"),
    `${forShadowTree(read(source, "style.css"))}\n${read(extension, "overlay.css")}`,
  );
}
