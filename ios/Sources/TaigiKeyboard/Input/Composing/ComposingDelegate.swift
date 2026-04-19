import Foundation

/// Platform adapter that executes a `ComposingTransition.Effect` against
/// the host text-input surface.
///
/// Keeps `ComposingManager` independent of `_Keyboard/` — the iOS
/// implementation (in `KeyboardViewController+TextInput`) switches on the
/// effect enum and forwards to `UITextDocumentProxy`. Android's Phase II
/// mirror implements the same contract against `InputConnection`.
///
/// The binding contract (iOS / Android mappings) lives in
/// `docs/architecture/composing-state-boundary.md` §2.2.
protocol ComposingDelegate: AnyObject {
    func execute(_ effect: ComposingTransition.Effect)
}
