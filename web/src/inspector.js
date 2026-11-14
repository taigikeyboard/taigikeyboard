// The open composition, drawn as the engine cut it and ordered it. The
// figures come from the engine (`explain`); this file only lays them out.
// The writing page does not load it. `dev.html` does.

import { translate } from "./i18n.js";

/** @typedef {import("../types/shared.d.ts").ExplainBufferPiece} ExplainBufferPiece */
/** @typedef {import("../types/shared.d.ts").ExplainCandidatePiece} ExplainCandidatePiece */
/** @typedef {import("../types/shared.d.ts").ExplainView} ExplainView */
/** @typedef {import("../types/shared.d.ts").ExplainWordPiece} ExplainWordPiece */
/** @typedef {import("./i18n.js").StringKey} StringKey */

/** How many comparison cards to draw before the next click. */
const PAGE = 8;

/** @type {Array<[string, StringKey, StringKey]>} */
const CANONICAL = [
  ["tier", "web.inspectorStageTier", "web.inspectorDetailTier"],
  [
    "abbreviation",
    "web.inspectorStageAbbreviation",
    "web.inspectorDetailAbbreviation",
  ],
  [
    "preference_class",
    "web.inspectorStagePreference",
    "web.inspectorDetailPreference",
  ],
  [
    "coverage_kind",
    "web.inspectorStageCoverageKind",
    "web.inspectorDetailCoverage",
  ],
  [
    "user_weight",
    "web.inspectorStageUserWeight",
    "web.inspectorDetailUserWeight",
  ],
  ["context_rank", "web.inspectorStageContext", "web.inspectorDetailContext"],
  [
    "walker_cost",
    "web.inspectorStageWalkerCost",
    "web.inspectorDetailWalkerCost",
  ],
  ["score", "web.inspectorStageScore", "web.inspectorDetailScore"],
  ["frequency", "web.inspectorStageFrequency", "web.inspectorDetailFrequency"],
  [
    "coverage",
    "web.inspectorStageCoverageBytes",
    "web.inspectorDetailCoverageBytes",
  ],
  ["source_rank", "web.inspectorStageSource", "web.inspectorDetailSource"],
  ["stable_idx", "web.inspectorStageStable", "web.inspectorDetailStable"],
];

/**
 * The order the cards and the mechanism list draw. The engine compares in
 * this same order after the page installs it.
 */
let stages = CANONICAL.map((stage) => stage);

/** @returns {string[]} */
export function defaultSortFields() {
  return CANONICAL.map(([id]) => id);
}

/**
 * @param {string[]} ids
 * @returns {void}
 */
export function setSortFieldOrder(ids) {
  const next = ids
    .map((id) => CANONICAL.find(([field]) => field === id))
    .filter(
      /** @returns {stage is [string, StringKey, StringKey]} */ (stage) =>
        Boolean(stage),
    );
  if (next.length === CANONICAL.length) {
    stages = next;
  }
}

/** @type {Record<string, StringKey>} */
const SOURCE_KEYS = {
  kautian: "common.moeDict",
  taigitv: "common.newwordDict",
  itaigi: "common.iTaigiDict",
  sitbut: "common.taiwanPlantDict",
  taihoa: "common.taiHuaDict",
  taijit: "common.taiwanJapanDict",
  kungge: "common.kunggeDict",
  stti: "common.sttiDict",
  khpoo: "common.accentDict",
  khiin: "dictionary.khiin",
  dev: "dictionary.devSupplementDict",
  lkk: "dictionary.lkkDict",
  custom: "web.inspectorKindCustom",
  other: "web.inspectorSourceOther",
};

/** @type {Record<string, StringKey>} */
const PREFERENCE = {
  picked: "web.inspectorPrefPicked",
  custom: "web.inspectorPrefCustom",
  rest: "web.inspectorPrefRest",
};

/** @type {Record<string, StringKey>} */
const COVERAGE = {
  full: "web.inspectorCovFull",
  prefix: "web.inspectorCovPrefix",
  abbreviation: "web.inspectorAbbreviation",
};

