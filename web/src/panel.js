// The composition and the candidate list, drawn at the caret. Nothing here
// reaches the textarea until a commit, so its undo stack holds only committed
// words (docs/architecture/web-roadmap.md § Text surface).

function span(className, text) {
  const element = document.createElement("span");
  element.className = className;
  element.textContent = text;
  return element;
}

/// One candidate: its key, its script, and the other script when it has one. A
/// `label` of `""` is the §34 literal, which takes no key and is centred across
/// its cell (`CandidateItemView.isLiteralCell` on macOS); the Telex guide's rows
/// have no label at all.
function drawCell({ text, annotation }, label, position, { isCurrent }) {
  const cell = document.createElement("li");
  cell.dataset.position = position;
  cell.setAttribute("aria-current", String(isCurrent));
  cell.toggleAttribute("data-literal", label === "");
  cell.append(span("index", label ?? ""), span("text", text));
  if (annotation) {
    cell.append(span("annotation", annotation));
  }
  return cell;
}

export class Panel {
  #root;
  #preedit;
  #candidates;
  #arrows;
  #pageUp;
  #pageDown;
  #spoken;

  /// `onPick(position)`: a click on the `position`-th cell of the shown page.
  /// `onTurnPage(isForward)`: a click on a page arrow.
  /// `spoken`: a live region outside `root`, which `hidden` would silence.
  constructor(root, spoken, onPick, onTurnPage) {
    this.#root = root;
    this.#spoken = spoken;
    this.#preedit = root.querySelector("#preedit");
    this.#candidates = root.querySelector("#candidates");
    this.#arrows = root.querySelector("#page-arrows");
    this.#pageUp = root.querySelector("#page-up");
    this.#pageDown = root.querySelector("#page-down");
    this.#pageUp.addEventListener("click", () => onTurnPage(false));
    this.#pageDown.addEventListener("click", () => onTurnPage(true));
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
  showTable({ cells, labels, cursor, cursorVisible, pageSize, vertical }) {
    const start = Math.floor(cursor / pageSize) * pageSize;
    const shown = cells.slice(start, start + pageSize);
    this.#candidates.replaceChildren(
      ...shown.map((cell, position) =>
        drawCell(cell, labels[position], position, {
          isCurrent: cursorVisible && start + position === cursor,
        }),
      ),
    );
    // Focus stays in the textarea, so a screen reader hears the list only here:
    // the one cell Enter would take, not the page on every key.
    const position = cursorVisible ? cursor - start : 0;
    this.#spoken.textContent = [
      labels[position],
      shown[position]?.text,
      shown[position]?.annotation,
    ]
      .filter(Boolean)
      .join(" ");
    this.#candidates.toggleAttribute("data-vertical", vertical);
    // A column has no arrows on macOS either: it shows a scroller there.
    this.#arrows.hidden = vertical || cells.length <= pageSize;
    this.#pageUp.disabled = start === 0;
    this.#pageDown.disabled = start + pageSize >= cells.length;
    this.#syncVisibility();
  }

  hideTable() {
    this.#candidates.replaceChildren();
    this.#arrows.hidden = true;
    this.#spoken.textContent = "";
    this.#syncVisibility();
  }

  clear() {
    this.clearPreedit();
    this.hideTable();
  }

  /// Under the caret's line, kept inside the editor; above the line when the
  /// editor or the window has no room below it, so Copy and Clear stay clear,
  /// but never past the editor's top, where it could not be scrolled to.
  /// A caret scrolled out of view leaves the panel at the textarea's edge.
  place({ left, top, lineHeight }) {
    const root = this.#root;
    const editor = root.offsetParent;
    const roomRight = editor.clientWidth - root.offsetWidth;
    const inView = Math.max(0, Math.min(top, editor.clientHeight));
    const below = inView + lineHeight;
    const room = Math.min(
      editor.clientHeight,
      window.innerHeight - editor.getBoundingClientRect().top,
    );
    const isRoomBelow = below + root.offsetHeight <= room;
    root.style.left = `${Math.max(0, Math.min(left, roomRight))}px`;
    const above = Math.max(0, inView - root.offsetHeight);
    root.style.top = `${isRoomBelow ? below : above}px`;
  }

  #syncVisibility() {
    this.#root.hidden =
      !this.#preedit.hasChildNodes() && !this.#candidates.hasChildNodes();
  }
}
