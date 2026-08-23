// Proves the committed macOS Rust artefacts actually link and round-trip.

import SwiftProtobuf
@testable import TaigiInputMethodCore
import XCTest

/// The first real gate on `macos/RustEngine/RustTaigi.xcframework`: PR1 only
/// validated the artefact's *shape* (one macos-arm64_x86_64 slice). These cases push
/// bytes through the static archive and back, exercising SwiftProtobuf encode
/// / decode, the C module import, the swift-bridge wrapper, and the Rust
/// dispatcher in one hop.
final class EngineFfiSmokeTests: XCTestCase {
    /// `TlToPoj` is a pure table lookup — no `lexiconInstall`, no dictionary
    /// artefacts, no engine state. Oracle verified against the Rust function
    /// directly: `tl_display_to_poj_display("guá") == "góa"`, returned in NFC.
    func testProcessRequest_tlToPoj_returnsPojDisplayForm() throws {
        var tlToPoj = Taigi_Engine_TlToPoj()
        tlToPoj.input = "gu\u{00E1}"

        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.tlToPoj = tlToPoj

        var config = Taigi_Engine_AppConfig()
        config.inputMode = "tl"

        var request = Taigi_Engine_Request()
        request.id = 42
        request.type = .cmdPhonetics
        request.configSnapshot = config
        request.phonetics = phonetics

        let responseBytes = try RustEngineBridge.processRequest([UInt8](request.serializedData()))
        let response = try Taigi_Engine_Response(serializedBytes: Data(responseBytes))

        XCTAssertEqual(response.error, .ok, "engine reported \(response.error) for a valid request")
        XCTAssertEqual(response.id, request.id, "response must echo the request id")
        guard case let .phonetics(phoneticsResponse)? = response.payload else {
            return XCTFail("expected a phonetics payload, got \(String(describing: response.payload))")
        }
        guard case let .stringResult(stringResult)? = phoneticsResponse.result else {
            return XCTFail("expected a stringResult, got \(String(describing: phoneticsResponse.result))")
        }
        XCTAssertEqual(stringResult.output, "g\u{00F3}a", "TL guá must convert to POJ góa in NFC")
    }

    /// Negative control, and the macOS side of case T4 in
    /// `docs/engine/ffi-safety.md` (iOS runs it as
    /// `RustEngineBridgeTests.test_T4_malformedBytes_returnsFailParse`).
    /// A happy path alone cannot tell "the Rust dispatcher answered" from
    /// "something Swift-side handed back a default `Response`": only the Rust
    /// side turns undecodable bytes into `FAIL_PARSE`.
    func testProcessRequest_malformedBytes_returnsFailParse() throws {
        let responseBytes = RustEngineBridge.processRequest([0xFF, 0xFF, 0xFF, 0xFF])
        let response = try Taigi_Engine_Response(serializedBytes: Data(responseBytes))

        XCTAssertEqual(response.error, .failParse, "malformed bytes must come back as FAIL_PARSE")
    }
}
