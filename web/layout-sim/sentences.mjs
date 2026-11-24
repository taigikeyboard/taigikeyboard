// The sentences the layout simulation types: real example sentences from the
// corpus submodule, read at run time (their licence allows quoting, not
// rewriting, so no copy lives in this repository).

import { readFileSync } from "node:fs";

const CORPUS = new URL(
  "../../corpus/taigi-typing/exampleSentences.js",
  import.meta.url,
);

/**
 * One sentence as clauses → words, each word the keys a user types for it:
 * toneless, no hyphen, lower case.
 * @typedef {string[][]} Sentence
 */

/**
 * Every corpus sentence's Tâi-lô.
 * @returns {string[]}
 */
function readCorpus() {
  let source;
  try {
    source = readFileSync(CORPUS, "utf8");
  } catch {
    throw new Error(
      "corpus/taigi-typing is missing: run `git submodule update --init corpus/taigi-typing`",
    );
  }
  const tailo = [...source.matchAll(/"tailo": "([^"]+)"/g)].map(
    (match) => match[1],
  );
  if (tailo.length === 0) {
    throw new Error(
      `no "tailo" entries in ${CORPUS.pathname}: format changed?`,
    );
  }
  return tailo;
}

/**
 * The keys typed for one Tâi-lô word: tone marks and hyphens dropped,
 * superscript nasal as `nn`.
 * @param {string} word
 * @returns {string}
 */
const keysOf = (word) =>
  word
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[ⁿᴺ]/g, "nn")
    .replace(/[^A-Za-z]/g, "")
    .toLowerCase();

const all = readCorpus();
// A fixed pick, so runs compare: every 600th sentence plus the longest one.
const picked = all.filter((_, i) => i % 600 === 7);
picked.push(
  all.reduce((longest, tailo) =>
    tailo.length > longest.length ? tailo : longest,
  ),
);

/** @type {Sentence[]} */
export const SENTENCES = picked.map((tailo) =>
  tailo
    .split(/[,.?!;:，。？！；：「」『』"“”]+/)
    .map((clause) => clause.trim().split(/\s+/).map(keysOf).filter(Boolean))
    .filter((words) => words.length > 0),
);