/** @type {Record<string, StringKey>} */
const CONTEXT = {
  learned: "web.inspectorContextLearned",
  bundled: "web.inspectorContextBundled",
  none: "web.inspectorContextNone",
};

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * @template {keyof HTMLElementTagNameMap} K
 * @param {K} tag
 * @param {string | null} [className]
 * @param {string | null} [text]
 * @returns {HTMLElementTagNameMap[K]}
 */
function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) {
    node.className = className;
  }
  if (text != null) {
    node.textContent = text;
  }
  return node;
}

/**
 * @param {string} id
 * @returns {string}
 */
function sourceLabel(id) {
  const key = SOURCE_KEYS[id];
  return key ? translate(key) : id;
}

/**
 * @param {number} value
 * @param {number} digits
 * @returns {string}
 */
function formatNumber(value, digits) {
  if (!Number.isFinite(value)) {
    return "—";
  }
  return String(Number(value.toFixed(digits)));
}

/**
 * @param {number} picks
 * @param {number} lastUsedMs
 * @returns {string}
 */
function picksText(picks, lastUsedMs) {
  if (!(picks > 0)) {
    return translate("web.inspectorNever");
  }
  if (lastUsedMs > 0) {
    const days = Math.round((Date.now() - lastUsedMs) / DAY_MS);
    if (days >= 1) {
      return translate("web.inspectorPicksAgo", {
        count: String(picks),
        days: String(days),
      });
    }
  }
  return translate("web.inspectorPicks", { count: String(picks) });
}

/**
 * @param {ExplainCandidatePiece} row
 * @param {string} id
 * @returns {string}
 */
function stageText(row, id) {
  switch (id) {
    case "tier":
      return translate(
        row.tier === "whole"
          ? "web.inspectorTierWhole"
          : "web.inspectorTierFront",
      );
    case "abbreviation":
      return translate(
        row.coverage === "abbreviation"
          ? "web.inspectorAbbreviation"
          : "web.inspectorReading",
      );
    case "preference_class":
      return translate(PREFERENCE[row.preference] ?? "web.inspectorPrefRest");
    case "coverage_kind":
      return translate(COVERAGE[row.coverage] ?? "web.inspectorCovFull");
    case "user_weight":
      return formatNumber(row.weight, 4);
    case "context_rank":
      return translate(CONTEXT[row.context] ?? "web.inspectorContextNone");
    case "walker_cost":
      return row.walkerCost == null
        ? translate("web.inspectorUnpriced")
        : String(row.walkerCost);
    case "score":
      return formatNumber(row.score, Math.abs(row.score) >= 100 ? 1 : 2);
    case "frequency":
      return String(row.frequency);
    case "coverage":
      return String(row.coverageBytes);
    case "source_rank":
      return row.sources.length === 0
        ? `${row.sourceRank} · ${translate("web.inspectorNoSource")}`
        : `${row.sourceRank} · ${sourceLabel(row.rankingSource)}`;
    default:
      return "·";
  }
}

/**
 * @param {ExplainCandidatePiece} row
 * @param {string} id
 * @returns {string}
 */
function stageTitle(row, id) {
  switch (id) {
    case "user_weight":
      return String(row.weight);
    case "score":
      return String(row.score);
    case "walker_cost":
      return row.walkerCost == null ? "" : String(row.walkerCost);
    case "frequency":
      return String(row.frequency);
    case "coverage":
      return String(row.coverageBytes);
    case "source_rank":
      return String(row.sourceRank);
    default:
      return "";
  }
}

/**
 * Lower means the row sorts earlier. Matches `CandidateSortKey`.
 *
 * @param {ExplainCandidatePiece} row
 * @param {string} id
 * @returns {number}
 */
