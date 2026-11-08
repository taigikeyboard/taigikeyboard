// Restores the theme, the font, the TPS key panel's room and the sidebar before
// first paint, so a reload does not flash. `<html>` carries the first visit's font
// and closed sidebar, which also stand when storage is blocked. Classic and blocking, so it runs before the page's content is parsed.
// A file rather than an inline script, which the page's CSP will not allow. The
// first child of `<body>`: from `<head>`, WebKit left `body` on stale or missing
// rules in up to 4 of 10 reloads (found by bisecting; why is not known).

try {
  const root = document.documentElement;
  if (localStorage.getItem("taigi.sidebar") === "open") {
    root.dataset.sidebar = "open";
  }
  const stored = JSON.parse(localStorage.getItem("taigi.settings"));
  const theme = stored?.values?.candidateAppearanceMode;
  if (typeof theme === "string") {
    root.dataset.theme = theme;
  }
  const font = stored?.values?.fontType;
  if (typeof font === "string") {
    root.dataset.font = font;
  }
  if (
    stored?.values?.inputMode === "tps" &&
    stored?.values?.tpsKeyboardShown === true
  ) {
    root.dataset.tpsKeyboard = "";
  }
} catch {
  // Blocked storage or a damaged document: the defaults paint.
}
