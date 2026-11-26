// Unit tests for the browser extension E2E popup-click helper
// (`web/extension/e2e/helpers.mjs` `clickClosingPopup`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { clickClosingPopup } from "../extension/e2e/helpers.mjs";

/**
 * @typedef {import("../extension/e2e/helpers.mjs").ClosablePopup} ClosablePopup
 */

/**
 * Creates a minimal deferred promise with external `resolve` and `reject`.
 * @template T
 * @returns {{
 *   promise: Promise<T>,
 *   resolve: (value: T) => void,
 *   reject: (reason?: unknown) => void,
 * }}
 */
function deferred() {
  /** @type {(value: T) => void} */
  let resolve = () => {};
  /** @type {(reason?: unknown) => void} */
  let reject = () => {};
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/**
 * Builds a `ClosablePopup` stub that records selectors, click phases, and
 * close-event registrations.
 * @param {{
 *   onTrial?: () => Promise<void>,
 *   onClick?: () => Promise<void>,
 *   onWaitForClose?: () => Promise<unknown>,
 * }} handlers
 * @returns {{
 *   popup: ClosablePopup,
 *   calls: string[],
 *   selectors: string[],
 *   waitCount: () => number,
 * }}
 */
function stubPopup(handlers) {
  /** @type {string[]} */
  const calls = [];
  /** @type {string[]} */
  const selectors = [];
  let waits = 0;
  /** @type {ClosablePopup} */
  const popup = {
    locator(selector) {
      selectors.push(selector);
      return {
        async click(options) {
          if (options?.trial) {
            calls.push("trial");
            await handlers.onTrial?.();
            return;
          }
          calls.push("click");
          await handlers.onClick?.();
        },
      };
    },
    waitForEvent(event) {
      assert.equal(event, "close");
      waits += 1;
      calls.push("waitForClose");
      return handlers.onWaitForClose
        ? handlers.onWaitForClose()
        : Promise.resolve();
    },
  };
  return { popup, calls, selectors, waitCount: () => waits };
}

test("resolves on normal path: trial, click, then close", async () => {
  const close = deferred();
  const { popup, calls, selectors } = stubPopup({
    async onClick() {
      queueMicrotask(() => close.resolve(undefined));
    },
    onWaitForClose: () => close.promise,
  });

  await clickClosingPopup(popup, "turn-on");

  assert.deepEqual(selectors, ["#turn-on"]);
  assert.deepEqual(calls, ["trial", "waitForClose", "click"]);
});

test("tolerates target-closed rejection when close fires first", async () => {
  const close = deferred();
  const click = deferred();
  const { popup } = stubPopup({
    onClick: () => click.promise,
    onWaitForClose: () => close.promise,
  });

  const running = clickClosingPopup(popup, "turn-on");
  await Promise.resolve();
  close.resolve(undefined);
  await Promise.resolve();
  click.reject(
    new Error(
      "locator.click: Target page, context or browser has been closed\nCall log:\n  - performing click action",
    ),
  );

  await running;
});

test("tolerates target-closed rejection when click rejects before close resolves", async () => {
  const close = deferred();
  const click = deferred();
  const { popup } = stubPopup({
    onClick: () => click.promise,
    onWaitForClose: () => close.promise,
  });

  const running = clickClosingPopup(popup, "turn-off");
  await Promise.resolve();
  click.reject(
    new Error(
      "locator.click: Target page, context or browser has been closed\nCall log:\n  - performing click action",
    ),
  );
  await new Promise((resolve) => setTimeout(resolve, 10));
  close.resolve(undefined);

  await running;
});

test("rejects immediately on trial failure without real click or close listener", async () => {
  const trialError = Object.assign(
    new Error("locator.click: Timeout 5000ms exceeded."),
    {
      name: "TimeoutError",
    },
  );
  const { popup, calls, waitCount } = stubPopup({
    async onTrial() {
      throw trialError;
    },
  });

  await assert.rejects(
    () => clickClosingPopup(popup, "missing"),
    (error) => {
      assert.equal(error, trialError);
      return true;
    },
  );
  assert.deepEqual(calls, ["trial"]);
  assert.equal(waitCount(), 0);
});

for (const { label, makeError } of [
  {
    label: "TimeoutError",
    makeError: () =>
      Object.assign(new Error("locator.click: Timeout 5000ms exceeded."), {
        name: "TimeoutError",
      }),
  },
  {
    label: "Page crashed",
    makeError: () => new Error("locator.click: Page crashed"),
  },
]) {
  test(`rejects immediately on genuine real-click ${label} without waiting on close or leaking rejections`, async () => {
    const close = deferred();
    const clickError = makeError();
    let closeSettled = false;
    const closeObserved = close.promise.catch(() => {
      closeSettled = true;
    });

    /** @type {unknown[]} */
    const unhandled = [];
    /** @param {unknown} reason */
    const onUnhandled = (reason) => {
      unhandled.push(reason);
    };
    process.on("unhandledRejection", onUnhandled);

    try {
      const { popup } = stubPopup({
        async onClick() {
          throw clickError;
        },
        onWaitForClose: () => close.promise,
      });

      await assert.rejects(
        () => clickClosingPopup(popup, "turn-on"),
        (error) => {
          assert.equal(error, clickError);
          return true;
        },
      );
      assert.equal(closeSettled, false);

      // Subsequent rejection of the close waiter (e.g. context disposal) must
      // not escape as an unhandled rejection.
      close.reject(new Error("Target closed after click failure"));
      await closeObserved;
      await new Promise((resolve) => setTimeout(resolve, 10));
      assert.deepEqual(unhandled, []);
    } finally {
      process.off("unhandledRejection", onUnhandled);
    }
  });
}

test("rethrows original target-closed click error when waitForEvent(close) rejects", async () => {
  const clickError = new Error(
    "locator.click: Target page, context or browser has been closed",
  );
  const closeError = new Error("Context closed before close event");
  const { popup } = stubPopup({
    async onClick() {
      throw clickError;
    },
    async onWaitForClose() {
      throw closeError;
    },
  });

  await assert.rejects(
    () => clickClosingPopup(popup, "turn-on"),
    (error) => {
      assert.equal(error, clickError);
      return true;
    },
  );
});

test("rethrows close error when real click resolves but popup stays open", async () => {
  const closeError = Object.assign(
    new Error(
      'page.waitForEvent: Timeout 30000ms exceeded while waiting for event "close"',
    ),
    { name: "TimeoutError" },
  );
  const { popup } = stubPopup({
    async onClick() {},
    async onWaitForClose() {
      throw closeError;
    },
  });

  await assert.rejects(
    () => clickClosingPopup(popup, "turn-on"),
    (error) => {
      assert.equal(error, closeError);
      return true;
    },
  );
});
