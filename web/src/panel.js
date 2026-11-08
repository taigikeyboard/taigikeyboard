// The composition and the candidate list, drawn at the caret. Nothing here
// reaches the textarea until a commit, so its undo stack holds only committed
// words (docs/architecture/web-roadmap.md § Text surface).

function span(className, text) {
  const element = document.createElement("span");
  element.className = className;
  element.textContent = text;
  return element;
}

/// One candidate: its key, its script, and the other script when it has one.
/// `isLiteral`: the §34 literal, which takes no key and is centred across its
/// cell (`CandidateItemView.isLiteralCell` on macOS). The Telex guide's rows have
/// no label at all.
function drawCell({ text, annotation }, label, position, flags) {
  const { isCurrent, isLiteral } = flags;
  const cell = document.createElement("li");
  cell.dataset.position = position;
  cell.setAttribute("aria-current", String(isCurrent));
  cell.toggleAttribute("data-literal", isLiteral);
  cell.append(span("index", label ?? ""), span("text", text));
  if (annotation) {
    cell.append(span("annotation", annotation));
  }
  return cell;
}

/// Between the composition and a list beside it.
const LIST_GAP = 8;

export class Panel {
  #root;
  #editor;
  #preedit;
  #list;
  #candidates;
  #arrows;
  #pageUp;
  #pageDown;
  #unfold;
  #ruler;
  #spoken;
  /// The `layout` on screen, `null` for a paged list.
  #shownLayout = null;
  /// The cells on screen, to tell a fold from a new list.
  #shownCells = null;
  /// The mode being left, for the next `place` (`#startUnfold`): the window's
  /// size and a copy of the list as drawn.
  #unfoldFrom = null;
  #unfolding = [];
  /// That copy while the change plays: the drawings the new mode does not have.
  #ghost = null;

