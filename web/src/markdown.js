// The stored text's Markdown and the editor's blocks, both ways. The editor
// shows formatting and never its markers; the text is kept, saved and exported
// as this Markdown subset: headings, quotes, lists, bold, italic, underline
// (`<u>`) and links. No DOM, so Node tests it.
//
// A block is `{ type, runs }`: `paragraph`, `heading` (with `level` 1–3),
// `quote`, or `list` (with `ordered`, `start` and `items`, each a run list). A
// run is `{ text, bold, italic, underline, url }`; `\n` in a run's text is a
// line break inside its block.

/// An empty line, as the textarea's text always had: paragraphs apart.
const PARAGRAPH_BREAK = /\n{2,}/;

const QUOTE = /^> ?/;
const HEADING = /^(#{1,3}) /;
const BULLET = /^- /;
const NUMBERED = /^(\d+)\. /;

/// `[text](url)`, one level of balanced parentheses in the URL, not across a
/// line.
const LINK = /\[([^[\]\n]+)\]\(((?:[^()\s]|\([^()\s]*\))+)\)/;

/// Inline spans, the first that matches earliest wins: a star touching no space
/// on its inner side, as CommonMark's flanking rule asks.
const SPANS = [
  { pattern: LINK, apply: (match) => ({ url: match[2] }), inner: 1 },
  {
    pattern: /\*\*\*(?=[^\s*])(.+?)(?<=[^\s*])\*\*\*/,
    apply: () => ({ bold: true, italic: true }),
    inner: 1,
  },
  {
    pattern: /\*\*(?=\S)(.+?)(?<=\S)\*\*/,
    apply: () => ({ bold: true }),
    inner: 1,
  },
  {
    pattern: /\*(?=[^\s*])(.+?)(?<=[^\s*])\*/,
    apply: () => ({ italic: true }),
    inner: 1,
  },
  { pattern: /<u>(.+?)<\/u>/, apply: () => ({ underline: true }), inner: 1 },
];

const PLAIN = { bold: false, italic: false, underline: false, url: null };

/// A backslash keeps a marker character as text; while spans are read it stands
/// in as a private-use character, which no span pattern matches.
const ESCAPES = [
  ["\\\\", "\u{E004}", "\\"],
  ["\\*", "\u{E000}", "*"],
  ["\\<", "\u{E001}", "<"],
  ["\\]", "\u{E002}", "]"],
  ["\\[", "\u{E003}", "["],
];

function hideEscapes(text) {
  return ESCAPES.reduce(
    (out, [escaped, stand]) => out.replaceAll(escaped, stand),
    text,
  );
}

function restoreEscapes(text) {
  return ESCAPES.reduce(
    (out, [, stand, plain]) => out.replaceAll(stand, plain),
    text,
  );
}

/// Plain text as Markdown that reads back as the same text, for text that would
/// otherwise read as a span: stars, a `<u>` tag and link brackets get a backslash.
function escapeInline(text) {
  return text.replace(/[\\*[\]]/g, "\\$&").replace(/<(?=\/?u>)/g, "\\<");
}

/// A line of a paragraph that would read as a heading, an item or a quote.
const BLOCK_PREFIX = /^(?:#{1,3} |- |> ?)/;
const NUMBER_PREFIX = /^(\d+)\. /;

function escapeLine(line) {
  // A backslash of the text's own before a prefix, which reading would take
  // for an escape.
  if (BLOCK_PREFIX.test(line) || /^\\(?=#{1,3} |- |>)/.test(line)) {
    return `\\${line}`;
  }
  // `1\. ` of the text's own would read as an escaped `1. `.
  if (/^\d+\\\. /.test(line)) {
    return line.replace(/^(\d+)\\/, "$1\\\\");
  }
  return line.replace(NUMBER_PREFIX, "$1\\. ");
}

function unescapeLine(line) {
  return line.replace(/^\\(?=#{1,3} |- |>)/, "").replace(/^(\d+)\\\. /, "$1. ");
}

function parseInline(text, format = PLAIN) {
  const runs = [];
  let rest = text;
  while (rest !== "") {
    let first = null;
    for (const span of SPANS) {
      const match = rest.match(span.pattern);
      if (
        match !== null &&
        (first === null || match.index < first.match.index)
      ) {
        first = { span, match };
      }
    }
    if (first === null) {
      runs.push({ ...format, text: rest });
      break;
    }
    const { span, match } = first;
    if (match.index > 0) {
      runs.push({ ...format, text: rest.slice(0, match.index) });
    }
    runs.push(
      ...parseInline(match[span.inner], { ...format, ...span.apply(match) }),
    );
    rest = rest.slice(match.index + match[0].length);
  }
  return runs;
}

/// A line's spans, its escaped characters back as text.
function inline(text) {
  return parseInline(hideEscapes(text)).map((run) => ({
    ...run,
    text: restoreEscapes(run.text),
    url: run.url === null ? null : restoreEscapes(run.url),
  }));
}

function lineKind(line) {
  if (HEADING.test(line)) {
    return "heading";
  }
  if (BULLET.test(line)) {
    return "bullet";
  }
  return NUMBERED.test(line) ? "numbered" : "paragraph";
}

/// One paragraph's lines in runs of one kind; a heading is always its own.
function parseLines(lines) {
  const groups = [];
  for (const line of lines) {
    const kind = lineKind(line);
    // A line of spaces between a list and a paragraph is no paragraph.
    if (
      kind === "paragraph" &&
      line.trim() === "" &&
      groups.length > 0 &&
      groups.at(-1).kind !== "paragraph"
    ) {
      continue;
    }
    const last = groups.at(-1);
    if (last?.kind === kind && kind !== "heading") {
      last.lines.push(line);
    } else {
      groups.push({ kind, lines: [line] });
    }
  }
  return groups.map(({ kind, lines: group }) => {
    if (kind === "heading") {
      const [prefix, hashes] = group[0].match(HEADING);
      return {
        type: "heading",
        level: hashes.length,
        runs: inline(group[0].slice(prefix.length)),
      };
    }
    if (kind === "paragraph") {
      return {
        type: "paragraph",
        runs: inline(group.map(unescapeLine).join("\n")),
      };
    }
    const marker = kind === "bullet" ? BULLET : NUMBERED;
    return {
      type: "list",
      ordered: kind === "numbered",
      start: kind === "numbered" ? Number(group[0].match(NUMBERED)[1]) : 1,
      items: group.map((line) => inline(line.replace(marker, ""))),
    };
  });
}

/// The stored text as blocks. Text with no markers is one paragraph per
/// blank-line block, its single line breaks kept.
export function parse(markdown) {
  return markdown
    .split(PARAGRAPH_BREAK)
    .filter((block) => block.trim() !== "")
    .flatMap((block) => {
      const lines = block.split("\n");
      if (lines.every((line) => QUOTE.test(line))) {
        const inner = lines.map((line) => line.replace(QUOTE, ""));
        return [{ type: "quote", runs: inline(inner.join("\n")) }];
      }
      return parseLines(lines);
    });
}

const sameFormat = (a, b) =>
  a.bold === b.bold &&
  a.italic === b.italic &&
  a.underline === b.underline &&
  a.url === b.url;

/// Neighbouring runs with the same format are one.
function merged(runs) {
  const out = [];
  for (const run of runs) {
    const last = out.at(-1);
    if (last !== undefined && sameFormat(last, run)) {
      last.text += run.text;
    } else if (run.text !== "") {
      out.push({ ...PLAIN, ...run });
    }
  }
  return out;
}

/// One run's markers around its text, spaces at either end kept outside so the
/// markers stay flanking.
/// Whether `text` reads back as itself, so it needs no backslash.
function readsAsText(text) {
  const [only, ...more] = inline(text);
  return more.length === 0 && only.text === text && sameFormat(only, PLAIN);
}

function markRun(run, isStrict) {
  const [, lead, core, trail] = run.text.match(/^(\s*)([\s\S]*?)(\s*)$/);
  if (core === "") {
    return run.text;
  }
  let text = isStrict || !readsAsText(core) ? escapeInline(core) : core;
  if (run.underline) {
    text = `<u>${text}</u>`;
  }
  const stars = (run.bold ? "**" : "") + (run.italic ? "*" : "");
  text = `${stars}${text}${stars}`;
  return `${lead}${text}${trail}`;
}

function serializeRuns(list, isStrict) {
  let out = "";
  for (let index = 0; index < list.length; ) {
    const { url } = list[index];
    if (url === null) {
      out += markRun(list[index], isStrict);
      index += 1;
      continue;
    }
    let label = "";
    while (index < list.length && list[index].url === url) {
      label += markRun({ ...list[index], url: null }, isStrict);
      index += 1;
    }
    out += `[${label}](${url})`;
  }
  return out;
}

/// Runs as Markdown, a backslash only where text would read as a span. Runs
/// that read as text alone may still meet across a boundary (`\<u>` then
/// `</u>`): when the whole does not read back, every marker character gets one.
function writeRuns(runs) {
  const list = merged(runs);
  const out = serializeRuns(list, false);
  const isFaithful =
    JSON.stringify(merged(inline(out))) === JSON.stringify(list);
  return isFaithful ? out : serializeRuns(list, true);
}

/// A heading's or an item's runs on one line: neither has a line break.
const oneLine = (runs) =>
  runs.map((run) => ({ ...run, text: run.text.replaceAll("\n", " ") }));

/// The blocks as the stored text, one empty line between blocks.
export function serialize(blocks) {
  return blocks
    .map((block) => {
      switch (block.type) {
        case "heading":
          return `${"#".repeat(block.level)} ${writeRuns(oneLine(block.runs))}`;
        case "quote":
          return writeRuns(block.runs)
            .split("\n")
            .map((line) => `> ${line}`)
            .join("\n");
        case "list":
          return block.items
            .map((runs, index) => {
              const marker = block.ordered ? `${block.start + index}. ` : "- ";
              return `${marker}${writeRuns(oneLine(runs))}`;
            })
            .join("\n");
        default:
          return writeRuns(block.runs).split("\n").map(escapeLine).join("\n");
      }
    })
    .join("\n\n");
}

/// What the URL field holds, ready for a link: `https://` when no scheme is
/// named (`localhost:3000` names none), and the characters that would end a
/// Markdown link percent-encoded. `null` when empty.
export function normalizeUrl(input) {
  const url = input.trim();
  if (url === "") {
    return null;
  }
  const hasScheme = /^([a-z][a-z0-9+.-]*:\/\/|mailto:|tel:)/i.test(url);
  return encodeUrl(hasScheme ? url : `https://${url}`);
}

/// `url` with the characters that would end a Markdown link percent-encoded.
export function encodeUrl(url) {
  return url.replace(/[()<>\s]/gu, (character) => {
    if (character === "(") {
      return "%28";
    }
    return character === ")" ? "%29" : encodeURIComponent(character);
  });
}

/// The blocks as plain text, for `.txt`: no markers, a list item keeps its
/// bullet or number so the list still reads as one.
export function plainText(blocks) {
  const text = (runs) => runs.map((run) => run.text).join("");
  return blocks
    .map((block) => {
      if (block.type !== "list") {
        return text(block.runs);
      }
      return block.items
        .map((runs, index) => {
          const marker = block.ordered ? `${block.start + index}. ` : "- ";
          return `${marker}${text(runs)}`;
        })
        .join("\n");
    })
    .join("\n\n");
}

/// A text from before the editor, kept as plain text: its paragraphs as they
/// were, nothing read as Markdown.
export function fromPlainText(text) {
  return serialize(
    text
      .split(PARAGRAPH_BREAK)
      .filter((block) => block.trim() !== "")
      .map((block) => ({
        type: "paragraph",
        runs: [{ ...PLAIN, text: block }],
      })),
  );
}
