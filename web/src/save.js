// Save in the sidebar: the title and the text as a file, in the format the
// button names, as ZenPen's `formatText` writes them.

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/// The title as `<h1>`, then one `<p>` per blank-line paragraph, its single
/// line breaks kept.
function toHtml(title, text) {
  const paragraphs = text
    .split(/\n{2,}/)
    .filter((block) => block.trim() !== "")
    .map((block) => `<p>${escapeHtml(block).replaceAll("\n", "<br>\n")}</p>`);
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