  /// `onPick(position)`: a click on a cell, `data-position` as drawn.
  /// `onTurnPage(isForward)`: a click on a page arrow.
  /// `onScrolled(offset)`: the wheel moved the unfolded grid.
  /// `ruler`: outside `root`, which `hidden` would stop laying out.
  /// `spoken`: a live region outside `root`, which `hidden` would silence.
  constructor(root, ruler, spoken, onPick, onTurnPage, onScrolled) {
    this.#root = root;
    this.#ruler = ruler;
    this.#spoken = spoken;
    this.#editor = root.parentElement;
    this.#preedit = this.#editor.querySelector("#preedit");
    this.#list = root.querySelector("#list");
    this.#candidates = root.querySelector("#candidates");
    this.#arrows = root.querySelector("#page-arrows");
    this.#pageUp = root.querySelector("#page-up");
    this.#pageDown = root.querySelector("#page-down");
    this.#unfold = root.querySelector("#unfold");
    this.#pageUp.addEventListener("click", () => onTurnPage(false));
    this.#pageDown.addEventListener("click", () => onTurnPage(true));
    // A collapsed list's page down is its unfolding (`navigate_collapsed`).
    this.#unfold.addEventListener("click", () => onTurnPage(true));
    this.#candidates.addEventListener("scroll", () => {
      onScrolled(this.#candidates.scrollTop);
    });
    // Focus stays in the textarea, which a commit needs.
    for (const part of [root, this.#preedit]) {
      part.addEventListener("mousedown", (event) => event.preventDefault());
    }
    this.#candidates.addEventListener("click", (event) => {
      const cell = event.target.closest("li");
      if (cell) {
        onPick(Number(cell.dataset.position));
      }
    });
  }

  get isShown() {
    return !this.#root.hidden || this.#preedit.hasChildNodes();
  }

  contains(node) {
    return this.#root.contains(node) || this.#preedit.contains(node);
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
  }

  clearPreedit() {
    this.#preedit.replaceChildren();
  }

  /// `[maximumWindowWidth, ...widths]` of `[text, fontSize]` runs, as drawn: the
  /// DOM rather than a canvas, whose widths fall short in WebKit under the
  /// default font (web-guidelines.md). One write and one read.
  measure(runsJson) {
    const spans = JSON.parse(runsJson).map(([text, fontSize]) => {
      const run = span("", text);
      run.style.fontSize = `${fontSize}px`;
      return run;
    });
    const windowWidth = this.#ruler.parentElement.clientWidth;
    this.#ruler.replaceChildren(...spans);
    const widths = spans.map((run) => run.getBoundingClientRect().width);
    this.#ruler.replaceChildren();
    return Float32Array.of(windowWidth, ...widths);
  }

  showTable(table) {
    this.#endUnfold();
    if (table.layout) {
      this.#showLaidOut(table);
    } else {
      this.#showPage(table);
    }
    this.#syncVisibility();
  }

  /// A list `ExpandableListModel` holds: a packed row or a column, or the grid.
  #showLaidOut({ cells, labels, cursor, layout }) {
    const list = this.#candidates;
    const { collapsed, rows, isColumn, isExpanded } = layout;
    // A new list arrives collapsed too; only the same cells fold.
    const cellsJson = JSON.stringify(cells);
    this.#unfoldFrom =
      layout.transition &&
      this.#shownLayout &&
      this.#shownLayout.isExpanded !== isExpanded &&
      this.#shownCells === cellsJson
        ? this.#snapshot()
        : null;
    this.#shownLayout = layout;
    this.#shownCells = cellsJson;
    const labelOf = new Map(
      layout.keyed.map((index, at) => [index, labels[at]]),
    );
    const draw = (index) =>
      drawCell(cells[index], labelOf.get(index), index, {
        isCurrent: index === cursor,
        isLiteral: layout.leadCellIsUnkeyed && index === 0,
      });
    const drawn = isExpanded
      ? rows.flatMap((row, at) =>
          row.map(({ index, start, span: columns }) => {
            const cell = draw(index);
            cell.style.gridRow = `${at + 1}`;
            cell.style.gridColumn = `${start + 1} / span ${columns}`;
            return cell;
          }),
        )
      : collapsed.map(({ index, width }) => {
          const cell = draw(index);
          if (!isColumn) {
            cell.style.minWidth = `${width}px`;
          }
          return cell;
        });
    this.#setShape({
      row: !isColumn && !isExpanded,
      grid: isExpanded,
      vertical: isColumn && !isExpanded,
      inline: isColumn,
    });
    const lengths = {
      "--candidate-item": layout.itemHeight,
      // A column keeps its height as it unfolds, so its rows are the model's.
      "--candidate-row": layout.itemHeight,
      "--candidate-column": layout.columnWidth,
      "--candidate-window": layout.windowHeight,
      "--candidate-grid": layout.gridHeight,
      "--candidate-peek": layout.peek,
      "--candidate-band-x": layout.keyedBand?.x ?? 0,
      "--candidate-band-y": layout.keyedBand?.y ?? 0,
      "--candidate-band-width": layout.keyedBand?.width ?? 0,
      "--candidate-band-height": layout.keyedBand?.height ?? 0,
      "--candidate-primary": layout.primaryColumn ?? 0,
    };
    for (const [name, length] of Object.entries(lengths)) {
      list.style.setProperty(name, `${length}px`);
    }
    // A count, not a length: the `px` loop above would break `repeat()`.
    list.style.setProperty("--candidate-columns", `${layout.columns}`);
    list.replaceChildren(...drawn);
    if (isExpanded) {
      list.scrollTop = layout.scrollY;
    }
    this.#speak(labelOf.get(cursor), cells[cursor]);
    // An unfolded column that pages keeps its chevron column on every page.
    this.#arrows.hidden = isExpanded ? !layout.isPaged : !layout.hasOverflow;
    this.#pageUp.hidden = true;
    this.#pageDown.hidden = true;
    this.#unfold.hidden = isExpanded && !layout.hasNextPage;
    this.#unfold.toggleAttribute("data-column", isColumn);
  }

  /// The reply carries the whole list; the page in view is the cursor's.
  #showPage({ cells, labels, cursor, cursorVisible, pageSize, vertical }) {
    this.#forgetLayout();
    this.#setShape({ vertical, inline: vertical });
    this.#candidates.style.removeProperty("--candidate-row");
    const start = Math.floor(cursor / pageSize) * pageSize;
    const shown = cells.slice(start, start + pageSize);
    this.#candidates.replaceChildren(
      ...shown.map((cell, position) =>
        drawCell(cell, labels[position], position, {
          isCurrent: cursorVisible && start + position === cursor,
          isLiteral: labels[position] === "",
        }),
      ),
    );
    const position = cursorVisible ? cursor - start : 0;
    this.#speak(labels[position], shown[position]);
    // A column has no arrows on macOS either: it shows a scroller there.
    this.#arrows.hidden = vertical || cells.length <= pageSize;
    this.#pageUp.disabled = start === 0;
    this.#pageDown.disabled = start + pageSize >= cells.length;
    this.#pageUp.hidden = false;
    this.#pageDown.hidden = false;
    this.#unfold.hidden = true;
  }

  /// `inline`: a cell holds its scripts side by side.
  #setShape({ row, grid, vertical, inline }) {
    const list = this.#candidates;
    list.toggleAttribute("data-row", Boolean(row));
    list.toggleAttribute("data-grid", Boolean(grid));
    list.toggleAttribute("data-vertical", Boolean(vertical));
    list.toggleAttribute("data-inline", Boolean(inline));
  }

  #forgetLayout() {
    this.#shownLayout = null;
    this.#shownCells = null;
    this.#unfoldFrom = null;
  }

  /// Focus stays in the textarea, so a screen reader hears the list only here:
  /// the one cell Enter would take, not the page on every key.
  #speak(label, cell) {
    this.#spoken.textContent = [label, cell?.text, cell?.annotation]
      .filter(Boolean)
      .join(" ");
  }

  /// The window's size and a still copy of the list where it is drawn, ids
  /// dropped, taken before the new mode replaces it. Classes, not ids, style
  /// the list, so the copy draws as the original did.
  #snapshot() {
    const window = this.#root.getBoundingClientRect();
    const list = this.#list.getBoundingClientRect();
    const ghost = this.#list.cloneNode(true);
    for (const element of [ghost, ...ghost.querySelectorAll("[id]")]) {
      element.removeAttribute("id");
    }
    ghost.setAttribute("aria-hidden", "true");
    ghost.dataset.ghost = "";
    Object.assign(ghost.style, {
      position: "absolute",
      left: `${list.left - window.left}px`,
      top: `${list.top - window.top}px`,
      width: `${list.width}px`,
      height: `${list.height}px`,
      pointerEvents: "none",
      // Under the list, over the window's glass: the band it carries lies
      // under the cells, as the keyed band always does.
      zIndex: "-1",
    });
    return { width: window.width, height: window.height, ghost };
  }

  /// Plays the model's `ModeTransition` as macOS does: the window eases to
  /// its new size while every drawing moves and fades from its old frame to
  /// its new one — the new mode's cells on the list, the old mode's on the
  /// still copy laid over it — the chevron with them, and the keyed band
  /// fading in on the grid or out with it. Tracks carry frames; only their
  /// differences are used, so every drawing moves relative to where the
  /// page drew it. After `place`: an animation changes what `offsetHeight`
  /// reads.
  #startUnfold() {
    const from = this.#unfoldFrom;
    this.#unfoldFrom = null;
    if (!from || matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }
    // Set whenever `from` is: `#showLaidOut` copies only for a transition.
    const { transition } = this.#shownLayout;
    const timing = {
      duration: this.#shownLayout.animationMilliseconds,
      easing: "ease-in-out",
    };
    const to = this.#root.getBoundingClientRect();
    // The list holds its final size while the window eases around it, as
    // the Mac's cells lie still under the moving frame: every track's
    // difference is then measured from where the cell will stay.
    const list = this.#list.getBoundingClientRect();
    Object.assign(this.#list.style, {
      width: `${list.width}px`,
      height: `${list.height}px`,
    });
    const { ghost } = from;
    this.#root.append(ghost);
    this.#ghost = ghost;
    const window = this.#root.animate(
      {
        width: [`${from.width}px`, `${to.width}px`],
        height: [`${from.height}px`, `${to.height}px`],
      },
      timing,
    );
    window.onfinish = () => {
      if (this.#unfolding[0] === window) {
        this.#endUnfold();
      }
    };
    this.#unfolding = [window];
    const play = (element, keyframes, pseudoElement) => {
      this.#unfolding.push(
        element.animate(keyframes, { ...timing, pseudoElement }),
      );
    };
    const offset = (a, b) => `${a.x - b.x}px ${a.y - b.y}px`;
    const still = "0px 0px";
    const byPosition = (list) =>
      new Map([...list.children].map((cell) => [cell.dataset.position, cell]));
    const ghostList = ghost.querySelector(".candidates");
    const drawn = byPosition(this.#candidates);
    const left = byPosition(ghostList);
    const incoming = transition.isExpanding ? "expanded" : "collapsed";
    // A new-mode drawing eases into where the page drew it; an old-mode one,
    // on the copy, eases out of it. Widths are read before anything moves.
    const moves = transition.cells.flatMap((track) => {
      const isIncoming = track.layer === incoming || track.layer === "shared";
      const cell = (isIncoming ? drawn : left).get(String(track.index));
      return cell
        ? [
            {
              cell,
              isIncoming,
              track,
              width: cell.getBoundingClientRect().width,
            },
          ]
        : [];
    });
    const moved = new Set();
    for (const { cell, isIncoming, track, width } of moves) {
      const { from: start, to: end } = track;
      const keyframes = {
        translate: isIncoming
          ? [offset(start, end), still]
          : [still, offset(end, start)],
        opacity: [track.fromAlpha, track.toAlpha],
      };
      // A cell that changes width eases it too, by the model's difference
      // from the width the page drew — a collapsed column's cells are
      // narrower than the model's frame.
      const grow = end.width - start.width;
      if (grow !== 0) {
        keyframes.width = isIncoming
          ? [`${width - grow}px`, `${width}px`]
          : [`${width}px`, `${width + grow}px`];
      }
      if (!isIncoming) {
        moved.add(cell);
      }
      play(cell, keyframes);
    }
    // The copy keeps only the drawings the new mode has no twin for.
    for (const cell of left.values()) {
      if (!moved.has(cell)) {
        cell.style.visibility = "hidden";
      }
    }
    const { chevron } = transition;
    const chevronOpacity = [chevron.fromAlpha, chevron.toAlpha];
    const ghostArrows = ghost.querySelector(".page-arrows");
    if (this.#arrows.hidden) {
      play(ghostArrows, {
        translate: [still, offset(chevron.to, chevron.from)],
        opacity: chevronOpacity,
      });
    } else {
      ghostArrows.hidden = true;
      play(this.#arrows, {
        translate: [offset(chevron.from, chevron.to), still],
        opacity: chevronOpacity,
      });
    }
    play(
      transition.isExpanding ? this.#candidates : ghostList,
      { opacity: transition.isExpanding ? [0, 1] : [1, 0] },
      "::before",
    );
  }

  /// Stops a mode change mid-way or once played: every drawing back to where
  /// the page put it, the copy gone.
  #endUnfold() {
    for (const animation of this.#unfolding) {
      animation.cancel();
    }
    this.#unfolding = [];
    this.#ghost?.remove();
    this.#ghost = null;
    this.#list.style.removeProperty("width");
    this.#list.style.removeProperty("height");
  }

  hideTable() {
    this.#endUnfold();
    this.#forgetLayout();
    this.#setShape({});
    this.#candidates.replaceChildren();
    this.#arrows.hidden = true;
    this.#spoken.textContent = "";
    this.#syncVisibility();
  }

  clear() {
    this.clearPreedit();
    this.hideTable();
  }

  /// The composition on the caret's own line, over the text there, and the list
  /// under that line, or above it, or beside it where neither has the room: it
  /// never covers the composition, Copy or Clear. A caret scrolled out of view
  /// leaves both at the textarea's edge.
  place({ left, top, lineHeight }) {
    const editor = this.#editor;
    const preedit = this.#preedit;
    const lineTop = Math.max(0, Math.min(top, editor.clientHeight));
    const start = Math.max(
      0,
      Math.min(left, editor.clientWidth - preedit.offsetWidth),
    );
    preedit.style.left = `${start}px`;
    const height = preedit.offsetHeight || lineHeight;
    const line = Math.max(0, Math.min(lineTop, editor.clientHeight - height));
    preedit.style.top = `${line}px`;

    const room = Math.min(
      editor.clientHeight,
      window.innerHeight - editor.getBoundingClientRect().top,
    );
    this.#fitGrid(room);
    const root = this.#root;
    const below = line + height;
    const above = line - root.offsetHeight;
    const roomRight = editor.clientWidth - root.offsetWidth;
    let listLeft = Math.max(0, Math.min(start, roomRight));
    let listTop = below;
    if (below + root.offsetHeight > room) {
      listTop = above;
      if (above < 0) {
        const after = start + preedit.offsetWidth + LIST_GAP;
        const before = start - root.offsetWidth - LIST_GAP;
        listLeft = after <= roomRight ? after : Math.max(0, before);
        listTop = Math.max(0, Math.min(line, room - root.offsetHeight));
      }
    }
    root.style.left = `${listLeft}px`;
    root.style.top = `${listTop}px`;
    this.#startUnfold();
  }

  /// The model's five rows and a half can pass a short editor, which would
  /// cover Copy and Clear: the grid takes what there is and scrolls.
  #fitGrid(room) {
    const list = this.#candidates;
    if (!list.hasAttribute("data-grid")) {
      return;
    }
    list.style.removeProperty("max-height");
    const excess = this.#root.offsetHeight - room;
    if (excess > 0) {
      list.style.maxHeight = `${list.offsetHeight - excess}px`;
      list
        .querySelector('[aria-current="true"]')
        ?.scrollIntoView({ block: "nearest" });
    }
  }

  #syncVisibility() {
    this.#root.hidden = !this.#candidates.hasChildNodes();
  }
}
