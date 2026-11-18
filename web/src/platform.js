// The visitor's operating system, as the session names it: how keys are read
// and spelled (`Session` constructor). The page, the dev page, the browser
// extension's tabs and the options page share it.

/** @typedef {import("../types/shared.d.ts").Platform} Platform */

/** @returns {Platform} */
export function hostPlatform() {
  const name = navigator.userAgentData?.platform ?? navigator.platform;
  if (/mac|iphone|ipad/i.test(name)) {
    return "macos";
  }
  return /win/i.test(name) ? "windows" : "linux";
}