function sortRank(row, id) {
  switch (id) {
    case "tier":
      return row.tier === "whole" ? 0 : 1;
    case "abbreviation":
      return row.coverage === "abbreviation" ? 1 : 0;
    case "preference_class":
      return { picked: 0, custom: 1, rest: 2 }[row.preference] ?? 2;
    case "coverage_kind":
      return row.coverage === "prefix" ? 1 : 0;
    case "user_weight":
      return -row.weight;
    case "context_rank":
      return { learned: 0, bundled: 1, none: 2 }[row.context] ?? 2;
    case "walker_cost":
      return row.walkerCost == null ? Number.POSITIVE_INFINITY : row.walkerCost;
    case "score":
      return -row.score;
    case "frequency":
      return -row.frequency;
    case "coverage":
      return -row.coverageBytes;
    case "source_rank":
      return row.sourceRank;
    default:
      return 0;
  }
}

/**
 * @param {ExplainCandidatePiece} row
 * @param {ExplainCandidatePiece} previous
 * @param {string} id
 * @returns {boolean}
 */
function currentBeats(row, previous, id) {
  return sortRank(row, id) < sortRank(previous, id);
}

/** @typedef {{ row: ExplainCandidatePiece, index: number }} IndexedCandidate */

/**
 * @param {IndexedCandidate} entry
 * @param {IndexedCandidate | null} previous
 * @param {IndexedCandidate | null} next
 * @returns {string}
 */
function whyScored(entry, previous, next) {
  const { row } = entry;
  if (row.placedBy === "picked_over_sentence") {
    return translate("web.inspectorPickedOver");
  }
  if (!previous || row.placedBy === "lead") {
    const stage = next && CANONICAL.find(([id]) => id === next.row.placedBy);
    if (stage && next && currentBeats(next.row, row, stage[0])) {
      return translate("web.inspectorPinnedAhead", {
        index: String(next.index + 1),
        reason: translate(stage[1]),
      });
    }
    return translate("web.inspectorLead");
  }
  const stage = CANONICAL.find(([id]) => id === row.placedBy);
  const reason = translate(stage ? stage[1] : "web.inspectorReasonStable");
  const key =
    stage && currentBeats(row, previous.row, stage[0])
      ? "web.inspectorBeatsRow"
      : "web.inspectorLostTo";
  return translate(key, {
    index: String(previous.index + 1),
    reason,
  });
}

/**
 * @param {ExplainCandidatePiece} row
 * @returns {string}
 */
function whyPinned(row) {
  switch (row.placedBy) {
    case "sentence_after":
      return translate("web.inspectorSentenceAfter");
    case "literal":
      return translate("web.inspectorLiteral");
    default:
      return translate("web.inspectorSentence");
  }
}

/**
 * @param {ExplainCandidatePiece} row
 * @returns {HTMLParagraphElement}
 */
function coverageBar(row) {
  const bar = element("p", "coverage");
  if (row.before) {
    bar.append(element("span", "dim", row.before));
  }
  bar.append(element("span", "head", row.head || "·"));
  if (row.rest) {
    bar.append(element("span", "dim", row.rest));
  }
  const label =
    row.before === "" && row.rest === ""
      ? "web.inspectorCoversAll"
      : "web.inspectorCoversFront";
  bar.append(element("span", "coverage-label", translate(label)));
  return bar;
}

/**
 * @param {ExplainCandidatePiece} row
 * @returns {HTMLDivElement}
 */
function sourceBlock(row) {
  const block = element("div", "sources");
  block.append(
    element("span", "sources-label", translate("web.inspectorSources")),
  );
  if (row.sources.length === 0) {
    block.append(element("span", "chip", translate("web.inspectorNoSource")));
    return block;
  }
  for (const id of row.sources) {
    const chip = element(
      "span",
      id === row.rankingSource ? "chip rank" : "chip",
      sourceLabel(id),
    );
    if (id === row.rankingSource) {
      chip.title = translate("web.inspectorRank", {
        rank: String(row.sourceRank),
      });
    }
    block.append(chip);
  }
  block.append(
    element(
      "span",
      "rank-num",
      translate("web.inspectorRank", { rank: String(row.sourceRank) }),
    ),
  );
  return block;
}

/**
 * @param {HTMLDListElement} line
 * @param {StringKey} labelKey
 * @param {string} value
 * @returns {void}
 */
function fact(line, labelKey, value) {
  const item = element("div");
  item.append(element("dt", null, translate(labelKey)));
  item.append(element("dd", null, value));
  line.append(item);
}

