// Save in the sidebar: the title and the text as a file, in the format the
// button names, as ZenPen's `formatText` writes them. The text is the editor's
// Markdown (`markdown.js`); HTML renders its blocks, plain text drops markers.

import { parse, plainText } from "./markdown.js";

/// Links that open a page or a mail; any other scheme stays text.
const SAFE_URL = /^(https?|mailto):/i;

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/// One run: underline innermost, then italic, then bold.
function runHtml(run) {
  let html = escapeHtml(run.text).replaceAll("\n", "<br>\n");
  if (run.underline) {
    html = `<u>${html}</u>`;
  }
  if (run.italic) {
    html = `<em>${html}</em>`;
  }
  return run.bold ? `<strong>${html}</strong>` : html;
}

/// Runs, those of one link inside one `<a>`; an unsafe link stays its text.
function runsHtml(runs) {
  let html = "";
  for (let index = 0; index < runs.length; ) {
    const { url } = runs[index];
    let label = "";
    while (index < runs.length && runs[index].url === url) {
      label += runHtml(runs[index]);
      index += 1;
    }
    if (url === null || !SAFE_URL.test(url)) {
      html += label;
    } else {
      html += `<a href="${escapeHtml(url).replaceAll('"', "&quot;")}">${label}</a>`;
    }
  }
  return html;
}

function blockHtml(block) {
  switch (block.type) {
    case "heading":
      return `<h${block.level}>${runsHtml(block.runs)}</h${block.level}>`;
    case "quote":
      return `<blockquote><p>${runsHtml(block.runs)}</p></blockquote>`;
    case "list": {
      const tag = block.ordered ? "ol" : "ul";
      const start =
        block.ordered && block.start !== 1 ? ` start="${block.start}"` : "";
      const items = block.items.map((runs) => `<li>${runsHtml(runs)}</li>`);
      return `<${tag}${start}>\n${items.join("\n")}\n</${tag}>`;
    }
    default:
      return `<p>${runsHtml(block.runs)}</p>`;
  }
}

/// The title as `<h1>`, then each block of the text.
export function toHtml(title, text) {
  const blocks = parse(text).map(blockHtml);

  const heading = title === "" ? [] : [`<h1>${escapeHtml(title)}</h1>`];
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
</head>
<body>
${[...heading, ...blocks].join("\n")}
</body>
</html>
`;
}

/// ZenPen writes `#title#`, which CommonMark does not read as a heading.
function toMarkdown(title, text) {
  return title === "" ? text : `# ${title}\n\n${text}`;
}

function toPlainText(title, text) {
  const body = plainText(parse(text));
  return title === "" ? body : `${title}\n\n${body}`;
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

/// `root` holds one button per format, `data-format` naming it; `markdown()`
/// answers the text; `title` is the editable page title.
export function wireSave(root, markdown, title) {
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
      render(heading, markdown()),
    );
  });
}
