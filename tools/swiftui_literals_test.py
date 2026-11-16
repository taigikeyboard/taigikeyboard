"""Unit tests for the SwiftUI string-literal gate."""

from __future__ import annotations

import unittest

import swiftui_literals


class FindingsInTests(unittest.TestCase):
    def test_flags_a_literal_title_on_views_and_modifiers(self) -> None:
        source = (
            'Text("Hello")\n'
            'Button("Save") { save() }\n'
            'list.navigationTitle("Settings")\n'
        )

        self.assertEqual(
            swiftui_literals.findings_in(source),
            [(1, 'Text("'), (2, 'Button("'), (3, '.navigationTitle("')],
        )

    def test_flags_a_literal_on_the_line_after_the_parenthesis(self) -> None:
        self.assertEqual(
            swiftui_literals.findings_in('view\n    .confirmationDialog(\n        "Delete?",\n'),
            [(2, '.confirmationDialog( "')],
        )

    def test_flags_interpolation_only_and_multiline_literals(self) -> None:
        self.assertEqual(
            swiftui_literals.findings_in(
                'Text("\\(count)")\nText("""\n  long\n  """)\nText(#"raw"#)\n'
            ),
            [(1, 'Text("'), (2, 'Text("'), (5, 'Text(#"')],
        )

    def test_flags_explicit_localizing_apis(self) -> None:
        source = (
            "let key: LocalizedStringKey = title\n"
            'let a = String(localized: "x")\n'
            'let b = NSLocalizedString("x", comment: "")\n'
        )

        self.assertEqual(
            [line for line, _ in swiftui_literals.findings_in(source)], [1, 2, 3]
        )

    def test_allows_empty_titles_verbatim_text_and_generated_strings(self) -> None:
        source = (
            'Toggle("", isOn: $isOn)\n'
            'Picker(#""#, selection: $kind)\n'
            'Text(verbatim: "•")\n'
            "Text(lang.string(.dictionaryNoResults))\n"
            "Text(title)\n"
            'Image("keyboard-preview")\n'
            'proxy.setMarkedText("", selectedRange: range)\n'
            'contextText("abc")\n'
        )

        self.assertEqual(swiftui_literals.findings_in(source), [])


if __name__ == "__main__":
    unittest.main()
