import SwiftUI

/// 版權聲明視圖
struct CopyrightView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var currentPage = 0
    @Environment(\.openURL) private var openURL

    private let copyrightPages: [CopyrightPage] = [
        // 字體
        CopyrightPage(
            id: 0,
            title: AppTexts.openFontTitle,
            description: AppTexts.openFontCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.silOpenFontLicense,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://justfont.com/huninn/"
                )
            ]
        ),
        CopyrightPage(
            id: 1,
            title: AppTexts.iansuiFontTitle,
            description: AppTexts.iansuiFontCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.silOpenFontLicense11,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://github.com/ButTaiwan/iansui"
                )
            ]
        ),
        // 辭典
        CopyrightPage(
            id: 2,
            title: AppTexts.moeDict,
            description: AppTexts.moeCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccLicense,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by-nd/3.0/tw/"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://sutian.moe.edu.tw/"
                )
            ]
        ),
        CopyrightPage(
            id: 3,
            title: AppTexts.newwordDict,
            description: AppTexts.newwordCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccBy4License,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://www.taigitv.org.tw/taigi-words"
                )
            ]
        ),
        CopyrightPage(
            id: 4,
            title: AppTexts.kunggeDict,
            description: AppTexts.kunggeCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccByNcLicense,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"
                )
            ]
        ),
        CopyrightPage(
            id: 5,
            title: AppTexts.iTaigiDict,
            description: AppTexts.iTaigiCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.cc0License,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/public-domain/cc0/"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://itaigi.tw/"
                )
            ]
        ),
        CopyrightPage(
            id: 6,
            title: AppTexts.taiwanJapanDict,
            description: AppTexts.taiwanJapanCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccByNcSA3License,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "http://taigi.fhl.net/dict/"
                )
            ]
        ),
        CopyrightPage(
            id: 7,
            title: AppTexts.taiHuaDict,
            description: AppTexts.taiHuaCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccBySA4License,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                )
            ]
        ),
        CopyrightPage(
            id: 8,
            title: AppTexts.taiwanPlantDict,
            description: AppTexts.taiwanPlantCopyright,
            accentColor: Color.Theme.accent,
            licenseDescription: AppTexts.ccBySA4License,
            buttons: [
                CopyrightButton(
                    icon: "doc.text",
                    text: AppTexts.viewLicense,
                    url: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                ),
                CopyrightButton(
                    icon: "safari",
                    text: AppTexts.viewWebsite,
                    url: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"
                )
            ]
        )
    ]

    @State private var isPressed: [Bool]

    init() {
        let totalButtons = copyrightPages.reduce(0) { $0 + $1.buttons.count }
        _isPressed = State(initialValue: Array(repeating: false, count: totalButtons))
    }

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                CopyrightHeader(
                    currentPage: $currentPage,
                    totalPages: copyrightPages.count,
                    onDismiss: { dismiss() }
                )
                .padding(.top, geometry.safeAreaInsets.top)

                ZStack {
                    ForEach(copyrightPages.indices, id: \.self) { index in
                        CopyrightPageView(
                            page: copyrightPages[index],
                            isActive: index == currentPage,
                            geometry: geometry,
                            isPressed: $isPressed,
                            openURL: openURL
                        )
                        .opacity(index == currentPage ? 1 : 0)
                        .scaleEffect(index == currentPage ? 1 : 0.97)
                        .animation(
                            .spring(response: 0.7, dampingFraction: 0.85),
                            value: currentPage
                        )
                    }
                }
                .gesture(
                    DragGesture()
                        .onEnded { value in
                            let threshold: CGFloat = 50
                            if value.translation.width > threshold, currentPage > 0 {
                                withAnimation(.spring(response: 0.5, dampingFraction: 0.8)) {
                                    currentPage -= 1
                                }
                            } else if value.translation.width < -threshold, currentPage < copyrightPages.count - 1 {
                                withAnimation(.spring(response: 0.5, dampingFraction: 0.8)) {
                                    currentPage += 1
                                }
                            }
                        }
                )

                CopyrightBottomNavigation(
                    currentPage: $currentPage,
                    totalPages: copyrightPages.count,
                    onDismiss: { dismiss() }
                )
                .padding(.horizontal, 20)
                .padding(.bottom, max(20, geometry.safeAreaInsets.bottom))
            }
        }
        .background(
            Color.Theme.surfacePrimary
                .ignoresSafeArea()
        )
        .navigationBarHidden(true)
    }
}

#Preview {
    CopyrightView()
        .withLanguageEnvironment()
}
