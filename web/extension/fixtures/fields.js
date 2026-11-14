// What the page itself hears: a key the extension took must not show up here.
const log = /** @type {HTMLElement} */ (document.getElementById("log"));
for (const type of /** @type {const} */ (["keydown", "keyup"])) {
  document.addEventListener(type, (event) => {
    log.textContent += `${type}:${event.key}\n`;
  });
}
