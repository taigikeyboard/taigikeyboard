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
