/**
 * `window.quill`, for a test to read the document and its history from.
 *
 * @type {QuillInstance}
 */
window.quill = new Quill("#editor", {
  theme: "snow",
  modules: {
    toolbar: [[{ header: [1, 2, false] }], ["bold", "italic", "underline"]],
  },
});
