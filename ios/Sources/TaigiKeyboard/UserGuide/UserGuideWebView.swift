import SwiftUI
import WebKit

/// 嵌入的 WebView 用於顯示操作說明網頁
struct UserGuideWebView: UIViewRepresentable {
    let url: URL
    let onLoadingChange: (Bool) -> Void

    func makeUIView(context: Context) -> WKWebView {
        let webView = WKWebView()
        webView.navigationDelegate = context.coordinator

        // WebView Configuration
        webView.backgroundColor = UIColor.clear
        webView.isOpaque = false
        webView.layer.cornerRadius = 20
        webView.layer.masksToBounds = true

        return webView
    }

    /// 更新 WebView(僅第一次載入以避免無限循環)
    func updateUIView(_ uiView: WKWebView, context: Context) {
        guard !context.coordinator.hasLoadedInitially else { return }

        let request = URLRequest(url: url)
        uiView.load(request)
        context.coordinator.hasLoadedInitially = true
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(onLoadingChange: onLoadingChange)
    }

    class Coordinator: NSObject, WKNavigationDelegate {
        let onLoadingChange: (Bool) -> Void
        var hasLoadedInitially = false

        init(onLoadingChange: @escaping (Bool) -> Void) {
            self.onLoadingChange = onLoadingChange
        }

        func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
            onLoadingChange(true)
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            onLoadingChange(false)
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
            onLoadingChange(false)
        }
    }
}
