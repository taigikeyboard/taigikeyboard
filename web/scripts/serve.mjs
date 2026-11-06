// Serves a built site on localhost for `make serve` and Playwright, with the
// media type `WebAssembly.compileStreaming` insists on. Not `python3 -m
// http.server`: its listen backlog of 5 resets connections under parallel tests.
//   node scripts/serve.mjs <directory> <port>

import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, resolve, sep } from "node:path";

const MEDIA_TYPES = {
  ".css": "text/css",
  ".gz": "application/gzip",
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".webmanifest": "application/manifest+json",
  ".woff2": "font/woff2",
};

const [directory, port] = process.argv.slice(2);
const root = resolve(directory);

/// `null` for a URL that does not parse.
function fileOf(url) {
  try {
    const { pathname } = new URL(url, "http://localhost");
    const path = join(root, decodeURIComponent(pathname));
    return pathname.endsWith("/") ? join(path, "index.html") : path;
  } catch {
    return null;
  }
}

createServer(async (request, response) => {
  const file = fileOf(request.url);
  const isFile =
    file?.startsWith(root + sep) &&
    (await stat(file).then(
      (info) => info.isFile(),
      () => false,
    ));
  if (!isFile) {
    response.writeHead(404).end();
    return;
  }
  response.writeHead(200, {
    "Content-Type": MEDIA_TYPES[extname(file)] ?? "application/octet-stream",
  });
  createReadStream(file).pipe(response);
}).listen(Number(port), "127.0.0.1", () => {
  console.log(`http://127.0.0.1:${port}/`);
});
