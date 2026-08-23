// Reading and writing the hand-editable CSV exports of the user's own data.

import Foundation

/// The CSV dialect the three platforms share.
///
/// Only the quoting primitives live here now: the frequency and association
/// codecs went with the panes that exported them, and the surviving caller is
/// `CustomDictionaryCSV`.
///
/// CROSS-PLATFORM INVARIANT — mirrors
/// ios/Sources/TaigiKeyboard/App/Tabs/Dictionary/Utilities/CSVDocument.swift
/// (`parseLine` / `escape`) and
/// android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/DictionaryCsvCodec.kt.
/// Drift means a file exported on one platform imports differently on another,
/// which is the whole point of the format.
///
/// This is a SINGLE-RECORD dialect, not full RFC 4180: quoting inside a line is
/// honoured (including the doubled-quote escape), but a record is always one
/// line, because both mirrors split on newlines before parsing. `escape` can
/// therefore emit a quoted field containing a newline that no decoder here will
/// read back — the same asymmetry the other two platforms have, kept rather
/// than fixed so the three stay byte-compatible.
enum UserDataCSV {
    /// Splits one CSV line into its fields.
    ///
    /// A doubled quote inside a quoted field (`""`) is one literal quote, which
    /// is what makes a round-trip through `escape` lossless. An unbalanced
    /// quote leaves the rest of the line quoted rather than erroring — a
    /// hand-edited file gets a best effort, not a rejection.
    static func parseLine(_ line: String) -> [String] {
        var fields: [String] = []
        var current = ""
        var isInsideQuotes = false
        let characters = Array(line)
        var index = 0
        while index < characters.count {
            let character = characters[index]
            if character == "\"", isInsideQuotes, index + 1 < characters.count,
               characters[index + 1] == "\""
            {
                current.append("\"")
                index += 1
            } else if character == "\"" {
                isInsideQuotes.toggle()
            } else if character == ",", !isInsideQuotes {
                fields.append(current)
                current = ""
            } else {
                current.append(character)
            }
            index += 1
        }
        fields.append(current)
        return fields
    }

    /// Quotes a field only when it would otherwise change the parse. The
    /// trigger set is exactly `,` `"` and newline — a leading space or a `\r`
    /// is left alone, matching both mirrors.
    static func escape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }
}
