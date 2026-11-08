// The title and the subtitle each carry a delete button. A deleted line stays
// gone across visits: `<html data-hidden-header>` lists the ids style.css hides,
// stored as is in `taigi.hiddenHeader`, which `prepaint.js` restores before first paint.

export function wireHeaderDelete(buttons, textarea) {
  const root = document.documentElement;
  for (const button of buttons) {
    button.addEventListener("click", () => {
      const hidden = (root.dataset.hiddenHeader ?? "").split(" ");
      hidden.push(button.closest(".header-line").id);
      root.dataset.hiddenHeader = hidden.filter(Boolean).join(" ");
      try {
        localStorage.setItem("taigi.hiddenHeader", root.dataset.hiddenHeader);
      } catch {
        // Blocked storage: gone for this visit only.
      }
      // The button that had the focus is gone with its line.
      textarea.focus();
    });
  }
}
