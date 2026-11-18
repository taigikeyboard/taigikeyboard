// The options page of the ChromeOS input method and of the browser extension:
// one shell (`chromeos/options.html`, styled by `chromeos/options.css`) around
// the web page's settings form, cut at build time so all three keep one
// markup. Rows a package cannot honour are hidden, not removed:
// `Settings.#show` reads every control. Any drift in the page's markup fails
// the build rather than shipping a wrong form. ChromeOS cuts with `CHROMEOS`,
// the browser extension with its own list (`scripts/stage-extension.mjs`).

/**
 * @typedef {object} OptionsCut
 * @property {string} title
 * @property {string[]} hiddenControls
 * @property {string[]} removedOptions
 */

/**
 * What ChromeOS takes out of the form.
 *
 * @type {OptionsCut}
 */
const CHROMEOS = {
  /** Sets the page apart from the browser extension's (chromeos-release.md). */
  title: "台語齒盤 ChromeOS",
  /**
   * Rows with no effect on ChromeOS: it draws the candidate window itself
   * and has no key panel.
   */
  hiddenControls: [
    "tpsKeyboardShown",
    "candidateAppearanceMode",
    "fontType",
    "candidateTextSize",
  ],
  /**
   * The worker cannot measure cells, and the system window has no
   * expandable layout (chromeos-roadmap.md D4).
   */
  removedOptions: ["expandable", "verticalExpandable"],
};

const SECTION_TAG = '<details class="sidebar-section"';
const SUMMARY = /<summary data-i18n="([^"]+)">([^<]*)<\/summary>/;

/**
 * @typedef {object} OptionsMarkup
 * @property {string} form
 * @property {string} nav
 */

/**
 * `tag` (an opening tag such as `<details class="x">`) with `attribute`
 * added before its closing `>`.
 *
 * @param {string} tag
 * @param {string} attribute
 * @returns {string}
 */
function withAttribute(tag, attribute) {
  if (!tag.endsWith(">")) {
    throw new Error(`options form: ${tag} is not a whole opening tag`);
  }
  return `${tag.slice(0, -1)} ${attribute}>`;
}

/**
 * @param {string[]} lines
 * @param {string} needle
 * @returns {number}
 */
function onlyIndex(lines, needle) {
  const found = lines.flatMap((line, index) =>
    line.includes(needle) ? [index] : [],
  );
  if (found.length !== 1) {
    throw new Error(
      `options form: expected one line with ${needle}, found ${found.length}`,
    );
  }
  return found[0];
}

/**
 * Adds `hidden` to the nearest line above `from` that opens with `opening`.
 *
 * @param {string[]} lines
 * @param {number} from
 * @param {string} opening
 * @returns {void}
 */
function hideEnclosing(lines, from, opening) {
  for (let index = from; index >= 0; index -= 1) {
    if (lines[index].includes(opening)) {
      lines[index] = lines[index].replace(
        opening,
        withAttribute(opening, "hidden"),
      );
      return;
    }
  }
  throw new Error(`options form: no ${opening} above line ${from}`);
}

/**
 * Lays every section out as chrome://settings does: unfolded, with an id
 * (`section-1`, … in the page's order), its summary a heading rather than a
 * disclosure button over a card holding the rest. The side menu links each
 * shown section by its summary's key. The `<details>` stays: a reset button
 * finds its section with `closest("details")` (`Settings.#resetSection`).
 *
 * @param {string[]} lines
 * @returns {OptionsMarkup}
 */
function asCards(lines) {
  /** @type {string[]} */
  const links = [];
  let section = 0;
  const form = lines.flatMap((line, index) => {
    if (line.includes(SECTION_TAG)) {
      const summary = lines[index + 1]?.match(SUMMARY);
      if (!summary) {
        throw new Error(`options form: no summary under ${line.trim()}`);
      }
      section += 1;
      const id = `section-${section}`;
      const tag = line.trim();
      if (!tag.includes(" hidden")) {
        links.push(
          `<a href="#${id}" data-i18n="${summary[1]}">${summary[2]}</a>`,
        );
      }
      return [
        line.replace(
          tag,
          withAttribute(withAttribute(tag, `id="${id}"`), "open"),
        ),
      ];
    }
    if (SUMMARY.test(line)) {
      return [
        line.replace(
          "<summary ",
          '<summary role="heading" aria-level="2" tabindex="-1" ',
        ),
        line.replace(/<summary.*/, '<div class="options-card">'),
      ];
    }
    if (line.includes("</details>")) {
      if (line.trim() !== "</details>") {
        throw new Error(
          `options form: ${line.trim()} is not a lone </details>`,
        );
      }
      return [line.replace("</details>", "</div>"), line];
    }
    return [line];
  });
  return {
    form: form.join("\n"),
    nav: `<nav class="options-nav">\n${links.join("\n")}\n</nav>`,
  };
}

/**
 * `<form id="settings">…</form>` of `indexHtml`, without what `cut` names:
 * `hiddenControls`, rows to hide, and `removedOptions`, options to drop;
 * every section laid out as a card, and the side menu of the shown ones
 * (`asCards`).
 *
 * @param {string} indexHtml
 * @param {OptionsCut} [cut]
 * @returns {OptionsMarkup}
 */
export function optionsMarkup(indexHtml, cut = CHROMEOS) {
  const lines = indexHtml.split("\n");
  const start = onlyIndex(lines, '<form id="settings">');
  const end = onlyIndex(lines, "</form>");
  const form = lines.slice(start, end + 1);
  for (const name of cut.hiddenControls) {
    hideEnclosing(
      form,
      onlyIndex(form, `name="${name}"`),
      '<label class="setting">',
    );
  }
  for (const option of cut.removedOptions) {
    form.splice(onlyIndex(form, `<option value="${option}"`), 1);
  }
  return asCards(form);
}

/**
 * The shell line that `mark` matches, replaced by `markup`.
 *
 * @param {string} shell
 * @param {RegExp} mark
 * @param {string} markup
 * @returns {string}
 */
function fill(shell, mark, markup) {
  if (!mark.test(shell)) {
    throw new Error(`options page: no ${mark.source} line to replace`);
  }
  // A function, so a `$` in the markup is never read as a pattern.
  return shell.replace(mark, () => markup);
}

/**
 * `shell` (web/chromeos/options.html) with its `<title>` set to `cut.title`,
 * its `<!-- nav: … -->` line replaced by the side menu and its
 * `<!-- form: … -->` line by the form of `indexHtml`, both cut as
 * `optionsMarkup` does.
 *
 * @param {string} shell
 * @param {string} indexHtml
 * @param {OptionsCut} [cut]
 * @returns {string}
 */
export function optionsPage(shell, indexHtml, cut = CHROMEOS) {
  const { form, nav } = optionsMarkup(indexHtml, cut);
  const titled = fill(
    shell,
    /<title>[^<]*<\/title>/,
    `<title>${cut.title}</title>`,
  );
  const navigated = fill(titled, /^\s*<!-- nav:.*-->$/m, nav);
  return fill(navigated, /^\s*<!-- form:.*-->$/m, form);
}
