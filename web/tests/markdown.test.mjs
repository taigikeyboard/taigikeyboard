// The format bubble's Markdown toggles (`markdown.js`) and Save's HTML over them
// (`save.js`), without a browser.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  linkAt,
  normalizeUrl,
  toggleBold,
  toggleBulletList,
  toggleHeading,
  toggleItalic,
  toggleLink,
  toggleNumberedList,
  toggleQuote,
  toggleUnderline,
} from "../src/markdown.js";
import { toHtml } from "../src/save.js";

/// `value` with `change` applied, and the selection it leaves.
function applied(value, change) {
  const text =
    value.slice(0, change.start) + change.text + value.slice(change.end);
  return [text, text.slice(change.selectStart, change.selectEnd)];
}

/// `toggleHeading` at `level`, in the toggles' shape.
const heading =
  (level) =>
  (...range) =>
    toggleHeading(level, ...range);

/// Selects `part` (its first occurrence) in `value` and runs `toggle`.
function run(toggle, value, part, ...rest) {
  const start = value.indexOf(part);
  return applied(value, toggle(value, start, start + part.length, ...rest));
}

test("bold wraps the selection and selects its text", () => {
  assert.deepEqual(run(toggleBold, "我欲去台北", "台北"), [
    "我欲去**台北**",
    "台北",
  ]);
});

test("bold comes off whether the markers are selected or just outside", () => {
  assert.deepEqual(run(toggleBold, "去**台北**", "**台北**"), [
    "去台北",
    "台北",
  ]);
  assert.deepEqual(run(toggleBold, "去**台北**", "台北"), ["去台北", "台北"]);
});

test("spaces and punctuation at the ends stay outside the markers", () => {
  // trace: 「 and 」 are \p{P}; markers inside them stay flanking.
  assert.deepEqual(run(toggleBold, "我「台灣」是", "「台灣」"), [
    "我「**台灣**」是",
    "台灣",
  ]);
  assert.deepEqual(run(toggleItalic, "a hó-sè. b", " hó-sè. "), [
    "a *hó-sè*. b",
    "hó-sè",
  ]);
  // A selection over the punctuation and the markers takes them off.
  assert.deepEqual(run(toggleBold, "我「**台灣**」是", "「**台灣**」"), [
    "我「台灣」是",
    "台灣",
  ]);
});

test("italic tells one star from two", () => {
  assert.deepEqual(run(toggleItalic, "**台北**", "台北"), [
    "***台北***",
    "台北",
  ]);
  assert.deepEqual(run(toggleItalic, "***台北***", "台北"), [
    "**台北**",
    "台北",
  ]);
  assert.deepEqual(run(toggleBold, "***台北***", "台北"), ["*台北*", "台北"]);
  assert.deepEqual(run(toggleItalic, "*台北*", "台北"), ["台北", "台北"]);
});

test("nothing to wrap in a selection of punctuation", () => {
  assert.equal(toggleBold("a，b", 1, 2), null);
});

test("quote marks every line of the paragraph the selection touches", () => {
  const value = "頭前\n\n第一逝\n第二逝\n\n後壁";
  const [text, selected] = run(toggleQuote, value, "一");
  assert.equal(text, "頭前\n\n> 第一逝\n> 第二逝\n\n後壁");
  assert.equal(selected, "> 第一逝\n> 第二逝");
  assert.deepEqual(run(toggleQuote, text, "二"), [
    "頭前\n\n第一逝\n第二逝\n\n後壁",
    "第一逝\n第二逝",
  ]);
});

test("quote over two paragraphs leaves the blank line between them", () => {
  const value = "a\n\nb";
  assert.deepEqual(applied(value, toggleQuote(value, 0, value.length)), [
    "> a\n\n> b",
    "> a\n\n> b",
  ]);
});

test("a link goes around the selection and comes off with no URL", () => {
  const [text, selected] = run(
    toggleLink,
    "看教典",
    "教典",
    "https://sutian.moe.edu.tw",
  );
  assert.equal(text, "看[教典](https://sutian.moe.edu.tw)");
  assert.equal(selected, "教典");
  assert.deepEqual(linkAt(text, 2, 3), {
    start: 1,
    end: text.length,
    text: "教典",
    url: "https://sutian.moe.edu.tw",
  });
  assert.deepEqual(run(toggleLink, text, "典", null), ["看教典", "教典"]);
  assert.deepEqual(run(toggleLink, text, "教", "https://a.tw"), [
    "看[教典](https://a.tw)",
    "教典",
  ]);
  assert.equal(toggleLink("看教典", 1, 3, null), null);
});