/**
 * @param {ExplainCandidatePiece} row
 * @returns {HTMLDListElement}
 */
function facts(row) {
  const line = element("dl", "facts");
  fact(
    line,
    "web.inspectorStageScore",
    formatNumber(row.score, Math.abs(row.score) >= 100 ? 1 : 2),
  );
  fact(line, "web.inspectorStageFrequency", String(row.frequency));
  fact(
    line,
    "web.inspectorStageWalkerCost",
    row.walkerCost == null
      ? translate("web.inspectorUnpriced")
      : String(row.walkerCost),
  );
  fact(line, "web.inspectorStageUserWeight", formatNumber(row.weight, 4));
  line.append(element("p", "picks", picksText(row.picks, row.lastUsedMs)));
  return line;
}

/**
 * @param {string} existing
 * @param {"win" | "lose"} mark
 * @returns {string}
 */
function paint(existing, mark) {
  const kept = existing
    .split(" ")
    .filter((part) => part === "win" || part === "lose");
  if (!kept.includes(mark)) {
    kept.push(mark);
  }
  return kept.join(" ");
}

/**
 * Stages before `placedBy` tied. That stage is `win` on the row whose value
 * sorts earlier and `lose` on the other. Later stages were not compared.
 *
 * @param {IndexedCandidate[]} scored
 * @returns {string[][]}
 */
function marksFor(scored) {
  const marks = scored.map(() => stages.map(() => "base"));
  for (const [index, { row }] of scored.entries()) {
    const at = stages.findIndex(([id]) => id === row.placedBy);
    if (at < 0 || index === 0) {
      continue;
    }
    const stageId = stages[at][0];
    const won = currentBeats(row, scored[index - 1].row, stageId);
    marks[index] = stages.map((_, stage) => {
      if (stage < at) {
        return "tie";
      }
      if (stage > at) {
        return "skip";
      }
      return won ? "win" : "lose";
    });
    marks[index - 1][at] = paint(marks[index - 1][at], won ? "lose" : "win");
  }
  return marks;
}

/**
 * @param {ExplainCandidatePiece} row
 * @param {string[]} marks
 * @returns {HTMLOListElement}
 */
function pipeline(row, marks) {
  const list = element("ol", "pipeline");
  for (const [stage, [id, label]] of stages.entries()) {
    const mark = marks[stage];
    const item = element("li", `step ${mark}`);
    item.append(element("small", null, `${stage + 1} ${translate(label)}`));
    const value = element("b", null, stageText(row, id));
    const title = stageTitle(row, id);
    if (title) {
      value.title = title;
    }
    item.append(value);
    if (mark.split(" ").includes("win") && !mark.split(" ").includes("lose")) {
      item.append(element("em", null, translate("web.inspectorDecided")));
    }
    list.append(item);
  }
  return list;
}

/**
 * @param {IndexedCandidate} entry
 * @param {IndexedCandidate} previous
 * @param {string} stageId
 * @returns {HTMLParagraphElement}
 */
function versus(entry, previous, stageId) {
  const currentWon = currentBeats(entry.row, previous.row, stageId);
  const winnerEntry = currentWon ? entry : previous;
  const loserEntry = currentWon ? previous : entry;
  const line = element("p", "versus");
  const winner = element("span", "versus-win");
  winner.append(
    element(
      "small",
      null,
      translate("web.inspectorPosition", {
        index: String(winnerEntry.index + 1),
      }),
    ),
  );
  winner.append(element("b", null, stageText(winnerEntry.row, stageId)));
  const loser = element("span", "versus-lose");
  loser.append(
    element(
      "small",
      null,
      translate("web.inspectorPosition", {
        index: String(loserEntry.index + 1),
      }),
    ),
  );
  loser.append(element("b", null, stageText(loserEntry.row, stageId)));
  line.append(winner);
  const stageEntry = stages.find(([id]) => id === stageId);
  line.append(
    element(
      "span",
      "versus-stage",
      stageEntry ? translate(stageEntry[1]) : stageId,
    ),
  );
  line.append(loser);
  return line;
}

