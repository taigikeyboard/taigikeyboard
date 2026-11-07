// Learning kept across visits, one tab learning at a time
// (docs/architecture/web-roadmap.md § Phase 8). Words: `tests/learning.test.mjs`.

import { expect, test } from "@playwright/test";

import { candidates, open } from "./helpers.mjs";

const RAW = "kausiu";
const PICKED = "狗岫";

/// The first cell a slot key picks, past the literal roman row.
const topCell = (page) =>
  candidates(page)
    .filter({ has: page.locator(".index", { hasText: /^.+$/ }) })
    .first()
    .locator(".text");

async function expectTop(page, editor, hanji) {
  await expect(async () => {
    await editor.pressSequentially(RAW);
    try {
      await expect(topCell(page)).toHaveText(hanji, { timeout: 500 });
    } finally {
      await editor.press("Escape");
    }
  }).toPass();
}

/// Clicks `PICKED` and commits it, three times: enough to rank it first.
async function pickThrice(page, editor) {
  for (let round = 0; round < 3; round += 1) {
    await editor.pressSequentially(RAW);
    await candidates(page).filter({ hasText: PICKED }).first().click();
    await editor.press("Enter");
  }
  await expect(editor).toHaveValue(PICKED.repeat(3));
}

/// Waits for the page's copy in IndexedDB, written after the commits.
function expectSaved(page) {
  return expect
    .poll(() =>
      page.evaluate(async () => {
        const { readSnapshot } = await import("./learning.js");
        return Object.keys((await readSnapshot()) ?? {}).length;
      }),
    )
    .toBe(4);
}

test("a pick still ranks first after a reload", async ({ page }) => {
  const editor = await open(page);
  await expectTop(page, editor, "教授");
  await pickThrice(page, editor);
  await expectTop(page, editor, PICKED);
  await expectSaved(page);
  await page.reload();
  await expect(editor).toBeFocused();
  await expectTop(page, editor, PICKED);
});

test("a second tab learns only once the first has closed", async ({
  context,
  page,
}) => {
  const first = await open(page);
  await pickThrice(page, first);
  await expectSaved(page);

  const second = await context.newPage();
  const editor = await open(second);
  // Not the lock holder: it neither read the first tab's copy nor learns.
  await expectTop(second, editor, "教授");

  await page.close();
  // The lock passes: the second tab takes the first one's copy.
  await expectTop(second, editor, PICKED);
});

test("a tab closed right after its picks hands them on", async ({
  context,
  page,
}) => {
  const first = await open(page);
  await pickThrice(page, first);
  // No wait for the copy: each commit's save starts at once, and nothing is
  // saved on the way out. Closed without unload: Playwright's WebKit keeps a
  // page's Web Lock after `runBeforeUnload`.
  await page.close();

  const next = await context.newPage();
  const editor = await open(next);
  await expectTop(next, editor, PICKED);
});

test("a new instance after a trap learns from the saved copy", async ({
  page,
}) => {
  const editor = await open(page);
  await pickThrice(page, editor);
  await expectSaved(page);

  // What a panic reaches the page as (`typing.spec.mjs`).
  await page.evaluate(async () => {
    const { Session } = await import("./pkg/taigi_web.js?instance=1");
    Session.prototype.keyDown = () => {
      throw new WebAssembly.RuntimeError("unreachable");
    };
  });
  await editor.press("a");

  await expectTop(page, editor, PICKED);
});

/// A digest of the page's copy in IndexedDB, `""` before the first save.
const savedDigest = (page) =>
  page.evaluate(async () => {
    const { readSnapshot } = await import("./learning.js");
    const files = (await readSnapshot()) ?? {};
    const names = Object.keys(files).sort();
    const bytes = new Uint8Array(
      await new Blob(names.map((n) => files[n])).arrayBuffer(),
    );
    const digest = await crypto.subtle.digest("SHA-256", bytes);
    return names.length === 0
      ? ""
      : btoa(String.fromCharCode(...new Uint8Array(digest)));
  });

/// Runs `change` and waits for the copy it saves to land: a reload aborts an
/// IndexedDB write still running, in every engine (`learning.js`).
async function expectSavedAfter(page, change) {
  await expectSaved(page);
  const before = await savedDigest(page);
  await change();
  await expect.poll(() => savedDigest(page)).not.toBe(before);
}

const records = (page) => page.locator("#learning-records");
const recordRows = (page) => page.locator("#records-rows li");

/// Folds the section and opens it again: it loads when opened, as the
/// desktop panes do, not on every commit.
async function reopenRecords(page) {
  const summary = records(page).locator("summary");
  await summary.click();
  await summary.click();
}

test("Learning Records list a pick, and a deleted one stays deleted", async ({
  page,
}) => {
  const editor = await open(page);
  await expect(records(page)).toBeVisible();
  await expect(page.locator("#records-empty")).toHaveText(
    "Nothing Learned Yet",
  );
  await pickThrice(page, editor);
  await reopenRecords(page);
  // trace: `tests/learning.test.mjs`, three picks of 狗岫 for `kausiu`.
  await expect(recordRows(page)).toHaveCount(1);
  await expect(recordRows(page).first()).toHaveText(/káu-siū\s*狗岫\s*3/);

  await expectSavedAfter(page, () =>
    recordRows(page).first().getByRole("button").click(),
  );
  await expect(recordRows(page)).toHaveCount(0);
  await expect(page.locator("#records-empty")).toBeVisible();
  await page.reload();
  await expect(recordRows(page)).toHaveCount(0);
  await expectTop(page, editor, "教授");
});

test("Delete Learning Records empties the list for good", async ({ page }) => {
  const editor = await open(page);
  await pickThrice(page, editor);
  await reopenRecords(page);
  await expect(recordRows(page)).toHaveCount(1);

  await expectSavedAfter(page, () => page.locator("#records-clear").click());
  await expect(recordRows(page)).toHaveCount(0);
  await page.reload();
  await expect(records(page)).toBeVisible();
  await expect(recordRows(page)).toHaveCount(0);
  await expectTop(page, editor, "教授");
});

test("a tab that does not learn shows no Learning Records", async ({
  context,
  page,
}) => {
  await open(page);
  await expect(records(page)).toBeVisible();

  const second = await context.newPage();
  await open(second);
  await expect(records(second)).toBeHidden();

  await page.close();
  await expect(records(second)).toBeVisible();
});

test("the filter takes a slash as text", async ({ page }) => {
  const editor = await open(page);
  await pickThrice(page, editor);
  const filter = page.locator("#records-filter");
  await filter.fill("");
  await filter.pressSequentially("k/");
  await expect(filter).toBeFocused();
  await expect(filter).toHaveValue("k/");
  await expect(page.locator("#records-empty")).toHaveText("No results");
});
