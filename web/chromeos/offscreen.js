// Keeps the input method's worker alive: a message resets its 30 s idle timer,
// and a key alone never wakes a stopped worker (chromeos-roadmap.md § Spike result).
setInterval(() => {
  chrome.runtime
    .sendMessage({ keepAlive: true })
    .catch((/** @type {unknown} */ error) =>
      console.warn(
        "keepAlive.failed",
        error instanceof Error ? error.message : String(error),
      ),
    );
}, 20_000);
