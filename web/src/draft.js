// The text outlives the tab: it is kept in `localStorage` as it changes, put
// back at load, and shared by every open tab. Every page write goes through
// `execCommand`, which fires `input`, so one listener sees typing, commits,
// paste, undo and Clear.

const STORAGE_KEY = "taigi.text";
// A burst of keys is one write.
const SAVE_DELAY = 300;

/// Puts the stored text back and keeps it current.
export function keepDraft(textarea, surface) {
  try {
    surface.replaceAll(localStorage.getItem(STORAGE_KEY) ?? "");
  } catch {
    // Blocked storage: the page starts empty.
  }
  /// The pending save; `null` when storage holds this tab's text.
  let timer = null;
  const flush = () => {
    if (timer === null) {
      return;
    }
    clearTimeout(timer);
    timer = null;
    try {
      localStorage.setItem(STORAGE_KEY, textarea.value);
    } catch {
      // Private windows and a full quota refuse. The text stays in this tab.
    }
  };
  textarea.addEventListener("input", () => {
    clearTimeout(timer);
    timer = setTimeout(flush, SAVE_DELAY);
  });
  // `localStorage` is synchronous, so a write on the way out completes. A
  // hidden tab may be discarded without `pagehide`.
  window.addEventListener("pagehide", flush);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      flush();
    }
  });
  // Another tab saved: this one takes its text, so its next key does not write
  // an older copy over it.
  window.addEventListener("storage", (event) => {
    if (event.key !== STORAGE_KEY || event.newValue === null) {
      return;
    }
    if (event.newValue !== textarea.value) {
      clearTimeout(timer);
      timer = null;
      surface.replaceAll(event.newValue);
    }
  });
}