/**
 * @param {IndexedCandidate} entry
 * @param {IndexedCandidate | null} previous
 * @param {IndexedCandidate | null} next
 * @param {string[]} marks
 * @returns {HTMLElement}
 */
function scoredCard(entry, previous, next, marks) {
  const { row, index } = entry;
  const card = element("article", "candidate");
  const head = element("div", "candidate-head");
  head.append(
    element(
      "p",
      "place",
      translate("web.inspectorPosition", { index: String(index + 1) }),
    ),
  );
  const name = element("p", "name");
  name.append(element("b", null, row.hanji || row.roman));
  if (row.hanji && row.roman) {
    name.append(element("span", null, row.roman));
  }
  head.append(name);
  head.append(coverageBar(row));
  head.append(sourceBlock(row));
  head.append(facts(row));
  head.append(element("p", "why", whyScored(entry, previous, next)));
  const at = stages.findIndex(([id]) => id === row.placedBy);
  if (previous && at >= 0) {
    head.append(versus(entry, previous, stages[at][0]));
  }
  card.append(head);
  card.append(pipeline(row, marks));
  return card;
}

/**
 * @param {ExplainView} view
 * @returns {number}
 */
function sentenceWordCount(view) {
  return (view.words ?? []).filter(
    (piece) => !piece.gap && piece.kind !== "gap",
  ).length;
}

/**
 * @typedef {object} FrontStep
 * @property {StringKey} label
 * @property {string} text
 * @property {string} [title]
 * @property {boolean} hold
 */

/**
 * Checks that fill slot 0 before the twelve stages. The first check the
 * next row fails keeps the sentence; the rest are not compared. Every
 * check passing is the one case that replaces the sentence.
 *
 * @param {ExplainCandidatePiece} sentence
 * @param {ExplainCandidatePiece | undefined} rival
 * @param {number} wordCount
 * @returns {{ steps: FrontStep[], decided: number }}
 */
function frontSteps(sentence, rival, wordCount) {
  /** @type {FrontStep[]} */
  const steps = [
    {
      label: "web.inspectorFrontWords",
      text:
        wordCount >= 2
          ? translate("web.inspectorFrontManyWords", {
              count: String(wordCount),
            })
          : translate("web.inspectorFrontOneWord"),
      hold: wordCount < 2,
    },
  ];
  if (rival) {
    steps.push(
      {
        label: "web.inspectorFrontKind",
        text: translate(COVERAGE[rival.coverage] ?? "web.inspectorCovFull"),
        hold: rival.coverage !== "prefix",
      },
      {
        label: "web.inspectorFrontSpan",
        text: translate(
          rival.tier === "whole"
            ? "web.inspectorTierWhole"
            : "web.inspectorTierFront",
        ),
        hold: rival.tier !== "whole",
      },
      {
        label: "web.inspectorFrontSyllables",
        text:
          rival.syllables == null
            ? "—"
            : translate("web.inspectorFrontSyllableValue", {
                count: String(rival.syllables),
              }),
        hold: rival.syllables != null && rival.syllables < 2,
      },
      {
        label: "web.inspectorFrontWeight",
        text: formatNumber(rival.weight, 4),
        title: String(rival.weight),
        hold: !(rival.weight > sentence.weight),
      },
    );
  }
  const decided = steps.findIndex((step) => step.hold);
  return { steps, decided };
}

/**
 * @param {IndexedCandidate | null} rival
 * @param {FrontStep[]} steps
 * @param {number} decided
 * @returns {string}
 */
function frontWhy(rival, steps, decided) {
  if (!rival || decided === 0) {
    return translate("web.inspectorFrontHeldWords");
  }
  if (decided < 0) {
    return translate("web.inspectorFrontTaken", {
      index: String(rival.index + 1),
    });
  }
  return translate("web.inspectorFrontHeldHere", {
    index: String(rival.index + 1),
    reason: translate(steps[decided].label),
  });
}

/**
 * @param {ExplainCandidatePiece} sentence
 * @param {IndexedCandidate | null} rival
 * @param {number} wordCount
 * @returns {{ list: HTMLOListElement, steps: FrontStep[], decided: number }}
 */
