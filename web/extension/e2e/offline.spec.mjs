// Typing needs nothing but the extension's own package: no download, no
// server (docs/architecture/browser-extension.md § Package).

import { A, commitWord, expect, openTyping, test } from "./helpers.mjs";

test("nothing is fetched from anywhere but the site and the package", async ({
  context,
  worker,
  page,
}) => {
  const requested = [];
  context.on("request", (request) => requested.push(request.url()));
  // Anything else would fail here rather than quietly succeed.
  await context.route(
    (url) => url.protocol !== "chrome-extension:" && url.hostname !== "a.test",
    (route) => route.abort(),
  );
  const field = await openTyping(
    context,
    worker,
    page,
    `${A}/fields.html`,
    "#text",
  );
  await commitWord(page);
  await expect(field).toHaveValue("台灣");
  const elsewhere = requested.filter((url) => {
    const { protocol, hostname } = new URL(url);
    return protocol !== "chrome-extension:" && hostname !== "a.test";
  });
  expect(elsewhere).toEqual([]);
});
