// The HTTPS rules both halves of updating apply to what they fetch.

import Foundation

/// Shared by the manifest check and the package download, so the one clause
/// that carries the security here is written once.
enum UpdateHTTP {
    /// A session that leaves nothing behind.
    ///
    /// Ephemeral, so no cache or cookie outlives the transfer, and both timeouts
    /// set: `timeoutIntervalForRequest` alone bounds only the wait for the next
    /// byte, never the whole transfer.
    static func session(requestTimeout: TimeInterval, resourceTimeout: TimeInterval) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = requestTimeout
        configuration.timeoutIntervalForResource = resourceTimeout
        return URLSession(configuration: configuration)
    }

    /// Whether this is a 200 that actually arrived over HTTPS.
    ///
    /// The scheme is read back off the response rather than trusted from the
    /// request, because the release CDN redirects (measured: `github.com` →
    /// `release-assets.githubusercontent.com`). The only scheme that says
    /// anything about how the bytes travelled is the one they arrived under.
    static func isAcceptable(_ response: URLResponse) -> Bool {
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else { return false }
        return response.url?.scheme?.lowercased() == "https"
    }
}
