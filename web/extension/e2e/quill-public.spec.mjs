// A smoke test against Quill's public editor, which needs no account. It
// reaches the network, so it runs only when asked (EXTENSION_PUBLIC=1): the
// pinned fixture (`quill.spec.mjs`) is the gate, and this site being down is
// not the extension's regression.

import {
  candidates,
  commitWord,
  expect,
  preedit,
  test,
  turnOn,
} from "./helpers.mjs";

test("a word commits into the editor on quilljs.com", async ({
  context,
  worker,
  page,
}) => {
  test.skip(!process.env.EXTENSION_PUBLIC, "EXTENSION_PUBLIC=1 runs it");
  await page.goto("https://quilljs.com/");
  await turnOn(context, worker, page);
  // The page stacks three demo editors and shows the Snow one; part of it
  // lies under the page's header, so the focus is given, not clicked.
  const editor = page.locator(".ql-snow .ql-editor").first();
  await editor.evaluate((element) => element.focus());
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  await page.keyboard.press("Enter");
  await expect(editor).toHaveText("台灣");
  await commitWord(page);
  await expect(editor).toHaveText("台灣台灣");
  // Quill joins edits less than a second apart into one step: its undo takes
  // back both commits and the replaced text returns.
  await page.keyboard.press("ControlOrMeta+z");
  await expect(editor).toContainText("Quill Rich Text Editor");
  await expect(editor).not.toContainText("台灣");
});
