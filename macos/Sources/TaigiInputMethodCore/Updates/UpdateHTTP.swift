// The HTTPS rules both halves of updating apply to what they fetch.

import Foundation

/// Shared by the manifest check and the package download, so the one clause
/// that carries the security here is written once.
enum UpdateHTTP {
    /// Ephemeral session configuration that leaves no cache or cookies behind
    /// and sets both per-request idle and whole-transfer resource timeouts.
    static func configuration(
        requestTimeout: TimeInterval,
        resourceTimeout: TimeInterval,
    ) -> URLSessionConfiguration {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = requestTimeout
        configuration.timeoutIntervalForResource = resourceTimeout
        return configuration
    }

    /// A session that leaves nothing behind.
    ///
    /// Ephemeral, so no cache or cookie outlives the transfer, and both timeouts
    /// set: `timeoutIntervalForRequest` alone bounds only the wait for the next
    /// byte, never the whole transfer.
    static func session(requestTimeout: TimeInterval, resourceTimeout: TimeInterval) -> URLSession {
        URLSession(
            configuration: configuration(
                requestTimeout: requestTimeout,
                resourceTimeout: resourceTimeout,
            ),
        )
    }

    /// Whether `url` uses `https` and names a non-empty host.
    ///
    /// Foundation's `URL(string:)` accepts hostless forms such as `https:///path`
    /// (`host == nil`) and `https://:443/path` or `https://user@/path` (`host == ""`),
    /// so checking the scheme alone is not enough.
    static func isHTTPS(_ url: URL) -> Bool {
        guard url.scheme?.lowercased() == "https",
              let host = url.host(percentEncoded: false),
              !host.isEmpty
        else { return false }
        return true
    }

    /// Whether this is a 200 that actually arrived over HTTPS with a non-empty host.
    ///
    /// The URL is read back off the response rather than trusted from the
    /// request, because the release CDN redirects (measured: `github.com` →
    /// `release-assets.githubusercontent.com`). The only URL that says
    /// anything about how the bytes travelled is the one they arrived under.
    static func isAcceptable(_ response: URLResponse) -> Bool {
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else { return false }
        return response.url.map(isHTTPS) ?? false
    }
}
