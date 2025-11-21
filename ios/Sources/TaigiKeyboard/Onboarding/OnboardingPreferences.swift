import SwiftUI

/// 引導流程偏好設定（使用 AppStorage 儲存）
struct OnboardingPreferences {
    @AppStorage("hasSeenWelcome") var hasSeenWelcome = false
}
