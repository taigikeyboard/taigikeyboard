//
//  Bundle.swift
//  ISEmojiView
//
//  Created by Beniamin Sarkisyan on 01/08/2018.
//

import Foundation
import UIKit

extension Bundle {

    class var podBundle: Bundle {
        // Since ISEmojiView is now integrated directly into the project,
        // use the bundle that contains the EmojiView class
        return Bundle(for: EmojiView.classForCoder())
    }

}
