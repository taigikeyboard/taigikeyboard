import Foundation

extension Bundle {
    /// `CFBundleShortVersionString` — the version the Home tab shows and a
    /// `.taigi` backup names as its writer.
    var shortVersion: String {
        object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0"
    }
}
