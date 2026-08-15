// Per-session IMKInputController. PR2 spike: echoes typed characters.

import InputMethodKit

/// One instance per client text session. This is the InputMethodKit shell the
/// composing engine plugs into later; for now it only proves the wiring by
/// echoing plain typed characters back to the client itself.
///
/// `@objc(TaigiInputController)` pins the Objective-C runtime name that
/// `InputMethodServerControllerClass` looks up in the bundle's Info.plist.
@objc(TaigiInputController)
public final class TaigiInputController: IMKInputController {
    /// `static`: IMK builds one controller per client text session, and the
    /// category never varies, so a stored property would create an `os_log`
    /// handle per session.
    private static let logger = DebugLogger(category: "InputController")

    /// The recognized-event mask is this controller's standing contract with
    /// IMK, so it is declared in full here rather than widened later:
    /// `.flagsChanged` arrives for the modifier chords the composing slices
    /// need, and `.keyUp` is excluded because nothing will act on it.
    override public func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask([.keyDown, .flagsChanged]).rawValue)
    }

    /// CHROMIUM DEADLOCK RULE — never query the client synchronously from here
    /// (`attributesForCharacterIndex:lineHeightRectangle:`, `selectedRange`,
    /// `markedRange`, `length`, `attributedSubstringFromRange:`, …). Chromium
    /// hosts deadlock on a synchronous round-trip during activation
    /// (Chromium issue 503787240, hit by azooKey-Desktop). Pinned by
    /// `ActivateServerClientQueryTests`.
    override public func activateServer(_ sender: Any!) {
        Self.logger.debug("activateServer")
    }

    /// Deliberately does not call `super`. `IMKInputController`'s implementation
    /// re-enters the controller to flush its own notion of the composition;
    /// this controller owns its buffer outright, so forcing a commit is its
    /// decision alone. There is no buffer yet, so today this is a no-op.
    override public func commitComposition(_ sender: Any!) {
        Self.logger.debug("commitComposition")
    }

    override public func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        guard let client = sender as? IMKTextInput else { return false }
        guard let text = Self.echoableText(of: event) else { return false }

        Self.logger.debug("echo \(text)")
        client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
        return true
    }

    /// The characters this spike echoes, or `nil` when the event must fall
    /// through to the host so its shortcuts, navigation, and editing keys keep
    /// working.
    ///
    /// Three things disqualify an event, and none of the three subsumes the
    /// others. Command / control / option chords belong to the host.
    /// `NSEvent.specialKey` covers keys that carry neither a control character
    /// nor a private-use scalar — `.lineSeparator` is `U+2028` and
    /// `.paragraphSeparator` is `U+2029`, both ordinary separator scalars.
    /// And the scalar rules catch what `specialKey` does not enumerate: Escape
    /// (`U+001B`) has no `SpecialKey` case, and AppKit maps function keys into
    /// the private-use range `U+F700...U+F8FF` beyond the named ones.
    static func echoableText(of event: NSEvent) -> String? {
        let disqualifyingModifiers: NSEvent.ModifierFlags = [.command, .control, .option]
        guard event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            .isDisjoint(with: disqualifyingModifiers)
        else { return nil }

        guard event.specialKey == nil else { return nil }

        guard let characters = event.characters, !characters.isEmpty else { return nil }
        guard characters.unicodeScalars.allSatisfy(isTextScalar) else { return nil }

        return characters
    }

    /// Hoisted out of `isTextScalar`, which runs once per scalar per keystroke;
    /// `CharacterSet.controlCharacters` materializes a bridged set each access.
    private static let controlCharacters = CharacterSet.controlCharacters
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    private static func isTextScalar(_ scalar: Unicode.Scalar) -> Bool {
        !controlCharacters.contains(scalar) && !appKitFunctionKeyRange.contains(scalar.value)
    }
}
