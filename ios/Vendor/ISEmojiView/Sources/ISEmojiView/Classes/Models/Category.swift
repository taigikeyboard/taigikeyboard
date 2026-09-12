//
//  Category.swift
//  ISEmojiView
//
//  Created by Beniamin Sarkisyan on 03/08/2018.
//

import Foundation

public enum Category: Equatable {
    case recents
    case smileysAndPeople
    case animalsAndNature
    case foodAndDrink
    case activity
    case travelAndPlaces
    case objects
    case symbols
    case flags
    case custom(String, String)

    static var count = 10

    var title: String {
        switch self {
        case .recents:
            "Frequently Used"
        case .smileysAndPeople:
            "Smileys & People"
        case .animalsAndNature:
            "Animals & Nature"
        case .foodAndDrink:
            "Food & Drink"
        case .activity:
            "Activity"
        case .travelAndPlaces:
            "Travel & Places"
        case .objects:
            "Objects"
        case .symbols:
            "Symbols"
        case .flags:
            "Flags"
        case let .custom(title, _):
            title
        }
    }

    var iconName: String {
        switch self {
        case .recents:
            "ic_recents"
        case .smileysAndPeople:
            "ic_smileys_people"
        case .animalsAndNature:
            "ic_animals_nature"
        case .foodAndDrink:
            "ic_food_drink"
        case .activity:
            "ic_activity"
        case .travelAndPlaces:
            "ic_travel_places"
        case .objects:
            "ic_objects"
        case .symbols:
            "ic_symbols"
        case .flags:
            "ic_flags"
        case let .custom(_, iconName):
            iconName
        }
    }
}
