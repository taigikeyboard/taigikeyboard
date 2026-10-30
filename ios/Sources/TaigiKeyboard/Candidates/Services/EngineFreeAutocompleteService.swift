import Foundation
import KeyboardKit

// KeyboardKit 11 engine-side members for services that own no KeyboardKit engine.

/// An `AutocompleteService` that produces suggestions itself and has no ignore / learn UX.
/// KeyboardKit routes ignore / learn through `autocompleteEngine(for:)`, so it gets a collection
/// with no engines.
protocol EngineFreeAutocompleteService: KeyboardKit.AutocompleteService {}

@MainActor
private let noEngines = AutocompleteEngineCollection([])

/// Every locale: these services do not branch on the keyboard locale.
private let allLocaleIdentifiers = Locale.availableIdentifiers

@MainActor
extension EngineFreeAutocompleteService {
    var supportedLocales: [String] {
        allLocaleIdentifiers
    }

    func autocompleteEngine(for _: Locale) -> any AutocompleteEngine {
        noEngines
    }

    func registerAdditionalEngines(_: [any AutocompleteEngine]) {}
    func warmUp(for _: Locale) async {}
}
