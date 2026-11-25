// Types corpus sentences word by word, as a user would, in every viewport ×
// candidate layout × candidate text size, and after every word (and after
// browsing the candidates) asserts the candidate window's layout invariants.
// Writes, per combination, a report JSON and a screenshot of each issue kind's
// first occurrence (Chromium also keeps a few clean shots) into the output
// directory; `summary.mjs` folds the reports into `summary.json`.
//
// Environment (VIEWS, LAYOUTS, ROOT_TEXT_SIZE): .claude/skills/layout/SKILL.md.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";
import {
  candidates,
  current,
  open,
  panel,
  preedit,
  setRootTextSize,
  settled,
} from "../e2e/helpers.mjs";
import { SENTENCES } from "./sentences.mjs";

/** @typedef {import("@playwright/test").Page} Page */

/**
 * @typedef {object} View
 * @property {string} name
 * @property {number} width
 * @property {number} height
 * @property {string[]} sizes the candidate text sizes it runs
 * @property {import("./sentences.mjs").Sentence[]} sentences
 */

/** @type {View[]} */
const ALL_VIEWS = [
  {
    name: "desktop",
    width: 1280,
    height: 800,
    sizes: ["13", "17", "23"],
    sentences: SENTENCES,
  },
  {
    name: "phone",
    width: 360,
    height: 800,
    sizes: ["13", "17", "23"],
    sentences: SENTENCES,
  },
  // A 360 × 800 phone at 200 % page zoom: one size and seven sentences (the
  // longest last), since every row is already at its narrowest.
  {
    name: "zoom200",
    width: 180,
    height: 400,
    sizes: ["17"],
    sentences: SENTENCES.slice(0, 6).concat(SENTENCES.slice(-1)),
  },
];
/**
 * Every candidate layout, with the arrows a user browsing it presses: through
 * the list, then into the next row or column.
 * @type {Record<string, string[]>}
 */
const BROWSE_KEYS = {
  vertical: ["ArrowDown", "ArrowDown"],
  horizontal: ["ArrowRight", "ArrowDown"],
  expandable: ["ArrowRight", "ArrowDown"],
  verticalExpandable: ["ArrowDown", "ArrowRight"],
};

/**
 * The names `variable` lists, every one of `known` when it is unset.
 * @param {string} variable
 * @param {string[]} known
 * @returns {string[]}
 */
function pick(variable, known) {
  const value = process.env[variable]?.trim();
  if (!value) {
    return known;
  }
  const names = value.split(/[\s,]+/);
  const unknown = names.filter((name) => !known.includes(name));
  if (unknown.length > 0) {
    throw new Error(
      `${variable}: unknown ${unknown.join(", ")}; known: ${known.join(", ")}`,
    );
  }
  return names;
}

const viewNames = pick(
  "VIEWS",
  ALL_VIEWS.map((view) => view.name),
);
const VIEWS = ALL_VIEWS.filter((view) => viewNames.includes(view.name));
const LAYOUTS = pick("LAYOUTS", Object.keys(BROWSE_KEYS));
const ROOT_TEXT_SIZE = process.env.ROOT_TEXT_SIZE?.trim();

/**
 * Every layout invariant the 2026-11-24 audit's acceptance criteria name, as
 * issue strings `kind` or `kind:text`. Runs in the page, on `#panel`.
 * @param {HTMLElement | SVGElement} root
 * @returns {string[]}
 */
