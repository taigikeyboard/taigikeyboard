// 中文: DictionaryTab 詞典 info button 用的 metadata — 顯示說明文字與外部連結。
// 中文: extension 內每個 static 對應一個系統詞典(教育部 / iTaigi / 台日大辭典 …)。

import Foundation

/// Dictionary metadata for info buttons in DictionaryTab.
// 中文: 詞典資訊資料 model:說明文字 + 詞典官網連結。
struct DictionaryInfo {
    let description: String
    let websiteURL: URL?
}

extension DictionaryInfo {
    static let moe = DictionaryInfo(
        description: "教育部編纂，收錄台語常用詞。",
        websiteURL: URL(string: "https://sutian.moe.edu.tw/"),
    )
    static let stti = DictionaryInfo(
        description: "教育部提供逐學科專業術語ê台語對譯。",
        websiteURL: URL(string: "https://stti.moe.edu.tw/"),
    )
    static let newword = DictionaryInfo(
        description: "公視台語台整理ê台語新詞。",
        websiteURL: URL(string: "https://www.taigitv.org.tw/taigi-words"),
    )
    static let kungge = DictionaryInfo(
        description: "國立臺灣工藝研究發展中心收錄ê台語工藝相關台語詞。",
        websiteURL: URL(string: "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"),
    )
    static let iTaigi = DictionaryInfo(
        description: "一个群眾編輯ê開放台語辭典",
        websiteURL: URL(string: "https://itaigi.tw/"),
    )
    static let taiwanJapan = DictionaryInfo(
        description: "日本時代小川尚義編纂ê台語辭典。",
        websiteURL: URL(string: "http://taigi.fhl.net/dict/"),
    )
    static let taiHua = DictionaryInfo(
        description: "「台華線頂辭典」是鄭良偉教授提供資料、楊允言教授編修",
        websiteURL: nil,
    )
    static let taiwanPlant = DictionaryInfo(
        description: "日本時代佐佐木舜一整理ê台灣植物台語名。",
        websiteURL: URL(string: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"),
    )
    static let variant = DictionaryInfo(
        description: "依據教典資料標示台語異用字。",
        websiteURL: nil,
    )
    static let khpoo = DictionaryInfo(
        description: "補充在地腔口差異",
        websiteURL: nil,
    )
    static let khiin = DictionaryInfo(
        description: "「水台文」、「台字田」用字",
        websiteURL: nil,
    )
    static let lkk = DictionaryInfo(
        description: "李江却台語文教基金會漢羅合用建議用字。",
        websiteURL: nil,
    )
}
