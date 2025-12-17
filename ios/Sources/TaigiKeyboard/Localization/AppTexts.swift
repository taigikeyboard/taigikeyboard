import Foundation

/// 向後相容層：統一命名空間入口
/// 所有文字常數已拆分至專門檔案，此 enum 提供統一存取介面
enum AppTexts {
    // MARK: - KeyboardTexts
    static var keyboardSettings: LocalizedText { KeyboardTexts.keyboardSettings }
    static var inputMode: LocalizedText { KeyboardTexts.inputMode }
    static var pojMode: LocalizedText { KeyboardTexts.pojMode }
    static var tlMode: LocalizedText { KeyboardTexts.tlMode }
    static var autoCapitalization: LocalizedText { KeyboardTexts.autoCapitalization }
    static var autoSpace: LocalizedText { KeyboardTexts.autoSpace }
    static var inputSettings: LocalizedText { KeyboardTexts.inputSettings }
    static var layoutSettings: LocalizedText { KeyboardTexts.layoutSettings }
    static var doubleTapCombination: LocalizedText { KeyboardTexts.doubleTapCombination }
    static var doubleTapOO: LocalizedText { KeyboardTexts.doubleTapOO }
    static var doubleTapNN: LocalizedText { KeyboardTexts.doubleTapNN }
    static var phahTaigiLayout: LocalizedText { KeyboardTexts.phahTaigiLayout }
    static var customFont: LocalizedText { KeyboardTexts.customFont }
    static var fontSystemDefault: LocalizedText { KeyboardTexts.fontSystemDefault }
    static var fontOpenHuninn: LocalizedText { KeyboardTexts.fontOpenHuninn }
    static var fontIansui: LocalizedText { KeyboardTexts.fontIansui }
    static var fontSystemWarning: LocalizedText { KeyboardTexts.fontSystemWarning }
    static var outputBothScripts: LocalizedText { KeyboardTexts.outputBothScripts }

    // MARK: - ActionTexts
    static var done: LocalizedText { ActionTexts.done }
    static var cancel: LocalizedText { ActionTexts.cancel }
    static var confirmKey: LocalizedText { ActionTexts.confirmKey }
    static var confirm: LocalizedText { ActionTexts.confirm }
    static var clearCache: LocalizedText { ActionTexts.clearCache }
    static var clearCacheMessage: LocalizedText { ActionTexts.clearCacheMessage }
    static var clear: LocalizedText { ActionTexts.clear }
    static var resetSettings: LocalizedText { ActionTexts.resetSettings }
    static var resetSettingsMessage: LocalizedText { ActionTexts.resetSettingsMessage }
    static var reset: LocalizedText { ActionTexts.reset }

    // MARK: - OnboardingTexts
    static var onboardingWelcomeTitle: LocalizedText { OnboardingTexts.onboardingWelcomeTitle }
    static var onboardingWelcomeMessage: LocalizedText { OnboardingTexts.onboardingWelcomeMessage }
    static var onboardingAddKeyboardTitle: LocalizedText { OnboardingTexts.onboardingAddKeyboardTitle }
    static var onboardingCompletedTitle: LocalizedText { OnboardingTexts.onboardingCompletedTitle }
    static var onboardingCompletedMessage: LocalizedText { OnboardingTexts.onboardingCompletedMessage }
    static var onboardingGoToSettings: LocalizedText { OnboardingTexts.onboardingGoToSettings }
    static var onboardingSkip: LocalizedText { OnboardingTexts.onboardingSkip }
    static var onboardingGetStarted: LocalizedText { OnboardingTexts.onboardingGetStarted }
    static var onboardingStartSetup: LocalizedText { OnboardingTexts.onboardingStartSetup }
    static var onboardingKeyboardNotEnabled: LocalizedText { OnboardingTexts.onboardingKeyboardNotEnabled }
    static var onboardingKeyboardEnabled: LocalizedText { OnboardingTexts.onboardingKeyboardEnabled }
    static var onboardingFullAccessNotEnabled: LocalizedText { OnboardingTexts.onboardingFullAccessNotEnabled }
    static var onboardingFullAccessEnabled: LocalizedText { OnboardingTexts.onboardingFullAccessEnabled }
    static var onboardingStep1Settings: LocalizedText { OnboardingTexts.onboardingStep1Settings }
    static var onboardingStep2AddKeyboard: LocalizedText { OnboardingTexts.onboardingStep2AddKeyboard }
    static var onboardingStep3FullAccess: LocalizedText { OnboardingTexts.onboardingStep3FullAccess }

