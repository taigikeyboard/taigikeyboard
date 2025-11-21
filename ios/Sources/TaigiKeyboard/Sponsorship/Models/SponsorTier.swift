import SwiftUI

/// 贊助等級定義
enum SponsorTier: String, CaseIterable {
    case coffee = "com.siansiansu.taigikeyboard.donate.small"
    case meal = "com.siansiansu.taigikeyboard.donate.medium"
    case premium = "com.siansiansu.taigikeyboard.donate.large"

    var title: LocalizedText {
        switch self {
        case .coffee: return AppTexts.coffeeTier
        case .meal: return AppTexts.mealTier
        case .premium: return AppTexts.premiumTier
        }
    }

    var icon: String {
        switch self {
        case .coffee: return "cup.and.saucer.fill"
        case .meal: return "fork.knife"
        case .premium: return "star.fill"
        }
    }

    var color: Color {
        switch self {
        case .coffee: return Color.Theme.accent
        case .meal: return Color.Theme.accentSecondary
        case .premium: return Color.Theme.accentTertiary
        }
    }
}