function frontPipeline(sentence, rival, wordCount) {
  const { steps, decided } = frontSteps(sentence, rival?.row, wordCount);
  const list = element("ol", "pipeline front");
  for (const [stage, step] of steps.entries()) {
    let mark = "tie";
    if (decided < 0) {
      mark = stage === steps.length - 1 ? "lose" : "tie";
    } else if (stage === decided) {
      mark = "win";
    } else if (stage > decided) {
      mark = "skip";
    }
    const item = element("li", `step ${mark}`);
    item.append(
      element("small", null, `${stage + 1} ${translate(step.label)}`),
    );
    const value = element("b", null, step.text);
    if (step.title) {
      value.title = step.title;
    }
    item.append(value);
    if (mark === "win" || mark === "lose") {
      item.append(element("em", null, translate("web.inspectorDecided")));
    }
    list.append(item);
  }
  return { list, steps, decided };
}

/**
 * @param {IndexedCandidate} entry
 * @param {IndexedCandidate | null} rival
 * @param {number} wordCount
 * @returns {HTMLElement}
 */
function pinnedCard(entry, rival, wordCount) {
  const { row, index } = entry;
  const card = element(
    "article",
    row.role === "sentence" ? "candidate pinned sentence" : "candidate pinned",
  );
  card.append(
    element(
      "p",
      "place",
      translate("web.inspectorPosition", { index: String(index + 1) }),
    ),
  );
  const name = element("p", "name");
  name.append(element("b", null, row.hanji || row.roman));
  if (row.hanji && row.roman) {
    name.append(element("span", null, row.roman));
  }
  card.append(name);
  card.append(coverageBar(row));
  if (row.sources.length > 0) {
    card.append(sourceBlock(row));
  }
  card.append(facts(row));
  if (row.role === "sentence") {
    const front = frontPipeline(row, rival, wordCount);
    const weightDecides =
      rival &&
      (front.decided < 0 || front.decided === front.steps.length - 1) &&
      front.steps.length > 1;
    card.append(
      element("p", "why", frontWhy(rival, front.steps, front.decided)),
    );
    if (weightDecides) {
      card.append(versus(rival, entry, "user_weight"));
    }
    card.append(front.list);
  } else {
    card.append(element("p", "why", whyPinned(row)));
  }
  return card;
}

/**
 * @param {(fields: string[], focus?: string) => void} move
 * @returns {HTMLElement}
 */
function mechanism(move) {
  const section = element("section", "board mechanism");
  section.append(
    element("p", "quiet", translate("web.inspectorMechanismLead")),
  );
  section.append(element("p", "quiet", translate("web.inspectorSortNote")));
  const list = element("ol", "mechanism-list");
  for (const [id, label, detail] of stages) {
    const item = element("li");
    item.draggable = true;
    item.tabIndex = 0;
    item.dataset.field = id;
    item.append(element("b", null, translate(label)));
    item.append(element("p", null, translate(detail)));
    item.addEventListener("dragstart", (event) => {
      if (event.dataTransfer) {
        event.dataTransfer.setData("text/plain", id);
        event.dataTransfer.effectAllowed = "move";
      }
      item.classList.add("dragging");
    });
    item.addEventListener("dragend", () => item.classList.remove("dragging"));
    item.addEventListener("dragover", (event) => {
      event.preventDefault();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "move";
      }
      item.classList.add("over");
    });
    item.addEventListener("dragleave", () => item.classList.remove("over"));
    item.addEventListener("drop", (event) => {
      event.preventDefault();
      item.classList.remove("over");
      move(movedFields(event.dataTransfer?.getData("text/plain") ?? "", id));
    });
    item.addEventListener("keydown", (event) => {
      if (event.key !== "ArrowUp" && event.key !== "ArrowDown") {
        return;
      }
      event.preventDefault();
      const ids = stages.map(([field]) => field);
      const index = ids.indexOf(id);
      const target = event.key === "ArrowUp" ? index - 1 : index + 1;
      if (target < 0 || target >= ids.length) {
        return;
      }
      move(movedFields(id, ids[target]), id);
    });
    list.append(item);
  }
  section.append(list);
  const reset = element("button", "setting-reset");
  reset.type = "button";
  reset.textContent = translate("web.inspectorSortReset");
  reset.addEventListener("mousedown", (event) => event.preventDefault());
  reset.addEventListener("click", () => move(defaultSortFields()));
  section.append(reset);
  return section;
}

