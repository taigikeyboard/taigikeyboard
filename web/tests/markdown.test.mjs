// The stored text's Markdown and the editor's blocks (`markdown.js`), and Save's
// HTML and plain text over them (`save.js`), without a browser.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  encodeUrl,
  fromPlainText,
  normalizeUrl,
  parse,
  plainText,
  serialize,
} from "../src/markdown.js";
import { toHtml } from "../src/save.js";

const run = (text, format = {}) => ({
  text,
  bold: false,
  italic: false,
  underline: false,
  url: null,
  ...format,
});

const paragraph = (...runs) => ({ type: "paragraph", runs });

/// `markdown` reads as blocks that write back as the same text.
function roundTrip(markdown) {
  assert.equal(serialize(parse(markdown)), markdown);
}

test("text with no markers is one paragraph per blank-line block", () => {
  assert.deepEqual(parse("台灣\n第二逝\n\n\n後壁"), [
    paragraph(run("台灣\n第二逝")),
    paragraph(run("後壁")),
  ]);
  assert.deepEqual(parse("  \n\n"), []);
});

test("every block and span reads back as written", () => {
  for (const markdown of [
    "頭前\n\n我欲去**台北**揣朋友",
    "- a\n- **b**",
    "3. a\n4. b",
    "> q1\n> q2",
    "a\nb\n\nc",
    "看[*教典*](https://a.tw/x_(y))",
    "<u>台北</u> & 1 < 2",
    "***x*** **<u>y</u>**",
    "5 * 3",
    "x [注] y",
    "# 一\n\n## 二\n\n### 三",
  ]) {
    roundTrip(markdown);
  }
});

test("blocks read with their kinds, levels and list numbers", () => {
  assert.deepEqual(parse("## 標題\n- a\n- b\n\n7. c"), [
    { type: "heading", level: 2, runs: [run("標題")] },
    { type: "list", ordered: false, start: 1, items: [[run("a")], [run("b")]] },
    { type: "list", ordered: true, start: 7, items: [[run("c")]] },
  ]);
  assert.deepEqual(parse("> **a**\n> b"), [
    {
      type: "quote",
      runs: [run("a", { bold: true }), run("\nb")],
    },
  ]);
});

test("spans nest: a link's text keeps its formats", () => {
  assert.deepEqual(parse("[**教**典](https://a.tw)")[0].runs, [
    run("教", { bold: true, url: "https://a.tw" }),
    run("典", { url: "https://a.tw" }),
  ]);
});

test("plain text that looks like Markdown writes back with a backslash", () => {
  const blocks = [
    paragraph(run("a*b*c # x")),
    paragraph(run("# hi\n- x\n1. y\n> z")),
    paragraph(run("<u>不是底線</u>")),
  ];
  const markdown = serialize(blocks);
  assert.equal(
    markdown,
    "a\\*b\\*c # x\n\n\\# hi\n\\- x\n1\\. y\n\\> z\n\n\\<u>不是底線\\</u>",
  );
  assert.deepEqual(parse(markdown), blocks);
});

test("formatted runs keep their spaces outside the markers", () => {
  const markdown = serialize([
    paragraph(run("我"), run(" 台北 ", { bold: true }), run("好")),
  ]);
  assert.equal(markdown, "我 **台北** 好");
});

test("a heading or an item is one line", () => {
  assert.equal(
    serialize([{ type: "heading", level: 1, runs: [run("a\nb")] }]),
    "# a b",
  );
});

test("plain text drops markers and keeps list bullets", () => {
  assert.equal(
    plainText(parse("# 一\n\n**粗**\n\n- a\n- b\n\n2. c")),
    "一\n\n粗\n\n- a\n- b\n\n2. c",
  );
});

