import SwiftUI
import StoreKit

/// 贊助支持視圖
struct SponsorshipView: View {
    @Environment(\.dismiss) private var dismiss
    @StateObject private var storeManager = StoreKitManager.shared
    @State private var showThankYou = false
    @State private var showError = false

    private var backgroundView: some View {
        Color.Theme.surfacePrimary
            .ignoresSafeArea()
    }

    private var heroSection: some View {
        VStack(spacing: 24) {
            heroIcon

            VStack(spacing: 14) {
                LocalizedTextView(AppTexts.sponsorTitle)
                    .font(Font.Theme.title)
                    .foregroundColor(Color.Theme.textPrimary)

                LocalizedTextView(AppTexts.sponsorSubtitle)
                    .font(Font.Theme.body)
                    .foregroundColor(Color.Theme.textSecondary)
                    .multilineTextAlignment(.center)
                    .lineSpacing(2)
                    .opacity(0.9)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 28)
        .padding(.vertical, 24)
        .themedCard()
    }

    private var heroIcon: some View {
        Image(systemName: "heart.fill")
            .font(.system(size: 64, weight: .medium))
            .foregroundColor(Color(red: 0.961, green: 0.729, blue: 0.706))
    }

    private var appIntroductionSection: some View {
        VStack(spacing: 20) {
            HStack {
                LocalizedTextView(AppTexts.aboutApp)
                    .font(Font.Theme.headline)
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            VStack(spacing: 16) {
                CommitmentRow(
                    icon: "heart.fill",
                    text: AppTexts.futurePlan4,
                    color: .red
                )
                CommitmentRow(
                    icon: "bubble.left.and.bubble.right.fill",
                    text: AppTexts.futurePlan1,
                    color: .blue
                )
                CommitmentRow(
                    icon: "shield.checkered",
                    text: AppTexts.futurePlan2,
                    color: .green
                )
                CommitmentRow(
                    icon: "books.vertical",
                    text: AppTexts.futurePlan3,
                    color: .purple
                )
            }
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 28)
        .themedCard()
    }

    private var sponsorTiersSection: some View {
        VStack(spacing: 16) {
            HStack {
                LocalizedTextView(AppTexts.oneTimeSponsor)
                    .font(Font.Theme.headline)
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }
            .padding(.horizontal, 4)

            ForEach(SponsorTier.allCases, id: \.self) { tier in
                SponsorTierCard(
                    tier: tier,
                    product: storeManager.product(for: tier.rawValue),
                    isPurchased: storeManager.isPurchased(tier.rawValue),
                    isLoading: storeManager.isLoading,
                    onTap: {
                        Task {
                            await processPurchase(tier: tier)
                        }
                    }
                )
            }
        }
        .padding(.horizontal, 20)
    }

    var body: some View {
        NavigationView {
            GeometryReader { geometry in
                ZStack {
                    backgroundView

                    ScrollView(.vertical, showsIndicators: false) {
                        VStack(spacing: 32) {
                            heroSection

                            appIntroductionSection

                            sponsorTiersSection
                        }
                        .padding(.top, 20)
                        .frame(maxWidth: min(geometry.size.width, 500))
                        .frame(maxWidth: .infinity)
                    }
                }
            }
            .background(Color.Theme.surfacePrimary)
            .navigationTitle("")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { dismiss() }) {
                        Image(systemName: "xmark")
                            .font(.system(size: 16, weight: .medium))
                            .foregroundColor(Color.Theme.textPrimary)
                    }
                }

            }
            .alert(isPresented: $showThankYou) {
                Alert(
                    title: Text(""),
                    message: Text(LanguageManager.shared.text(AppTexts.thankYouMessage)),
                    dismissButton: .default(Text(LanguageManager.shared.text(AppTexts.done)))
                )
            }
            .alert(isPresented: $showError) {
                Alert(
                    title: Text(LanguageManager.shared.text(AppTexts.purchaseFailed)),
                    message: Text(LanguageManager.shared.text(storeManager.errorMessage ?? AppTexts.unknownError)),
                    dismissButton: .default(Text(LanguageManager.shared.text(AppTexts.confirm)))
                )
            }
            .onReceive(storeManager.$errorMessage) { errorMessage in
                if errorMessage != nil {
                    showError = true
                }
            }
        }
    }

    /// 處理贊助購買流程
    private func processPurchase(tier: SponsorTier) async {
        guard let product = storeManager.product(for: tier.rawValue) else {
            storeManager.errorMessage = AppTexts.productNotFound
            return
        }

        do {
            if try await storeManager.purchase(product) != nil {
                showThankYou = true
            }
        } catch {}
    }
}

#Preview {
    SponsorshipView()
        .withLanguageEnvironment()
}