/**
 * @param {string} fromId
 * @param {string} toId
 * @returns {string[]}
 */
function movedFields(fromId, toId) {
  const ids = stages.map(([field]) => field);
  const from = ids.indexOf(fromId);
  const to = ids.indexOf(toId);
  if (from < 0 || to < 0 || from === to) {
    return ids;
  }
  const [moved] = ids.splice(from, 1);
  ids.splice(to, 0, moved);
  return ids;
}

/** @returns {HTMLUListElement} */
function legend() {
  const list = element("ul", "legend");
  /** @type {Array<[string, StringKey]>} */
  const entries = [
    ["win", "web.inspectorLegendWin"],
    ["lose", "web.inspectorLegendLose"],
    ["tie", "web.inspectorLegendTie"],
    ["skip", "web.inspectorLegendSkip"],
    ["base", "web.inspectorLegendBase"],
  ];
  for (const [className, key] of entries) {
    const item = element("li", className);
    item.append(element("i"));
    item.append(element("span", null, translate(key)));
    list.append(item);
  }
  return list;
}

/**
 * @typedef {ExplainWordPiece | (ExplainBufferPiece & {
 *   hanji?: null,
 *   roman?: undefined,
 *   kind?: undefined,
 *   userRow?: undefined,
 *   picks?: undefined,
 * })} TrackPiece
 */

/**
 * @param {TrackPiece[]} pieces
 * @param {boolean} labeled
 * @returns {HTMLDivElement}
 */
function track(pieces, labeled) {
  const row = element("div", "cuts");
  for (const [index, piece] of pieces.entries()) {
    const gap = piece.gap || piece.kind === "gap";
    const classes = gap ? "cut gap" : `cut tone-${index % 4}`;
    const block = element("div", piece.userRow ? `${classes} user` : classes);
    block.style.setProperty(
      "--span",
      String(Math.max(1, Array.from(piece.text).length)),
    );
    if (labeled && !gap && piece.hanji) {
      block.append(element("b", null, piece.hanji));
    }
    block.append(element("span", null, piece.text));
    if (labeled && !gap && piece.roman && piece.roman !== piece.text) {
      block.append(element("small", null, piece.roman));
    }
    if (labeled && !gap) {
      const kind =
        piece.kind === "custom"
          ? "web.inspectorKindCustom"
          : piece.kind === "reading"
            ? "web.inspectorKindReading"
            : "web.inspectorKindDictionary";
      block.append(element("small", "kind", translate(kind)));
      if ((piece.picks ?? 0) > 0) {
        block.append(element("small", null, picksText(piece.picks ?? 0, 0)));
      }
    }
    row.append(block);
  }
  return row;
}

/**
 * @param {ExplainView} view
 * @returns {HTMLElement}
 */
function cutSection(view) {
  const section = element("section", "board");
  const syllables = view.syllables ?? [];
  const words = view.words ?? [];
  const realSyllables = syllables.some((piece) => !piece.gap);
  if (!realSyllables && words.length === 0) {
    section.append(element("p", "quiet", translate("web.inspectorNoCut")));
    return section;
  }
  section.append(element("h2", null, translate("web.inspectorRaw")));
  section.append(element("p", "raw", view.raw));
  section.append(element("h3", null, translate("web.inspectorSyllables")));
  section.append(track(syllables, false));
  if (words.length > 0) {
    section.append(element("h3", null, translate("web.inspectorWords")));
    section.append(track(words, true));
    if (words.some((piece) => piece.userRow)) {
      section.append(element("p", "quiet", translate("web.inspectorUserSpan")));
    }
  }
  const caption =
    view.cutBy === "walker"
      ? "web.inspectorCutWalker"
      : view.cutBy === "syllables"
        ? "web.inspectorCutSyllables"
        : "web.inspectorNoCut";
  section.append(element("p", "quiet", translate(caption)));
  return section;
}

