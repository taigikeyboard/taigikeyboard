//
//  Constants.swift
//  ISEmojiView
//
//  Created by Beniamin Sarkisyan on 01/08/2018.
//

import Foundation
import UIKit

let EmojiSize = CGSize(width: 56, height: 42)
let EmojiFont = UIFont(name: "Apple color emoji", size: 36)
let TopPartSize = CGSize(width: EmojiSize.width * 1.3, height: EmojiSize.height * 1.6)
let BottomPartSize = CGSize(width: EmojiSize.width * 0.8, height: EmojiSize.height + 10)
let EmojiPopViewSize = CGSize(width: TopPartSize.width, height: TopPartSize.height + BottomPartSize.height)
let CollectionMinimumLineSpacing = CGFloat(0)
let CollectionMinimumInteritemSpacing = CGFloat(0)

public let MaxCountOfRecentsEmojis: Int = 50
