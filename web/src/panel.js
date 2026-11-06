// The composition and the candidate list, drawn at the caret. Nothing here
// reaches the textarea until a commit, so its undo stack holds only committed
// words (docs/architecture/web-roadmap.md § Text surface).

export class Panel {
  #root;
  #preedit;
  #candidates;

  /// `onPick(position)`: a click on the `position`-th cell of the shown page.
  constructor(root, onPick) {
    this.#root = root;
    this.#preedit = root.querySelector("#preedit");
    this.#candidates = root.querySelector("#candidates");
    // Focus stays in the textarea, which a commit needs.
    root.addEventListener("mousedown", (event) => event.preventDefault());
    this.#candidates.addEventListener("click", (event) => {
      const cell = event.target.closest("li");
      if (cell) {
        onPick(Number(cell.dataset.position));
      }
    });
  }

  get isShown() {
    return !this.#root.hidden;
  }

  contains(node) {
    return this.#root.contains(node);
  }

  /// `caret` counts Unicode scalars.
  showPreedit(text, caret) {
    const characters = Array.from(text);
    const mark = document.createElement("span");
    mark.textContent = "|";
    this.#preedit.replaceChildren(
      characters.slice(0, caret).join(""),
      mark,
      characters.slice(caret).join(""),
    );
    this.#syncVisibility();
  }

  clearPreedit() {
    this.#preedit.replaceChildren();
    this.#syncVisibility();
  }

  /// The reply carries the whole list; the page in view is the cursor's.
  showTable({ candidates, labels, cursor, cursorVisible, pageSize, vertical }) {
    const start = Math.floor(cursor / pageSize) * pageSize;
    const cells = candidates
      .slice(start, start + pageSize)
      .map((text, position) => {
        const cell = document.createElement("li");
        cell.dataset.position = position;
        cell.setAttribute(
          "aria-selected",
          String(cursorVisible && start + position === cursor),
        );
        const label = labels[position];
        cell.textContent = label ? `${label} ${text}` : text;
        return cell;
      });
    this.#candidates.replaceChildren(...cells);
    this.#candidates.toggleAttribute("data-vertical", vertical);
    this.#syncVisibility();
  }

  hideTable() {
    this.#candidates.replaceChildren();
    this.#syncVisibility();
  }

  clear() {
    this.clearPreedit();
    this.hideTable();
  }

  /// Under the caret's line, kept inside the editor; above the line when the
  /// window has no room below it. A caret scrolled out of view leaves the panel
  /// at the textarea's edge.
  place({ left, top, lineHeight }) {
    const root = this.#root;
    const room = root.offsetParent.clientWidth - root.offsetWidth;
    root.style.left = `${Math.max(0, Math.min(left, room))}px`;
    const inView = Math.max(0, Math.min(top, root.offsetParent.clientHeight));
    const below = inView + lineHeight;
    const editorTop = root.offsetParent.getBoundingClientRect().top;
    const isRoomBelow =
      editorTop + below + root.offsetHeight <= window.innerHeight;
    root.style.top = `${isRoomBelow ? below : inView - root.offsetHeight}px`;
  }

  #syncVisibility() {
    this.#root.hidden =
      !this.#preedit.hasChildNodes() && !this.#candidates.hasChildNodes();
  }
}
