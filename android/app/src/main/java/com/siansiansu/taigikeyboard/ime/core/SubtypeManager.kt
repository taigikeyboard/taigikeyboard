// Parses PrefHelper.subtypes string into a List<Subtype> and determines the active subtype.

package com.siansiansu.taigikeyboard.ime.core

class SubtypeManager(
    private val prefs: PrefHelper,
) {
    companion object {
        const val SUBTYPE_LIST_STR_DELIMITER = ";"
    }

    val subtypes: List<Subtype>
        get() {
            val listRaw = prefs.subtypes
            return if (listRaw.isBlank()) {
                listOf()
            } else {
                listRaw.split(SUBTYPE_LIST_STR_DELIMITER).map {
                    Subtype.fromString(it)
                }
            }
        }

    /**
     * Gets the active subtype and returns it. If the activeSubtypeId points to a non-existent
     * subtype, this method tries to determine a new active subtype.
     *
     * @returns The active subtype or null, if the subtype list is empty or no new active subtype
     *  could be determined.
     */
    fun getActiveSubtype(): Subtype? {
        for (subtype in subtypes) {
            if (subtype.id == prefs.activeSubtypeId) {
                return subtype
            }
        }
        val subtypeList = subtypes
        return if (subtypeList.isNotEmpty()) {
            prefs.activeSubtypeId = subtypeList[0].id
            subtypeList[0]
        } else {
            prefs.activeSubtypeId = -1
            null
        }
    }
}