test("a URL gets a scheme and loses what would end the link", () => {
  assert.equal(normalizeUrl("  "), null);
  assert.equal(normalizeUrl("taigikeyboard.tw"), "https://taigikeyboard.tw");
  assert.equal(normalizeUrl("localhost:3000"), "https://localhost:3000");
  assert.equal(
    normalizeUrl("mailto:someone@example.com"),
    "mailto:someone@example.com",
  );
  assert.equal(normalizeUrl("tel:0912"), "tel:0912");
  assert.equal(
    normalizeUrl("https://x.org/wiki/A_(b) c"),
    "https://x.org/wiki/A_%28b%29%20c",
  );
  assert.equal(normalizeUrl("a　b"), "https://a%E3%80%80b");
});

const body = (text) =>
  toHtml("", text).split("<body>\n")[1].split("\n</body>")[0];

test("text with no markers saves as HTML as before", () => {
  // trace: the textarea's toHtml: escape, one <p> per blank-line block, <br>
  // per break.
  assert.equal(
    body("台灣 & <b>\n第二逝\n\n\n後壁"),
    "<p>台灣 &amp; &lt;b&gt;<br>\n第二逝</p>\n<p>後壁</p>",
  );
});

test("HTML renders headings, lists, quotes, spans and safe links", () => {
  assert.equal(
    body("# 一\n## 二\n### 三"),
    "<h1>一</h1>\n<h2>二</h2>\n<h3>三</h3>",
  );
  assert.equal(
    body("頭前\n- a\n- **b**"),
    "<p>頭前</p>\n<ul>\n<li>a</li>\n<li><strong>b</strong></li>\n</ul>",
  );
  assert.equal(
    body("3. a\n4. b"),
    '<ol start="3">\n<li>a</li>\n<li>b</li>\n</ol>',
  );
  assert.equal(
    body("> 1 < 2\n> **a**"),
    "<blockquote><p>1 &lt; 2<br>\n<strong>a</strong></p></blockquote>",
  );
  assert.equal(
    body("**a** *b* ***c*** <u>d</u>"),
    "<p><strong>a</strong> <em>b</em> <strong><em>c</em></strong> <u>d</u></p>",
  );
  assert.equal(
    body("[*教典*](https://a.tw/*b*)"),
    '<p><a href="https://a.tw/*b*"><em>教典</em></a></p>',
  );
  assert.equal(
    body('[x](https://a"onmouseover=y)'),
    '<p><a href="https://a&quot;onmouseover=y">x</a></p>',
  );
  // Any other scheme leaves the link's text.
  assert.equal(body("[x](javascript:alert(1))"), "<p>x</p>");
  assert.equal(body("5 * 3 = 15, a* b*"), "<p>5 * 3 = 15, a* b*</p>");
});

test("what a write gives reads back as the same blocks (fuzz finds)", () => {
  for (const markdown of [
    "\\\\>#))https://a",
    "<*\n\\<u>**. 1**](a「-](</u>#」&-1",
    "&--[\n\n>*「\\**><**台)</u><u>**",
    "a\\*b\n- x\n  \nc",
  ]) {
    const blocks = parse(markdown);
    assert.deepEqual(
      parse(serialize(blocks)),
      parse(serialize(parse(serialize(blocks)))),
    );
    assert.equal(
      plainText(parse(serialize(blocks))).replace(/\s+/g, ""),
      plainText(blocks).replace(/\s+/g, ""),
    );
  }
});

test("a textarea text reads as plain text: nothing becomes a marker", () => {
  const text = "1. 一\n1. 二\n*三* # 四\n\n- 五\n> 六";
  assert.equal(plainText(parse(fromPlainText(text))), text);
  assert.ok(
    parse(fromPlainText(text)).every((block) => block.type === "paragraph"),
  );
});

test("a number with a backslash of its own reads back as written", () => {
  const blocks = [paragraph(run("1\\. x"))];
  assert.deepEqual(parse(serialize(blocks)), blocks);
});

test("a pasted URL is encoded so its link holds", () => {
  assert.equal(encodeUrl("https://a.b/x y)"), "https://a.b/x%20y%29");
  roundTrip("[see](https://a.b/x%20y%29)");
});
