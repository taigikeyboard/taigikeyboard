import Foundation

extension String {
    /// Replace the first occurrence of `target` with `replacement`.
    func replacingFirst(of target: String, with replacement: String) -> String {
        guard let range = range(of: target) else { return self }
        var result = self
        result.replaceSubrange(range, with: replacement)
        return result
    }
}