/**
 * @param {IndexedCandidate[]} scored
 * @param {number} shown
 * @param {() => void} expand
 * @returns {HTMLElement}
 */
function ladder(scored, shown, expand) {
  const section = element("section", "board");
  section.append(element("h2", null, translate("web.inspectorCandidates")));
  if (scored.length === 0) {
    section.append(element("p", "quiet", translate("web.inspectorNoScored")));
    return section;
  }
  section.append(element("p", "quiet", translate("web.inspectorHint")));
  section.append(legend());
  const marks = marksFor(scored);
  const list = element("div", "cards");
  for (const [index, entry] of scored.slice(0, shown).entries()) {
    list.append(
      scoredCard(
        entry,
        index > 0 ? scored[index - 1] : null,
        scored[index + 1] ?? null,
        marks[index],
      ),
    );
  }
  section.append(list);
  const rest = scored.length - shown;
  if (rest > 0) {
    const button = element(
      "button",
      "expand",
      translate("web.inspectorMore", {
        count: String(Math.min(PAGE, rest)),
      }),
    );
    button.type = "button";
    button.addEventListener("mousedown", (event) => event.preventDefault());
    button.addEventListener("click", expand);
    section.append(button);
  }
  return section;
}

export class Inspector {
  /**
   * `read` asks the session for the mechanism JSON, already parsed.
   * `onSort` receives the field names in the order the user dropped.
   * The second argument is the field to focus again, for a keyboard move.
   *
   * @param {HTMLElement} body
   * @param {HTMLElement} mechanismHost
   * @param {() => ExplainView | null} read
   * @param {(fields: string[]) => void} onSort
   */
  constructor(body, mechanismHost, read, onSort) {
    /** @type {HTMLElement} */
    this.body = body;
    /** @type {HTMLElement} */
    this.mechanismHost = mechanismHost;
    /** @type {() => ExplainView | null} */
    this.read = read;
    /** @type {(fields: string[]) => void} */
    this.onSort = onSort;
    this.raw = "";
    this.shown = PAGE;
    /** @type {string | null} */
    this.focusField = null;
  }

  /** @returns {void} */
  sync() {
    const view = this.read();
    const raw = view?.raw ?? "";
    if (raw !== this.raw) {
      this.raw = raw;
      this.shown = PAGE;
    }
    this.mountMechanism();
    this.body.replaceChildren();
    if (!view?.raw) {
      this.body.append(element("p", "quiet", translate("web.inspectorEmpty")));
      return;
    }
    const rows = (view.candidates ?? []).map((row, index) => ({ row, index }));
    const pinned = rows.filter(({ row }) => row.role !== "word");
    const scored = rows.filter(({ row }) => row.role === "word");
    this.body.append(cutSection(view));
    if (pinned.length > 0) {
      const section = element("section", "board");
      section.append(
        element("h2", null, translate("web.inspectorPinnedHeading")),
      );
      section.append(
        element("p", "quiet", translate("web.inspectorFrontLead")),
      );
      const rival = scored[0] ?? null;
      const wordCount = sentenceWordCount(view);
      const list = element("div", "cards pinned-row");
      for (const entry of pinned) {
        list.append(pinnedCard(entry, rival, wordCount));
      }
      section.append(list);
      this.body.append(section);
    }
    this.body.append(
      ladder(scored, this.shown, () => {
        this.shown += PAGE;
        this.sync();
      }),
    );
  }

  /** @returns {void} */
  mountMechanism() {
    const section = mechanism((fields, focus) => {
      this.focusField = focus ?? null;
      this.onSort(fields);
    });
    this.mechanismHost.replaceChildren(section);
    if (this.focusField) {
      const target = /** @type {HTMLElement | null} */ (
        section.querySelector(`[data-field="${this.focusField}"]`)
      );
      target?.focus();
      this.focusField = null;
    }
  }
}
