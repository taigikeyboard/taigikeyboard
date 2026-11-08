// The title is the user's to edit or delete, as ZenPen's header is. An edited
// title is kept in `localStorage`; it names a saved file (`save.js`). Its keys go
// straight to it: the engine composes only in the textarea.

const STORAGE_KEY = "taigi.title";

export function keepTitle(title, textarea) {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    // ZenPen's `loadState`: an empty title opens on the default one.
    if (stored) {
      title.textContent = stored;
    }
  } catch {
    // Blocked storage: the markup's title shows.
  }
  title.addEventListener("input", () => {
    try {
      localStorage.setItem(STORAGE_KEY, title.textContent);
    } catch {
      // Blocked storage: the edit lasts for this visit.
    }
  });
  // One line: Enter moves on to the text.
  title.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.isComposing) {
      event.preventDefault();
      textarea.focus();
    }
  });
}
