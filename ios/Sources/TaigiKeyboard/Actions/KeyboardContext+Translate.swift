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
    /// Inert unless 候選詞顯示 = 漢羅並排: under 羅馬字 there is no hanji to
    /// lead with, under 漢羅濫 every cell is already single-script (the split
    /// happens upstream, each cell commits its own script), and toggling
    /// the derived getter would overwrite the stored flag. The key stays
    /// visible; its active state reads the derived value.
    // 中文: 只有 漢羅並排 才切換;羅馬字 / 漢羅濫 為 no-op — 鍵仍顯示,但不改 stored 值。
    func toggleTranslateSwapped() {
        guard candidateDisplayMode.allowsSwapToggle else { return }
        isTranslateSwapped.toggle()
    }

    private func notifyDisplayChange() {
        DispatchQueue.main.async {
            self.objectWillChange.send()
        }
    }
}
