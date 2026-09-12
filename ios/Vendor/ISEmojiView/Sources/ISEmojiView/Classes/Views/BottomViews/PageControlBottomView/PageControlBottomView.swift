//
//  PageControlBottomView.swift
//  ISEmojiView
//
//  Created by Beniamin Sarkisyan on 01/08/2018.
//

import Foundation
import UIKit

protocol PageControlBottomViewDelegate: AnyObject {
    func pageControlBottomViewDidPressDeleteBackwardButton(_ bottomView: PageControlBottomView)
    func pageControlBottomViewDidPressDismissKeyboardButton(_ bottomView: PageControlBottomView)
}

final class PageControlBottomView: UIView {
    // MARK: - Internal variables

    weak var delegate: PageControlBottomViewDelegate?

    // MARK: - IBOutlets

    @IBOutlet private weak var pageControl: UIPageControl!

    @IBOutlet private weak var deleteButton: UIButton!

    // MARK: - Init functions

    static func loadFromNib(categoriesCount: Int, needToShowDeleteButton: Bool) -> PageControlBottomView {
        let nibName = String(describing: PageControlBottomView.self)

        guard let nib = Bundle.podBundle.loadNibNamed(nibName, owner: nil, options: nil) as? [PageControlBottomView] else {
            fatalError()
        }

        guard let bottomView = nib.first else {
            fatalError()
        }

        bottomView.pageControl.numberOfPages = categoriesCount
        bottomView.deleteButton.isHidden = !needToShowDeleteButton
        return bottomView
    }

    // MARK: - Internal functions

    func updatePageControlPage(_ page: Int) {
        pageControl.currentPage = page
    }

    // MARK: - IBActions

    @IBAction private func deleteBackward() {
        delegate?.pageControlBottomViewDidPressDeleteBackwardButton(self)
    }

    @IBAction private func dismissKeyboard() {
        delegate?.pageControlBottomViewDidPressDismissKeyboardButton(self)
    }
}