    // MARK: - CopyrightTexts
    static var copyrightNotice: LocalizedText { CopyrightTexts.copyrightNotice }
    static var viewLicense: LocalizedText { CopyrightTexts.viewLicense }
    static var viewWebsite: LocalizedText { CopyrightTexts.viewWebsite }
    static var moeDict: LocalizedText { CopyrightTexts.moeDict }
    static var moeCopyright: LocalizedText { CopyrightTexts.moeCopyright }
    static var ccLicense: LocalizedText { CopyrightTexts.ccLicense }
    static var iTaigiDict: LocalizedText { CopyrightTexts.iTaigiDict }
    static var iTaigiCopyright: LocalizedText { CopyrightTexts.iTaigiCopyright }
    static var cc0License: LocalizedText { CopyrightTexts.cc0License }
    static var newwordDict: LocalizedText { CopyrightTexts.newwordDict }
    static var newwordCopyright: LocalizedText { CopyrightTexts.newwordCopyright }
    static var ccBy4License: LocalizedText { CopyrightTexts.ccBy4License }
    static var openFontTitle: LocalizedText { CopyrightTexts.openFontTitle }
    static var openFontCopyright: LocalizedText { CopyrightTexts.openFontCopyright }
    static var silOpenFontLicense: LocalizedText { CopyrightTexts.silOpenFontLicense }
    static var taiwanPlantDict: LocalizedText { CopyrightTexts.taiwanPlantDict }
    static var taiwanPlantCopyright: LocalizedText { CopyrightTexts.taiwanPlantCopyright }
    static var ccBySA4License: LocalizedText { CopyrightTexts.ccBySA4License }
    static var taiHuaDict: LocalizedText { CopyrightTexts.taiHuaDict }
    static var taiHuaCopyright: LocalizedText { CopyrightTexts.taiHuaCopyright }
    static var taiwanJapanDict: LocalizedText { CopyrightTexts.taiwanJapanDict }
    static var taiwanJapanCopyright: LocalizedText { CopyrightTexts.taiwanJapanCopyright }
    static var ccByNcSA3License: LocalizedText { CopyrightTexts.ccByNcSA3License }
    static var iansuiFontTitle: LocalizedText { CopyrightTexts.iansuiFontTitle }
    static var iansuiFontCopyright: LocalizedText { CopyrightTexts.iansuiFontCopyright }
    static var silOpenFontLicense11: LocalizedText { CopyrightTexts.silOpenFontLicense11 }
    static var kunggeDict: LocalizedText { CopyrightTexts.kunggeDict }
    static var kunggeCopyright: LocalizedText { CopyrightTexts.kunggeCopyright }
    static var ccByNcLicense: LocalizedText { CopyrightTexts.ccByNcLicense }

    // MARK: - AppInfoTexts
    static var appTitle: LocalizedText { AppInfoTexts.appTitle }
    static var footerTagline: LocalizedText { AppInfoTexts.footerTagline }
    static var copyright: LocalizedText { AppInfoTexts.copyright }
    static var setupGuide: LocalizedText { AppInfoTexts.setupGuide }
    static var setupInfoMessage: LocalizedText { AppInfoTexts.setupInfoMessage }
    static var dictionarySettings: LocalizedText { AppInfoTexts.dictionarySettings }
    static var customDictionary: LocalizedText { AppInfoTexts.customDictionary }
    static var comingSoon: LocalizedText { AppInfoTexts.comingSoon }
    static var variantDictionary: LocalizedText { AppInfoTexts.variantDictionary }
    static var rateUs: LocalizedText { AppInfoTexts.rateUs }
    static var contactUs: LocalizedText { AppInfoTexts.contactUs }
    static var userGuide: LocalizedText { AppInfoTexts.userGuide }
    static var guidePreviousPage: LocalizedText { AppInfoTexts.guidePreviousPage }
    static var guideNextPage: LocalizedText { AppInfoTexts.guideNextPage }
}
