# TaigiKeyboard 台語齒盤

A Taiwanese input method for iOS, Android, macOS, Windows, and Linux.

![iOS 17+](https://img.shields.io/badge/iOS-17%2B-blue)
![Android 9+](https://img.shields.io/badge/Android-9%2B-green)
![macOS 14+](https://img.shields.io/badge/macOS-14%2B-lightgrey)
![Windows 10+](https://img.shields.io/badge/Windows-10%2B-blue)
![Linux Fcitx5 | IBus](https://img.shields.io/badge/Linux-Fcitx5%20%7C%20IBus-orange)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-Join_chat-5865F2?logo=discord&logoColor=white)](https://discord.gg/UFn9RdynPu)

## Download

| Platform | Download |
| --- | --- |
| iOS / iPadOS | [App Store](https://apps.apple.com/app/id6751871806) |
| Android | [Google Play](https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard) |
| macOS / Windows | [taigikeyboard.tw](https://taigikeyboard.tw) |
| Linux | [GitHub Releases](https://github.com/taigikeyboard/taigikeyboard/releases) (`.deb`, `.rpm`, Arch; Fcitx5 or IBus) |

See the [installation guide](docs/INSTALL.md) for setup instructions.

## Features

- Input in Pe̍h-ōe-jī (POJ), Tâi-lô (TL), and Taiwanese Phonetic Symbols (TPS)
- Hanji and mixed Hanji–Romanization output
- Continuous multi-syllable typing
- Tone marks, tone numbers, and tone variation
- Autocomplete; next-word prediction on iOS and Android
- Custom dictionary and frequency learning
- Lossless TL ↔ POJ ↔ TPS conversion

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/system-overview.md)
- [Build and test](docs/BUILDING.md)
- [Release history](CHANGELOG.md)

## Contributing

See the [contributing guide](CONTRIBUTING.md) to submit changes. Report security vulnerabilities through the [security policy](SECURITY.md).

## Community

[![Join Discord](https://img.shields.io/badge/Join_Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/UFn9RdynPu)

Ask questions, share feedback, and report problems.

## Support

<a href="https://ko-fi.com/N8J429DBK4"><img height="36" src="https://storage.ko-fi.com/cdn/kofi5.png?v=6" alt="Support me on Ko-fi" /></a>
<a href="https://www.paypal.com/ncp/payment/AZNAN8NSJXR7J"><img height="36" src="https://www.paypalobjects.com/en_US/i/btn/btn_donateCC_LG.gif" alt="Donate with PayPal (outside Taiwan only, USD)" /></a>

Support development and maintenance via Ko-fi or PayPal (outside Taiwan only, USD).

For Taiwan, scan or click the ECPay QR code:

<a href="https://p.ecpay.com.tw/5256DCE"><img width="200" height="200" src="https://payment.ecpay.com.tw/Upload/QRCode/202403/QRCode_e2510983-82aa-4501-8d92-33a5e154e34f.png" alt="Support via ECPay (Taiwan only)" /></a>

## License

Source code: [Apache License 2.0](LICENSE).

**Dictionary data is not Apache-licensed.** Each source keeps its own terms, and one carries a NonCommercial clause, so the compiled dictionary shipped in every app is non-commercial. See [dictionary licenses](dictionary/LICENSE) for terms and attribution, and [third-party licenses](THIRD_PARTY_LICENSES.md) for fonts and vendored code.

Released binaries follow the [Code signing policy](docs/CODE_SIGNING_POLICY.md).
