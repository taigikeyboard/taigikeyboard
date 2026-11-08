// Save in the sidebar: the text as a file, in the format the button names.

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

/// One `<p>` per blank-line paragraph, its single line breaks kept.
function toHtml(text, title) {
  const paragraphs = text
    .split(/\n{2,}/)
    .filter((block) => block.trim() !== "")
    .map((block) => `<p>${escapeHtml(block).replaceAll("\n", "<br>\n")}</p>`);
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
</head>
<body>
${paragraphs.join("\n")}
</body>
</html>
`;
}

/// The first line, as ZenPen names a file by its title.
function fileName(text) {
  const firstLine = text.trim().split("\n")[0].trim().slice(0, 40);
  // Characters no file system takes.
  const safe = firstLine.replace(/[\\/:*?"<>|]/g, "").trim();
  return safe === "" ? "taigi" : safe;
}

const asTyped = (text) => text;

const FORMATS = {
  markdown: { extension: "md", type: "text/markdown", render: asTyped },
  html: { extension: "html", type: "text/html", render: toHtml },
  text: { extension: "txt", type: "text/plain", render: asTyped },
};

function download(name, type, content) {
  const url = URL.createObjectURL(new Blob([content], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // Firefox and WebKit may start reading the URL after `click` returns.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/// `root` holds one button per format, `data-format` naming it.
export function wireSave(root, textarea) {
  root.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-format]");
    if (button === null) {
      return;
    }
    const { extension, type, render } = FORMATS[button.dataset.format];
    const text = textarea.value;
    const name = fileName(text);
    download(
      `${name}.${extension}`,
      `${type};charset=utf-8`,
      render(text, name),
    );
  });
}
