// Where a search result links out to.

import Foundation

/// Builds the 教典 and ChhoeTaigi query URLs for a reading.
///
/// Platform-owned by design, not engine work: these are two websites' query
/// conventions, and a convention is not phonetics. Mirrors
/// ios/Sources/TaigiKeyboard/Lexicon/Utils/ExternalLookupURLBuilder.swift and
/// android/.../ime/dictionary/ExternalLookupURLBuilder.kt — the same reading
/// has to reach the same page from all three.
enum ExternalLookupURLBuilder {
    /// 教育部臺灣台語常用詞辭典.
    static func moeURL(forTl tl: String) -> URL? {
        url(
            host: "sutian.moe.edu.tw",
            path: "/zh-hant/tshiau/",
            fixedQuery: [URLQueryItem(name: "lui", value: "tai_su")],
            readingParameter: "tsha",
            tl: tl,
        )
    }

    /// ChhoeTaigi 台語辭典.
    static func chhoeURL(forTl tl: String) -> URL? {
        url(
            host: "chhoe.taigi.info",
            path: "/s",
            fixedQuery: [
                URLQueryItem(name: "s", value: "su"),
                URLQueryItem(name: "f", value: "e"),
                URLQueryItem(name: "lmjf", value: "ki"),
            ],
            readingParameter: "lmj",
            tl: tl,
        )
    }

    /// Assembles the URL with the reading as ONE query value.
    ///
    /// `URLQueryItem` rather than percent-encoding into an interpolated
    /// string: `.urlQueryAllowed` leaves `&` and `=` alone, and a
    /// custom-dictionary romanization is whatever the user typed — a reading
    /// must not be able to add query parameters of its own.
    private static func url(
        host: String,
        path: String,
        fixedQuery: [URLQueryItem],
        readingParameter: String,
        tl: String,
    ) -> URL? {
        let digitTone = digitToneForm(tl)
        guard !digitTone.isEmpty else { return nil }

        var components = URLComponents()
        components.scheme = "https"
        components.host = host
        components.path = path
        components.queryItems = fixedQuery
            + [URLQueryItem(name: readingParameter, value: digitTone)]
        return components.url
    }

    /// The reading in the digit-tone form both sites index by, hyphens intact.
    static func digitToneForm(_ tl: String) -> String {
        tl.lowercased()
            .split(separator: "-", omittingEmptySubsequences: false)
            .map { syllableInDigitTone(String($0)) }
            .joined(separator: "-")
    }

    /// One syllable, with its tone as a digit — except tones 1 and 4, which
    /// both sites omit because the open and checked syllables carry no mark.
    private static func syllableInDigitTone(_ syllable: String) -> String {
        guard !syllable.isEmpty else { return "" }

        // The nasal marker is substituted first and on the RAW input, because
        // the already-digit-toned path below reads the last character and a
        // superscript ⁿ sitting after the digit would hide it.
        let withNasalConverted = syllable
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
        if let last = withNasalConverted.last, last.isNumber {
            // Still preprocessed: a digit-toned reading can carry `o͘`, which
            // is a letter with a mark rather than a tone, and the sites index
            // it spelled `oo`. Only the TONE is already resolved here.
            let normalized = RustEngineBridge.nfdPreprocessForLookup(withNasalConverted)
                ?? withNasalConverted
            let tone = String(last)
            return tone == "1" || tone == "4"
                ? String(normalized.dropLast())
                : normalized
        }

        // Diacritic form: the engine owns both the `o͘` / nasal normalisation
        // and the tone extraction, so a reading that reaches here spelled any
        // legitimate way produces the one key the sites index.
        guard let preprocessed = RustEngineBridge.nfdPreprocessForLookup(withNasalConverted),
              let stripped = RustEngineBridge.stripTone(preprocessed)
        else { return withNasalConverted }

        if stripped.tone.isEmpty || stripped.tone == "1" || stripped.tone == "4" {
            return stripped.bare
        }
        return stripped.bare + stripped.tone
    }
}