test("a URL gets a scheme and loses what would end the link", () => {
  assert.equal(normalizeUrl("  "), null);
  assert.equal(normalizeUrl("taigikeyboard.tw"), "https://taigikeyboard.tw");
  assert.equal(
    normalizeUrl("mailto:someone@example.com"),
    "mailto:someone@example.com",
  );
  assert.equal(
    normalizeUrl("https://x.org/wiki/A_(b) c"),
    "https://x.org/wiki/A_%28b%29%20c",
  );
});

test("text with no markers saves as HTML exactly as before", () => {
  const text = "台灣 & <b>\n第二逝\n\n\n後壁";
  // trace: today's toHtml: escape, one <p> per blank-line block, <br> per break.
  const body = "<p>台灣 &amp; &lt;b&gt;<br>\n第二逝</p>\n<p>後壁</p>";
  assert.ok(toHtml("", text).includes(`<body>\n${body}\n</body>`));
});

test("HTML reads bold, italic, quotes and safe links", () => {
  const body = (text) =>
    toHtml("", text).split("<body>\n")[1].split("\n</body>")[0];
  assert.equal(
    body("**a** *b* ***c***"),
    "<p><strong>a</strong> <em>b</em> <strong><em>c</em></strong></p>",
  );
  assert.equal(
    body("> 1 < 2\n> **a**"),
    "<blockquote><p>1 &lt; 2<br>\n<strong>a</strong></p></blockquote>",
  );
  assert.equal(
    body("[*教典*](https://a.tw/*b*)"),
    '<p><a href="https://a.tw/*b*"><em>教典</em></a></p>',
  );
  assert.equal(
    body('[x](https://a"onmouseover=y)'),
    '<p><a href="https://a&quot;onmouseover=y">x</a></p>',
  );
  assert.equal(
    body("[x](javascript:alert(1))"),
    "<p>[x](javascript:alert(1))</p>",
  );
  assert.equal(
    body("[x](HTTPS://A.TW)"),
    '<p><a href="HTTPS://A.TW">x</a></p>',
  );
  // A lone star and a star against a space are text.
  assert.equal(body("5 * 3 = 15, a* b*"), "<p>5 * 3 = 15, a* b*</p>");
});

test("a word between marked words is wrapped on its own", () => {
  assert.deepEqual(run(toggleItalic, "*a* b *c*", " b "), ["*a* *b* *c*", "b"]);
  assert.deepEqual(run(toggleBold, "**a** b **c**", " b "), [
    "**a** **b** **c**",
    "b",
  ]);
  // Two spans selected together: no toggle undoes them cleanly.
  const value = "**a** and **b**";
  assert.equal(toggleBold(value, 0, value.length), null);
  // Stars that close one span and open the next: no toggle reads them.
  assert.equal(toggleItalic("x*a*b*c*y", 5, 6), null);
  // A star between spaces is text, not an opener.
  assert.deepEqual(run(toggleItalic, "5 * 3 *a*", "a"), ["5 * 3 a", "a"]);
});

test("a line of spaces does not end a paragraph, as in Save's HTML", () => {
  assert.deepEqual(run(toggleQuote, "a\n  \nb", "a"), [
    "> a\n>   \n> b",
    "> a\n>   \n> b",
  ]);
  assert.ok(toHtml("", "> a\n>   \n> b").includes("<blockquote>"));
});

test("a host with a port gets https, and wide spaces are UTF-8 encoded", () => {
  assert.equal(normalizeUrl("localhost:3000"), "https://localhost:3000");
  assert.equal(
    normalizeUrl("example.com:8080/x"),
    "https://example.com:8080/x",
  );
  assert.equal(normalizeUrl("tel:0912"), "tel:0912");
  assert.equal(normalizeUrl("a　b"), "https://a%E3%80%80b");
});

