// The sidebar's Learning Records section (docs/architecture/web-roadmap.md § 8b):
// the desktop pane's list over the engine's learning stores, from
// `Session.learningRecords` (`learning_records.rs`), whose text arrives
// translated. The page keeps the view (kind, order, filter, page), so an
// instance replaced after a trap lists the same one. Only the tab whose stores
// are open shows it (maintainer 2026-11-07). It loads when opened and after its
// own changes, as the desktop panes do, not on every commit: a list redrawn
// under a click would lose the click.

/// `FILTER_SETTLE` in `listing.rs`: a word typed letter by letter is one query.
const FILTER_SETTLE = 200;

const SVG = "http://www.w3.org/2000/svg";

function deleteIcon() {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", "0 0 20 20");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", "M6 6l8 8M14 6l-8 8");
  path.setAttribute("fill", "none");
  path.setAttribute("stroke", "currentColor");
  path.setAttribute("stroke-width", "1.5");
  path.setAttribute("stroke-linecap", "round");
  svg.append(path);
  return svg;
}

function cell(className, text) {
  const node = document.createElement("span");
  node.className = className;
  node.textContent = text;
  return node;
}

export class LearningRecords {
  #root;
  #elements;
  /// `(action) => answer`: an engine call through `ime.js` `run`, `undefined`
  /// with no instance or after a trap.
  #run;
  /// Whether the session `run` calls has its stores open.
  #isOpen;
  /// After a write: the stores changed, so the page saves them now.
  #onWrite;
  #view = { kind: 0, order: 0, filter: "", page: 0 };
  #filterTimer;
  /// The delete label, read when the rows are drawn, so it follows the language.
  #deleteLabel;
  /// The page's language when the rows were drawn: the session's text is in it.
  #drawnLanguage;

  constructor(root, run, isOpen, onWrite, deleteLabel) {
    this.#root = root;
    this.#run = run;
    this.#isOpen = isOpen;
    this.#onWrite = onWrite;
    this.#deleteLabel = deleteLabel;
    const element = (id) => root.querySelector(`#${id}`);
    this.#elements = {
      kind: element("records-kind"),
      order: element("records-order"),
      filter: element("records-filter"),
      rows: element("records-rows"),
      empty: element("records-empty"),
      count: element("records-count"),
      page: element("records-page"),
      previous: element("records-previous"),
      next: element("records-next"),
      status: element("records-status"),
      clear: element("records-clear"),
    };
    const { kind, order, filter, rows, previous, next, clear } = this.#elements;
    root.addEventListener("toggle", () => this.refresh());
    kind.addEventListener("change", () => this.#rewind({ kind: +kind.value }));
    order.addEventListener("change", () =>
      this.#rewind({ order: +order.value }),
    );
    filter.addEventListener("input", () => {
      clearTimeout(this.#filterTimer);
      this.#filterTimer = setTimeout(
        () => this.#rewind({ filter: filter.value }),
        FILTER_SETTLE,
      );
    });
    previous.addEventListener("click", () => this.#step(-1));
    next.addEventListener("click", () => this.#step(1));
    // No confirmation (maintainer 2026-10-16).
    clear.addEventListener("click", () => this.#load({ type: "clear" }));
    rows.addEventListener("click", ({ target }) => {
      const button = target.closest("button[data-id]");
      if (button) {
        this.#delete(button);
      }
    });
  }

  /// The stores of the current session opened: the section takes its place.
  show() {
    this.#root.hidden = false;
    this.refresh();
  }

  /// Reloads what is on screen, when the section is open.
  refresh() {
    if (this.#root.open) {
      this.#load();
    }
  }

  /// After a settings change: only the language changes the session's text.
  redrawIfLanguageChanged() {
    if (document.documentElement.lang !== this.#drawnLanguage) {
      this.refresh();
    }
  }

  /// Another kind, order or filter: from the first page.
  #rewind(change) {
    Object.assign(this.#view, change, { page: 0 });
    this.#load();
  }

  #step(delta) {
    this.#view.page += delta;
    this.#load();
  }

  #delete(button) {
    const { rows } = this.#elements;
    const index = Array.from(rows.children).indexOf(button.closest("li"));
    const pageBefore = this.#view.page;
    this.#load({ type: "delete", id: Number(button.dataset.id) });
    if (this.#root.hidden) {
      return;
    }
    // Keyboard users stay in the list: the row that took its place, else the
    // one before (the previous page's last, when the engine pulled the page
    // back), else the filter.
    const buttons = rows.querySelectorAll("button[data-id]");
    const shown = this.#view.page < pageBefore ? buttons.length - 1 : index;
    (
      buttons[Math.min(shown, buttons.length - 1)] ?? this.#elements.filter
    ).focus();
  }

  #load(write) {
    const call = JSON.stringify({
      ...this.#view,
      utcOffsetSeconds: -new Date().getTimezoneOffset() * 60,
      write,
    });
    let answer;
    try {
      answer = this.#run((session) => session.learningRecords(call));
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) {
        throw error;
      }
      // Refused: the instance a trap started has not opened its stores yet.
      console.error(error);
    }
    const view = answer === undefined ? undefined : JSON.parse(answer);
    if (view?.wrote) {
      this.#onWrite();
    }
    // No instance, or one that does not learn (the save may have trapped):
    // the section waits for the next one to open its stores.
    if (view === undefined || !this.#isOpen()) {
      this.#root.hidden = true;
      return;
    }
    this.#draw(view);
  }

  #draw(view) {
    const { rows, empty, count, page, previous, next, status } = this.#elements;
    this.#view.page = view.page;
    this.#drawnLanguage = document.documentElement.lang;
    const deleteLabel = this.#deleteLabel();
    rows.replaceChildren(
      ...view.rows.map((row) => this.#row(row, deleteLabel)),
    );
    empty.textContent = view.empty ?? "";
    empty.hidden = view.empty === undefined;
    count.textContent = view.count;
    page.textContent = view.pageLabel;
    previous.disabled = !view.hasPrevious;
    next.disabled = !view.hasNext;
    status.textContent = view.message?.title ?? "";
    if (view.message?.detail) {
      console.error("web.learningRecords.failed", view.message.detail);
    }
  }

  #row({ id, tl, text, count, lastUsed }, deleteLabel) {
    const line = document.createElement("li");
    line.className = "record";
    // A word is its Hanji and its reading (重/tîng is not 重/tāng): the delete
    // button names both.
    const reading = cell("record-tl", tl);
    reading.id = `record-tl-${id}`;
    const word = cell("record-text", text);
    word.id = `record-${id}`;
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "record-delete";
    remove.dataset.id = String(id);
    remove.setAttribute("aria-label", deleteLabel);
    remove.setAttribute("aria-describedby", `${word.id} ${reading.id}`);
    remove.title = deleteLabel;
    remove.append(deleteIcon());
    line.append(
      reading,
      word,
      cell("record-count", String(count)),
      cell("record-last-used", lastUsed),
      remove,
    );
    return line;
  }
}
