// Keeps the input method's worker alive: a message resets its 30 s idle timer,
// and a key alone never wakes a stopped worker (chromeos-roadmap.md § Spike result).
setInterval(() => {
  chrome.runtime
    .sendMessage({ keepAlive: true })
    .catch((error) => console.warn("keepAlive.failed", error.message));
}, 20_000);