test("a long run of spaces in the selection trims at once", () => {
  const value = `a${" ".repeat(40_000)}b`;
  const started = performance.now();
  const change = toggleBold(value, 0, value.length);
  assert.ok(performance.now() - started < 200);
  assert.equal(change.text, `**${value}**`);
});

test("headings take the lines the selection touches, one level at a time", () => {
  assert.deepEqual(run(heading(2), "標題\n內文", "標"), [
    "## 標題\n內文",
    "## 標題",
  ]);
  assert.deepEqual(run(heading(2), "## 標題", "標"), ["標題", "標題"]);
  assert.deepEqual(run(heading(1), "## 標題", "標"), ["# 標題", "# 標題"]);
  assert.deepEqual(run(heading(3), "- 項目", "項"), ["### 項目", "### 項目"]);
});

test("lists take whole paragraphs and count each from 1", () => {
  assert.deepEqual(run(toggleBulletList, "a\nb\n\nc", "a"), [
    "- a\n- b\n\nc",
    "- a\n- b",
  ]);
  assert.deepEqual(run(toggleBulletList, "- a\n- b", "a"), ["a\nb", "a\nb"]);
  const value = "a\n\nb\nc";
  assert.deepEqual(applied(value, toggleNumberedList(value, 0, value.length)), [
    "1. a\n\n1. b\n2. c",
    "1. a\n\n1. b\n2. c",
  ]);
  assert.deepEqual(run(toggleBulletList, "1. a\n2. b", "a"), [
    "- a\n- b",
    "- a\n- b",
  ]);
  assert.deepEqual(run(toggleNumberedList, "1. a\n2. b", "b"), [
    "a\nb",
    "a\nb",
  ]);
});

test("underline is a u tag, on and off", () => {
  assert.deepEqual(run(toggleUnderline, "我欲去台北", "台北"), [
    "我欲去<u>台北</u>",
    "台北",
  ]);
  assert.deepEqual(run(toggleUnderline, "去<u>台北</u>", "<u>台北</u>"), [
    "去台北",
    "台北",
  ]);
  assert.deepEqual(run(toggleUnderline, "去<u>台北</u>", "台北"), [
    "去台北",
    "台北",
  ]);
  assert.equal(toggleUnderline("<u>a</u> b", 0, 10), null);
});

test("HTML reads headings, lists and underline", () => {
  const body = (text) =>
    toHtml("", text).split("<body>\n")[1].split("\n</body>")[0];
  assert.equal(
    body("# 一\n## 二\n### 三"),
    "<h1>一</h1>\n<h2>二</h2>\n<h3>三</h3>",
  );
  assert.equal(
    body("- a\n- **b**"),
    "<ul>\n<li>a</li>\n<li><strong>b</strong></li>\n</ul>",
  );
  assert.equal(
    body("3. a\n4. b"),
    '<ol start="3">\n<li>a</li>\n<li>b</li>\n</ol>',
  );
  assert.equal(
    body("頭前\n- a\n- b"),
    "<p>頭前</p>\n<ul>\n<li>a</li>\n<li>b</li>\n</ul>",
  );
  assert.equal(body("<u>台北</u> & <b>"), "<p><u>台北</u> &amp; &lt;b&gt;</p>");
  assert.equal(body("#tag\n-x\n1.5"), "<p>#tag<br>\n-x<br>\n1.5</p>");
});

test("a list reaches forward to the paragraph's end and back only over items", () => {
  assert.deepEqual(run(toggleBulletList, "Title:\nx\ny", "x"), [
    "Title:\n- x\n- y",
    "- x\n- y",
  ]);
  assert.deepEqual(run(toggleBulletList, "- a\n- b", "b"), ["a\nb", "a\nb"]);
  // A line of spaces is no item.
  assert.deepEqual(run(toggleNumberedList, "a\n  \nb", "a"), [
    "1. a\n  \n2. b",
    "1. a\n  \n2. b",
  ]);
});

test("a quote holds the headings and lists inside it", () => {
  const body = (text) =>
    toHtml("", text).split("<body>\n")[1].split("\n</body>")[0];
  assert.equal(
    body("> - a\n> - b"),
    "<blockquote><ul>\n<li>a</li>\n<li>b</li>\n</ul></blockquote>",
  );
  assert.equal(
    body("> # h\n> text"),
    "<blockquote><h1>h</h1>\n<p>text</p></blockquote>",
  );
});
