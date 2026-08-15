// The seam between an engine effect and the host document that performs it.

import Foundation

/// Performs one engine effect against whatever the current client is.
///
/// Passed per operation rather than stored on the manager: there is one engine
/// but one controller per client text session, so a stored delegate would be
/// whatever session activated last, and effects for the session the user is
/// actually typing in would be written into a different app's document.
@MainActor
protocol ComposingEffectExecutor: AnyObject {
    func execute(_ effect: ComposingTransition.Effect)
}
