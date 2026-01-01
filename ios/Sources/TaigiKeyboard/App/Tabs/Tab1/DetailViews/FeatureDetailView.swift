import SwiftUI

/// 新功能詳細頁面
///
/// 顯示新功能的詳細說明。
struct FeatureDetailView: View {
    let feature: FeatureType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(feature.detailParagraphs.indices, id: \.self) { index in
                Section {
                    // 連紲建議詞第 1 段：文字 + 圖片輪播
                    if feature == .nextWord && index == 0 {
                        VStack(alignment: .leading, spacing: 12) {
                            Text(languageManager.text(feature.detailParagraphs[index]))
                                .lineSpacing(6)
                                .fixedSize(horizontal: false, vertical: true)

                            ImageSlideshowView(
                                imageNames: ["nextword_1", "nextword_2", "nextword_3"],
                                interval: 1.5
                            )
                            .frame(maxWidth: .infinity)
                        }
                    }
                    // 拍字記持詞庫第 2 段：文字 + 圖片
                    else if feature == .userDict && index == 1 {
                        VStack(alignment: .leading, spacing: 12) {
                            Text(languageManager.text(feature.detailParagraphs[index]))
                                .lineSpacing(6)
                                .fixedSize(horizontal: false, vertical: true)

                            Image("feature_userdict")
                                .resizable()
                                .aspectRatio(contentMode: .fit)
                                .frame(maxWidth: .infinity)
                        }
                    }
                    // 大小寫切換：每段文字 + 對應圖片
                    else if feature == .caseSwitch {
                        VStack(alignment: .leading, spacing: 12) {
                            Text(languageManager.text(feature.detailParagraphs[index]))
                                .lineSpacing(6)
                                .fixedSize(horizontal: false, vertical: true)

                            if index == 0 {
                                // shift 圖片輪播
                                ImageSlideshowView(
                                    imageNames: ["case_shift_1", "case_shift_2"],
                                    interval: 1.5
                                )
                                .frame(maxWidth: .infinity)
                            } else if index == 1 {
                                // lowercase 圖片
                                Image("case_lowercase")
                                    .resizable()
                                    .aspectRatio(contentMode: .fit)
                                    .frame(maxWidth: .infinity)
                            } else if index == 2 {
                                // capslock 圖片
                                Image("case_capslock")
                                    .resizable()
                                    .aspectRatio(contentMode: .fit)
                                    .frame(maxWidth: .infinity)
                            }
                        }
                    }
                    else {
                        Text(languageManager.text(feature.detailParagraphs[index]))
                            .lineSpacing(6)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }

                // 異用字開關：第 1 段後插入教育部辭典連結
                if feature == .variant && index == 0 {
                    Section {
                        Link(destination: URL(string: "https://sutian.moe.edu.tw/zh-hant/siongkuantsuguan/")!) {
                            Label(languageManager.text(Tab1Texts.featureVariantDictLink), systemImage: "arrow.up.right.square")
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(feature.title))
        .navigationBarTitleDisplayMode(.large)
    }
}