function check(root) {
  /** @type {string[]} */
  const issues = [];
  if (/** @type {HTMLElement} */ (root).hidden) {
    return issues;
  }
  /** @param {string} id */
  const byId = (id) => /** @type {HTMLElement} */ (document.getElementById(id));
  /** @param {Element} element */
  const box = (element) => element.getBoundingClientRect();
  /** @param {string} length a computed length; `normal` reads as 0 */
  const px = (length) => Number.parseFloat(length) || 0;
  const p = box(root);
  const composition = box(byId("preedit"));
  const editor = box(byId("editor"));
  if (
    composition.width > 0 &&
    p.right > composition.x + 1 &&
    p.x < composition.right - 1 &&
    p.bottom > composition.y + 1 &&
    p.y < composition.bottom - 1
  ) {
    issues.push("panel-covers-composition");
  }
  if (p.bottom > editor.bottom + 1) {
    issues.push("panel-past-editor-bottom");
  }
  if (p.right > innerWidth + 0.5 || p.x < -0.5) {
    issues.push("panel-past-viewport");
  }
  const page = document.documentElement;
  if (page.scrollWidth > page.clientWidth) {
    issues.push("page-scrolls-sideways");
  }
  const list = byId("list");
  if (list.scrollWidth > list.clientWidth + 0.5) {
    issues.push("list-scrolls-sideways");
  }
  const ol = byId("candidates");
  if (ol.scrollWidth > ol.clientWidth + 0.5) {
    issues.push("candidates-scroll-sideways");
  }
  const arrows = byId("page-arrows");
  if (!arrows.hidden && box(arrows).right > p.right + 0.5) {
    issues.push("arrows-past-panel");
  }
  // Fractional, unlike `clientWidth`; the list has no border or scrollbar.
  const listWidth = box(ol).width;
  for (const li of ol.querySelectorAll("li")) {
    if (li.scrollHeight > li.clientHeight + 0.5) {
      issues.push(`row-taller-than-box:${li.textContent}`);
    }
    const row = box(li);
    const visible = row.bottom > p.y && row.y < p.bottom;
    const parts = [...li.querySelectorAll(".text, .annotation")];
    const cells = parts.map(box);
    // The widest a candidate could be: the list's width less the row's index,
    // the index's gap and the row's end padding, as if the row alone filled
    // the list and the romanization gave way (`maximum_primary_column_width`
    // in `taigi-desktop-core`). An annotation clipped to nothing still keeps
    // its `--candidate-annotation-gap` padding, which overflows into the end
    // padding rather than giving the candidate room. A wider candidate is cut
    // with "…" by design; a narrower cut one had room.
    const style = getComputedStyle(li);
    const index = /** @type {HTMLElement} */ (li.querySelector(".index"));
    const lead =
      index.offsetWidth > 0
        ? box(index).right -
          row.left +
          px(getComputedStyle(index).marginRight) +
          // `normal` on one-line cells, whose gap is the index's margin.
          px(style.columnGap)
        : px(style.paddingLeft);
    const room = listWidth - lead - px(style.paddingRight);
    for (const [i, part] of parts.entries()) {
      const range = document.createRange();
      range.selectNodeContents(part);
      const text = range.getBoundingClientRect();
      const cell = cells[i];
      if (
        part.classList.contains("text") &&
        text.width > cell.width + 0.5 &&
        text.width <= room
      ) {
        issues.push(`candidate-cut:${part.textContent}`);
      }
      if (!visible || cell.width === 0) {
        continue;
      }
      if (cell.right > p.right + 0.5 || cell.right > row.right + 0.5) {
        issues.push(`script-past-cell:${part.textContent}`);
      }
      if (text.top < row.top - 0.5 || text.bottom > row.bottom + 0.5) {
        issues.push(`script-past-row-height:${part.textContent}`);
      }
    }
  }
  return issues;
}

for (const view of VIEWS) {
  for (const layout of LAYOUTS) {
    for (const size of view.sizes) {
      const combination = `${view.name}-${layout}-${size}`;
      test(`${view.name} ${layout} ${size}`, async ({
        page,
        browserName,
      }, testInfo) => {
        const out = testInfo.project.outputDir;
        mkdirSync(out, { recursive: true });
        /** @type {{ sentence: number, clause: number, word: string, issue: string }[]} */
        const found = [];
        /** @type {Set<string>} */
        const seen = new Set();
        let cleanShots = 0;
        let complete = false;

        /**
         * Runs `check` and records its issues against `word`, screenshotting
         * each kind's first occurrence.
         * @param {number} sentence
         * @param {number} clause
         * @param {string} word
         */
        const record = async (sentence, clause, word) => {
          await settled(page);
          for (const issue of await panel(page).evaluate(check)) {
            const kind = issue.split(":")[0];
            found.push({ sentence, clause, word, issue });
            if (!seen.has(kind)) {
              seen.add(kind);
              await page.screenshot({
                path: join(
                  out,
                  `FAIL-${browserName}-${combination}-${kind}.png`,
                ),
              });
            }
          }
        };

        try {
          await page.setViewportSize({
            width: view.width,
            height: view.height,
          });
          const editor = await open(page, {
            settings: { candidateLayout: layout, candidateTextSize: size },
            docked: false,
          });
          if (ROOT_TEXT_SIZE) {
            await setRootTextSize(page, ROOT_TEXT_SIZE);
          }
          for (const [s, sentence] of view.sentences.entries()) {
            for (const [c, clause] of sentence.entries()) {
              const clauseStart = found.length;
              for (const [w, word] of clause.entries()) {
                await editor.pressSequentially(word);
                await record(s, c, word);
                // Two words in, the user browses the candidates.
                if (w === 1 && (await candidates(page).count()) > 1) {
                  for (const key of BROWSE_KEYS[layout]) {
                    await editor.press(key);
                  }
                  await expect(current(page)).toHaveCount(1);
                  await record(s, c, `${word}+browse`);
                }
              }
              if (
                browserName === "chromium" &&
                cleanShots < 3 &&
                found.length === clauseStart &&
                (c === 0 || sentence.length === 1)
              ) {
                cleanShots++;
                await page.screenshot({
                  path: join(out, `ok-${combination}-s${s}.png`),
                });
              }
              // Space commits the best candidate until the composition is empty.
              for (
                let i = 0;
                i < 12 && (await preedit(page).textContent()) !== "";
                i++
              ) {
                await editor.press("Space");
              }
            }
            await editor.press("Enter");
          }
          complete = true;
        } finally {
          // Also when a step throws, so the summary shows what was found and
          // that the combination did not finish.
          writeFileSync(
            join(out, `report-${browserName}-${combination}.json`),
            `${JSON.stringify(
              {
                browser: browserName,
                view: view.name,
                layout,
                size,
                rootTextSize: ROOT_TEXT_SIZE ?? null,
                complete,
                issues: found,
              },
              null,
              1,
            )}\n`,
          );
        }
        expect(
          found.map((entry) => entry.issue),
          `${combination}: ${[...seen].join(", ")}`,
        ).toEqual([]);
      });
    }
  }
}
