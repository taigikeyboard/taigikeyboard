import SwiftUI
import StoreKit

/// 贊助等級卡片組件
struct SponsorTierCard: View {
    let tier: SponsorTier
    let product: Product?
    let isPurchased: Bool
    let isLoading: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 18) {
                Image(systemName: tier.icon)
                    .font(.system(size: 24, weight: .semibold))
                    .foregroundColor(tier.color)
                    .frame(width: 32, height: 32)

                VStack(alignment: .leading, spacing: 8) {
                    LocalizedTextView(tier.title)
                        .font(Font.Theme.headline)
                        .foregroundColor(Color.Theme.textPrimary)

                    HStack(spacing: 8) {
                        if let product = product {
                            Text(StoreKitManager.shared.formattedPrice(for: product))
                                .font(.system(size: 22, weight: .bold))
                                .foregroundColor(tier.color)
                        } else if isLoading {
                            ProgressView()
                                .scaleEffect(0.8)
                        } else {
                            LocalizedTextView(AppTexts.priceLoadFailed)
                                .font(.system(size: 14, weight: .medium))
                                .foregroundColor(Color.Theme.textSecondary)
                        }

                        if isPurchased {
                            LocalizedTextView(AppTexts.alreadySponsored)
                                .font(.system(size: 12, weight: .medium))
                                .foregroundColor(Color.Theme.accent)
                        }
                    }
                }

                Spacer()

                if isPurchased {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 26))
                        .foregroundColor(tier.color)
                } else {
                    Image(systemName: "arrow.right.circle.fill")
                        .font(.system(size: 26))
                        .foregroundColor(tier.color)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 20)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(Color.Theme.surfaceSecondary)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(tier.color.opacity(0.3), lineWidth: 1)
                    )
            )
        }
        .buttonStyle(PlainButtonStyle())
        .disabled(isPurchased || isLoading || product == nil)
        .opacity(isPurchased ? 0.7 : (product == nil && !isLoading ? 0.5 : 1))
    }
}
