// Lexicon slice of the engine bridge: loading the dictionary data.

import Foundation

/// Record counts the engine reports after loading. Their only job is to make a
/// successful install self-evident in the log — an install that silently loads
/// zero records looks exactly like a working one until the user types.
struct LexiconInstallStats: Equatable {
    let dictionaryRecordCount: UInt64
    let prefixIndexEntryCount: UInt64
}

extension RustEngineBridge {
    /// Points the engine at the dictionary data. Sent once per process: the
    /// files are read-only and outlive every composing session.
    ///
    /// `dictionaryVersion` is the platform's stamp for the data it shipped; the
    /// engine records it so a later slice can tell which build's dictionary the
    /// user's data was learned against.
    ///
    /// `nil` means the engine has no lexicon installed. Nothing here retries or
    /// falls back: a search against an uninstalled engine returns no candidates,
    /// which is the same graceful degradation any other empty result produces.
    static func lexiconInstall(
        artifacts: DictionaryArtifacts,
        dictionaryVersion: UInt32
    ) -> LexiconInstallStats? {
        var install = Taigi_Engine_InstallRequest()
        install.triePath = artifacts.triePath
        install.dictionaryBinPath = artifacts.dictionaryBinPath
        install.associationBinPath = artifacts.associationBinPath
        install.dictionaryVersion = dictionaryVersion
        install.syllableInventoryPath = artifacts.syllableInventoryPath

        var lexicon = Taigi_Engine_LexiconRequest()
        lexicon.method = .install(install)

        let op = "lexiconInstall"
        guard let payload = roundtrip(payload: .lexicon(lexicon), op: op) else { return nil }
        guard case let .lexicon(response) = payload else {
            recordFailure(op: op, message: "expected a lexicon payload, got \(payload)")
            return nil
        }
        guard case let .installResult(result)? = response.result else {
            recordFailure(op: op, message: "response carried no install result")
            return nil
        }
        return LexiconInstallStats(
            dictionaryRecordCount: result.dictionaryRecordCount,
            prefixIndexEntryCount: result.prefixIndexEntryCount
        )
    }
}
