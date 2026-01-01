import SwiftUI

/// 圖片輪播元件
///
/// 自動循環播放圖片，用於設定引導頁面。
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
