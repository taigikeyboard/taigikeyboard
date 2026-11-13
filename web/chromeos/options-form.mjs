// The ChromeOS options page's form, cut from the web page's settings form at
// build time so both keep one markup. Rows ChromeOS cannot honour are hidden,
// not removed: `Settings.#show` reads every control. Any drift in the page's
// markup fails the build rather than shipping a wrong form. The browser
// extension cuts its own with another list (`scripts/stage-extension.mjs`).

/// What ChromeOS takes out of the form.
const CHROMEOS = {
  /// Rows with no effect on ChromeOS: it draws the candidate window itself
  /// and has no key panel.
  hiddenControls: [
    "tpsKeyboardShown",
    "candidateAppearanceMode",
    "fontType",
    "candidateTextSize",
  ],
  /// The worker cannot measure cells, and the system window has no
  /// expandable layout (chromeos-roadmap.md D4).
  removedOptions: ["expandable", "verticalExpandable"],
};

/// The page fills this section from the session; the options page has none.
const HIDDEN_SECTION_MARK = '<div id="shortcuts"></div>';

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

/// Adds `hidden` to the nearest line above `from` that opens with `opening`.
function hideEnclosing(lines, from, opening) {
  for (let index = from; index >= 0; index -= 1) {
    if (lines[index].includes(opening)) {
      lines[index] = lines[index].replace(
        opening,
        opening.replace(">", " hidden>"),
      );
      return;
    }
  }
  throw new Error(`options form: no ${opening} above line ${from}`);
}

/// `<form id="settings">…</form>` of `indexHtml`, without what `cut` names:
/// `hiddenControls`, rows to hide, and `removedOptions`, options to drop.
export function optionsForm(indexHtml, cut = CHROMEOS) {
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
  hideEnclosing(
    form,
    onlyIndex(form, HIDDEN_SECTION_MARK),
    '<details class="sidebar-section">',
  );
  for (const option of cut.removedOptions) {
    form.splice(onlyIndex(form, `<option value="${option}"`), 1);
  }
  return form.join("\n");
}

const FORM_MARK = /^\s*<!-- form:.*-->$/m;

/// `shell` (web/chromeos/options.html) with its `<!-- form: … -->` line
/// replaced by the form of `indexHtml`, cut as `optionsForm` does.
export function optionsPage(shell, indexHtml, cut = CHROMEOS) {
  if (!FORM_MARK.test(shell)) {
    throw new Error("options page: no <!-- form: … --> line to replace");
  }
  const form = optionsForm(indexHtml, cut);
  // A function, so a `$` in the markup is never read as a pattern.
  return shell.replace(FORM_MARK, () => form);
}
