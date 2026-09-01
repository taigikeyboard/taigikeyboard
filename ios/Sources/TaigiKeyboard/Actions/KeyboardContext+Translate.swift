// 中文: 為 KeyboardKit KeyboardContext 加上漢字 / 羅馬字顯示切換的擴充,後端寫進 SharedSettings。

import Combine
import KeyboardKit
import SwiftUI

/// KeyboardContext 翻譯狀態擴展
///
/// 提供漢字／羅馬字顯示模式切換功能。
public extension KeyboardContext {
    /// 是否為漢字優先模式（true = 漢字, false = 羅馬字）
    ///
    /// The getter is the DERIVED value (`false` while 候選詞顯示 = 羅馬字); the
    /// setter writes the stored flag so a stored `true` survives the mode.
    var isTranslateSwapped: Bool {
        get { SharedSettings.shared.isTranslateSwapped }
        set {
            SharedSettings.shared.storedIsTranslateSwapped = newValue
            notifyDisplayChange()
        }
    }

    /// Candidate cell rendering mode, mirrored on the context so a change from
    /// the in-keyboard settings overlay re-renders the strip + expanded overlay
    /// through the same `objectWillChange` path the 文/A toggle uses.
    // 中文: 候選詞顯示模式。鍵盤內 overlay 改值走這裡,與 文/A 切換共用同一條重繪路徑。
    var candidateDisplayMode: CandidateDisplayMode {
        get { SharedSettings.shared.candidateDisplayMode }
        set {
            SharedSettings.shared.candidateDisplayMode = newValue
            notifyDisplayChange()
        }
    }

    /// 切換顯示模式
    ///
    /// Inert under 羅馬字: there is no hanji to lead with, and toggling the
    /// derived-false getter would overwrite a stored `true`. The key stays
    /// visible; its active state reads the derived value (`false`).
    // 中文: 羅馬字模式下為 no-op(§3 Q11)— 鍵仍顯示,但不改 stored 值。
    func toggleTranslateSwapped() {
        guard candidateDisplayMode != .romanOnly else { return }
        isTranslateSwapped.toggle()
    }

    private func notifyDisplayChange() {
        DispatchQueue.main.async {
            self.objectWillChange.send()
        }
    }
}
