// Typed state for the currently visible Smartbar container — replaces raw R.id Int tags.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.view.View
import com.siansiansu.taigikeyboard.R

/**
 * One of the four mutually-exclusive Smartbar content containers visible at any
 * given time. The `id` constructor property is the matching `R.id.*` for the
 * inflated XML; `SmartbarView` already caches a typed `View?` accessor for
 * each, so [viewIn] is the preferred resolver (avoids a `findViewById` walk).
 */
enum class SmartbarContainer(
    val id: Int,
) {
    NUMBER_ROW(R.id.number_row),
    CANDIDATES(R.id.candidates_container),
    ENGLISH_CANDIDATES(R.id.english_candidates_container),
    TOOLBAR(R.id.toolbar_container),
    ;

    fun viewIn(smartbarView: SmartbarView): View? =
        when (this) {
            NUMBER_ROW -> smartbarView.numberRowView
            CANDIDATES -> smartbarView.candidatesContainer
            ENGLISH_CANDIDATES -> smartbarView.englishCandidatesContainer
            TOOLBAR -> smartbarView.toolbarContainer
        }
}
