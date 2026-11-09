// The Markdown markers the format bubble writes into the textarea. Each toggle
// answers which range to replace, with what, and what to select afterwards, or
// `null` when there is nothing to do. No DOM, so Node tests it.

/// `[text](url)`, one level of balanced parentheses in the URL; neither part
/// crosses a line. No `g` flag: a shared global regex keeps `lastIndex`; callers
/// make their own with `links`.
const LINK = /\[([^[\]\n]+)\]\(((?:[^()\s]|\([^()\s]*\))+)\)/;

/// Every link in `text`. Also read by Save's HTML (`save.js`).
export function links(text) {
  return text.matchAll(new RegExp(LINK, "g"));
}

/// Whitespace, punctuation and symbols: a marker next to one inside the range
/// would not be flanking (CommonMark), so `「台灣」` is bolded as `「**台灣**」`.
const EDGE = /[\s\p{P}\p{S}]/u;

/// Line prefixes the block toggles write. Also read by Save's HTML.
export const QUOTE = /^> ?/;
export const HEADING = /^(#{1,3}) /;
export const BULLET = /^- /;
export const NUMBERED = /^(\d+)\. /;

/// The paragraph break Save's HTML splits on (`/\n{2,}/`): an empty line.
const isBlank = (line) => line === "";

/// `[start, end)` without whitespace and punctuation at either end. A walk, not
/// a regex: an unanchored trailing pattern backtracks over long selections.
function trimmed(value, start, end) {
  let from = start;
  while (from < end) {
    const character = String.fromCodePoint(value.codePointAt(from));
    if (!EDGE.test(character)) {
      break;
    }
    from += character.length;
  }
  let to = end;
  while (to > from) {
    const last = value.charCodeAt(to - 1);
    const size = last >= 0xdc00 && last <= 0xdfff && to - 2 >= from ? 2 : 1;
    if (!EDGE.test(value.slice(to - size, to))) {
      break;
    }
    to -= size;
  }
  return [from, to];
}

/// The selection without whitespace at either end.
function stripped(value, start, end) {
  const text = value.slice(start, end);
  const lead = text.length - text.trimStart().length;
  const trail = text.length - text.trimEnd().length;
  return [start + lead, Math.max(start + lead, end - trail)];
}

/// A toggle's answer: `[start, end)` becomes `text`, then `selectLength` units
/// from `selectStart` are selected (all of `text` by default).
const change = (start, end, text, selectStart, selectLength = text.length) => ({
  start,
  end,
  text,
  selectStart,
  selectEnd: selectStart + selectLength,
});

/// `open` and `close` just outside `[start, end)` come off.
const unwrapAround = (value, start, end, open, close) =>
  change(
    start - open.length,
    end + close.length,
    value.slice(start, end),
    start - open.length,
  );

/// `open` and `close` go around `[start, end)`, which stays selected.
const wrapIn = (value, start, end, open, close) =>
  change(
    start,
    end,
    `${open}${value.slice(start, end)}${close}`,
    start + open.length,
    end - start,
  );

function runOf(text, character, isBackward) {
  let count = 0;
  let index = isBackward ? text.length - 1 : 0;
  while (text[index] === character) {
    count += 1;
    index += isBackward ? -1 : 1;
  }
  return count;
}

/// Whether a run of `count` stars reads as this marker: `**` is in a run of 2 or
/// 3, `*` in a run of 1 or 3 (`***x***` is both).
function isMarkerRun(count, marker) {
  return marker.length === 2 ? count >= 2 : count === 1 || count === 3;
}

/// Whether the star run ending at `at` opens a span rather than closing one: an
/// even count of such runs before it in its paragraph.
function opensSpan(value, at, marker) {
  const paragraphStart = value.lastIndexOf("\n\n", at) + 1;
  const isSpace = (character) =>
    character === undefined || /\s/.test(character);
  let count = 0;
  for (const run of value.slice(paragraphStart, at).matchAll(/\*+/g)) {
    const runStart = paragraphStart + run.index;
    const runEnd = runStart + run[0].length;
    // The run in front of the range is the one being asked about.
    if (runEnd === at) {
      break;
    }
    // A star with space on both sides, as in `5 * 3`, is text.
    const isMarker = !isSpace(value[runStart - 1]) || !isSpace(value[runEnd]);
    if (isMarker && isMarkerRun(run[0].length, marker)) {
      count += 1;
    }
  }
  return count % 2 === 0;
}

function toggleStars(marker, value, start, end) {
  const size = marker.length;
  // The markers inside the selection: `**台北**` selected.
  const [outerStart, outerEnd] = stripped(value, start, end);
  const outer = value.slice(outerStart, outerEnd);
  const lead = runOf(outer, "*", false);
  const trail = runOf(outer, "*", true);
  const core = outer.slice(lead, outer.length - trail);
  if (
    lead + trail < outer.length &&
    lead === trail &&
    isMarkerRun(lead, marker) &&
    !core.includes("*")
  ) {
    return change(outerStart, outerEnd, outer.slice(size, -size), outerStart);
  }
  // The markers just outside: `台北` selected in `**台北**`.
  const [innerStart, innerEnd] = trimmed(value, start, end);
  // Part of a span, or several: nothing a toggle can undo cleanly.
  if (
    innerStart >= innerEnd ||
    value.slice(innerStart, innerEnd).includes("*")
  ) {
    return null;
  }
  const before = runOf(value.slice(0, innerStart), "*", true);
  const after = runOf(value.slice(innerEnd), "*", false);
  if (before === after && isMarkerRun(before, marker)) {
    // Stars that close one span and open the next (`*a*b*c*` around `b`).
    if (!opensSpan(value, innerStart, marker)) {
      return null;
    }
    return unwrapAround(value, innerStart, innerEnd, marker, marker);
  }
  return wrapIn(value, innerStart, innerEnd, marker, marker);
}

export function toggleBold(value, start, end) {
  return toggleStars("**", value, start, end);
}

export function toggleItalic(value, start, end) {
  return toggleStars("*", value, start, end);
}

/// `<u>x</u>`: Markdown has no underline, and CommonMark passes inline HTML on.
export function toggleUnderline(value, start, end) {
  const open = "<u>";
  const close = "</u>";
  const [outerStart, outerEnd] = stripped(value, start, end);
  const outer = value.slice(outerStart, outerEnd);
  if (
    outer.length > open.length + close.length &&
    outer.startsWith(open) &&
    outer.endsWith(close) &&
    !outer.slice(open.length, -close.length).includes(open)
  ) {
    return change(
      outerStart,
      outerEnd,
      outer.slice(open.length, -close.length),
      outerStart,
    );
  }
  // Part of a span, or several: nothing a toggle can undo cleanly.
  if (outer.includes(open) || outer.includes(close)) {
    return null;
  }
  const [innerStart, innerEnd] = trimmed(value, start, end);
  if (innerStart >= innerEnd) {
    return null;
  }
  if (
    value.slice(innerStart - open.length, innerStart) === open &&
    value.slice(innerEnd, innerEnd + close.length) === close
  ) {
    return unwrapAround(value, innerStart, innerEnd, open, close);
  }
  return wrapIn(value, innerStart, innerEnd, open, close);
}

/// The lines the range touches, as `{ lines, first, last, offsets }`, widened by
/// `reach`: `lines` not at all, `paragraph` to whole paragraphs, `list` to the
/// paragraph's end and back over list items.
function touchedLines(value, start, end, reach) {
  const lines = value.split("\n");
  const offsets = [];
  let offset = 0;
  for (const line of lines) {
    offsets.push(offset);
    offset += line.length + 1;
  }
  const lineAt = (position) =>
    offsets.findLastIndex((lineStart) => lineStart <= position);
  let first = lineAt(start);
  let last = lineAt(Math.max(start, end - 1));
  // A list may interrupt a paragraph, so it reaches back only over items.
  const reachesBack = {
    lines: () => false,
    paragraph: (line) => !isBlank(line),
    list: (line) => BULLET.test(line) || NUMBERED.test(line),
  }[reach];
  while (first > 0 && reachesBack(lines[first - 1])) {
    first -= 1;
  }
  if (reach !== "lines") {
    while (last < lines.length - 1 && !isBlank(lines[last + 1])) {
      last += 1;
    }
  }
  return { lines, first, last, offsets };
}

/// Rewrites each written line the range touches: `isOn` says whether every
/// written line already has the prefix; `mark(line, index)` adds it, `unmark`
/// takes it off. `reach` is `lines`, `paragraph` or `list` (`touchedLines`).
/// Empty lines stay, and so do lines of spaces unless `marksSpaces`.
function toggleLines(
  value,
  start,
  end,
  reach,
  { isOn, mark, unmark, marksSpaces = false },
) {
  const { lines, first, last, offsets } = touchedLines(
    value,
    start,
    end,
    reach,
  );
  const touched = lines.slice(first, last + 1);
  const isLeft = (line) =>
    isBlank(line) || (!marksSpaces && line.trim() === "");
  const written = touched.filter((line) => !isLeft(line));
  if (written.length === 0) {
    return null;
  }
  const isRemoving = written.every(isOn);
  let index = 0;
  const text = touched
    .map((line) => {
      if (isBlank(line)) {
        index = 0;
      }
      if (isLeft(line)) {
        return line;
      }
      index += 1;
      return isRemoving ? unmark(line) : mark(line, index);
    })
    .join("\n");
  const from = offsets[first];
  return change(from, from + touched.join("\n").length, text, from);
}

/// The line's text without a list or heading prefix.
const bare = (line) =>
  line.replace(HEADING, "").replace(BULLET, "").replace(NUMBERED, "");

/// `> ` on every line of the paragraphs the range touches. Whole paragraphs:
/// CommonMark carries a quote on to the unmarked lines that follow it.
export function toggleQuote(value, start, end) {
  return toggleLines(value, start, end, "paragraph", {
    marksSpaces: true,
    isOn: (line) => QUOTE.test(line),
    mark: (line) => `> ${line}`,
    unmark: (line) => line.replace(QUOTE, ""),
  });
}

/// `# ` to `### ` on each line the range touches. A heading is one line, so
/// it takes no more than those; another level or a list prefix is replaced.
export function toggleHeading(level, value, start, end) {
  const prefix = `${"#".repeat(level)} `;
  return toggleLines(value, start, end, "lines", {
    isOn: (line) => line.match(HEADING)?.[1].length === level,
    mark: (line) => `${prefix}${bare(line)}`,
    unmark: (line) => line.replace(HEADING, ""),
  });
}

/// `- ` on every line from the first the range touches to its paragraph's end:
/// an unmarked line after an item would continue it. Back only over items.
export function toggleBulletList(value, start, end) {
  return toggleLines(value, start, end, "list", {
    isOn: (line) => BULLET.test(line),
    mark: (line) => `- ${bare(line)}`,
    unmark: (line) => line.replace(BULLET, ""),
  });
}

/// `1. `, `2. `… over the same lines as `toggleBulletList`, counted from 1.
export function toggleNumberedList(value, start, end) {
  return toggleLines(value, start, end, "list", {
    isOn: (line) => NUMBERED.test(line),
    mark: (line, index) => `${index}. ${bare(line)}`,
    unmark: (line) => line.replace(NUMBERED, ""),
  });
}

/// The link the range sits in, `{ start, end, text, url }`, or `null`.
export function linkAt(value, start, end) {
  for (const match of links(value)) {
    const linkEnd = match.index + match[0].length;
    if (match.index <= start && end <= linkEnd) {
      return {
        start: match.index,
        end: linkEnd,
        text: match[1],
        url: match[2],
      };
    }
  }
  return null;
}

/// What the URL field holds, ready for `[text](url)`: `https://` when no scheme
/// is named (`localhost:3000` names none), and the characters that would end
/// the link percent-encoded. `null` when empty.
export function normalizeUrl(input) {
  const url = input.trim();
  if (url === "") {
    return null;
  }
  const hasScheme = /^([a-z][a-z0-9+.-]*:\/\/|mailto:|tel:)/i.test(url);
  const full = hasScheme ? url : `https://${url}`;
  return full.replace(/[()<>\s]/gu, (character) => {
    if (character === "(") {
      return "%28";
    }
    return character === ")" ? "%29" : encodeURIComponent(character);
  });
}

/// `[x](url)` around the range; on a link already there, a new `url` replaces
/// its URL and `null` takes the link off, leaving its text.
export function toggleLink(value, start, end, url) {
  const link = linkAt(value, start, end);
  if (link !== null) {
    const text = url === null ? link.text : `[${link.text}](${url})`;
    const textStart = url === null ? link.start : link.start + 1;
    return change(link.start, link.end, text, textStart, link.text.length);
  }
  const [innerStart, innerEnd] = trimmed(value, start, end);
  if (url === null || innerStart >= innerEnd) {
    return null;
  }
  return wrapIn(value, innerStart, innerEnd, "[", `](${url})`);
}
