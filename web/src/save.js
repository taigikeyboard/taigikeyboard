// Save in the sidebar: the title and the text as a file, in the format the
// button names, as ZenPen's `formatText` writes them. The HTML reads the
// format bubble's Markdown (`markdown.js`): headings, lists, quotes, bold,
// italic, underline and links.

import { BULLET, HEADING, links, NUMBERED, QUOTE } from "./markdown.js";

const ITEM_MARKERS = { ul: BULLET, ol: NUMBERED };

/// Links that open a page or a mail; any other scheme stays text.
const SAFE_URL = /^(https?|mailto):/i;

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/// `**x**`, `*x*` and `<u>x</u>` on escaped text, a star touching no space on
/// its inner side, as CommonMark's flanking rule asks.
function emphasize(html) {
  return html
    .replace(
      /\*\*\*(?=[^\s*])(.+?)(?<=[^\s*])\*\*\*/g,
      "<strong><em>$1</em></strong>",
    )
    .replace(/\*\*(?=\S)(.+?)(?<=\S)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(?=[^\s*])(.+?)(?<=[^\s*])\*/g, "<em>$1</em>")
    .replace(/&lt;u&gt;(.+?)&lt;\/u&gt;/g, "<u>$1</u>");
}

/// Lines of text, links first so emphasis never reaches an `href`.
function inlineHtml(text) {
  let html = "";
  let from = 0;
  for (const match of links(text)) {
    const [whole, label, url] = match;
    html += emphasize(escapeHtml(text.slice(from, match.index)));
    html += SAFE_URL.test(url)
      ? `<a href="${escapeHtml(url).replaceAll('"', "&quot;")}">${emphasize(escapeHtml(label))}</a>`
      : emphasize(escapeHtml(whole));
    from = match.index + whole.length;
  }
  html += emphasize(escapeHtml(text.slice(from)));
  return html.replaceAll("\n", "<br>\n");
}

function lineKind(line) {
  if (HEADING.test(line)) {
    return "heading";
  }
  if (BULLET.test(line)) {
    return "ul";
  }
  return NUMBERED.test(line) ? "ol" : "p";
}

/// Consecutive lines of one kind; a heading is always its own.
function runHtml(kind, lines) {
  if (kind === "heading") {
    const [prefix, hashes] = lines[0].match(HEADING);
    const level = hashes.length;
    return `<h${level}>${inlineHtml(lines[0].slice(prefix.length))}</h${level}>`;
  }
  if (kind === "p") {
    return `<p>${inlineHtml(lines.join("\n"))}</p>`;
  }
  const marker = ITEM_MARKERS[kind];
  const first = kind === "ol" ? Number(lines[0].match(NUMBERED)[1]) : 1;
  const start = first === 1 ? "" : ` start="${first}"`;
  const items = lines.map(
    (line) => `<li>${inlineHtml(line.replace(marker, ""))}</li>`,
  );
  return `<${kind}${start}>\n${items.join("\n")}\n</${kind}>`;
}

/// Lines in runs of one kind: headings, list items, plain lines.
function linesHtml(lines) {
  const runs = [];
  for (const line of lines) {
    const kind = lineKind(line);
    const last = runs.at(-1);
    if (last?.kind === kind && kind !== "heading") {
      last.lines.push(line);
    } else {
      runs.push({ kind, lines: [line] });
    }
  }
  return runs.map(({ kind, lines }) => runHtml(kind, lines)).join("\n");
}

/// A blank-line paragraph, read on the raw text (escaping turns `>` into
/// `&gt;`): a quote, around its own runs, when every line starts with `>`.
function blockHtml(block) {
  const lines = block.split("\n");
  if (lines.every((line) => QUOTE.test(line))) {
    const inner = lines.map((line) => line.replace(QUOTE, ""));
    return `<blockquote>${linesHtml(inner)}</blockquote>`;
  }
  return linesHtml(lines);
}

/// The title as `<h1>`, then each blank-line paragraph (`blockHtml`).
export function toHtml(title, text) {
  const paragraphs = text
    .split(/\n{2,}/)
    .filter((block) => block.trim() !== "")
    .map(blockHtml);
  const heading = title === "" ? [] : [`<h1>${escapeHtml(title)}</h1>`];
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
</head>
<body>
${[...heading, ...paragraphs].join("\n")}
</body>
</html>
`;
}

/// ZenPen writes `#title#`, which CommonMark does not read as a heading.
function toMarkdown(title, text) {
  return title === "" ? text : `# ${title}\n\n${text}`;
}

function toPlainText(title, text) {
  return title === "" ? text : `${title}\n\n${text}`;
}

const FORMATS = {
  markdown: { extension: "md", type: "text/markdown", render: toMarkdown },
  html: { extension: "html", type: "text/html", render: toHtml },
  text: { extension: "txt", type: "text/plain", render: toPlainText },
};

/// The title, as ZenPen names a file by its header; an empty one leaves the
/// app's name, as ZenPen's leaves "ZenPen".
function fileName(title) {
  // Characters no file system takes.
  const safe = title.replace(/[\\/:*?"<>|]/g, "").trim();
  return safe === "" ? "台語齒盤" : safe;
}

function download(name, type, content) {
  const url = URL.createObjectURL(new Blob([content], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // Firefox and WebKit may start reading the URL after `click` returns.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/// `root` holds one button per format, `data-format` naming it; `title` is the
/// editable page title.
export function wireSave(root, textarea, title) {
  root.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-format]");
    if (button === null) {
      return;
    }
    const { extension, type, render } = FORMATS[button.dataset.format];
    // One line, as ZenPen strips the header's line breaks.
    const heading = title.textContent.replace(/[\t\r\n]/g, "").trim();
    download(
      `${fileName(heading)}.${extension}`,
      `${type};charset=utf-8`,
      render(heading, textarea.value),
    );
  });
}
