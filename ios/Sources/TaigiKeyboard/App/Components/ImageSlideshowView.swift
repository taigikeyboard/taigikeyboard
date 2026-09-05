// 自動循環的圖片輪播 View,setup guide 與 feature 詳情頁用。

import SwiftUI

/// Auto-cycling image slideshow.
///
/// Used in setup guide and feature detail pages.
// 自動循環圖片輪播 View。
// imageNames 為 asset 名稱清單;interval 為切換秒數(預設 2 秒)。
struct ImageSlideshowView: View {
    let imageNames: [String]
    let interval: TimeInterval

    init(imageNames: [String], interval: TimeInterval = 2.0) {
        self.imageNames = imageNames
        self.interval = interval
    }

    var body: some View {
        if imageNames.isEmpty {
            EmptyView()
        } else {
            TimelineView(.periodic(from: .now, by: interval)) { context in
                let index = Int(context.date.timeIntervalSince1970 / interval) % imageNames.count
                if let uiImage = UIImage(named: imageNames[index]) {
                    Image(uiImage: uiImage)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: .infinity)
                }
            }
        }
    }
}
