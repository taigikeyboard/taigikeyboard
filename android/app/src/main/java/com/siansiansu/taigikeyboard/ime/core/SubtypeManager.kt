// 中文: Subtype 管理器 — 從 PrefHelper.subtypes 字串解析出 List<Subtype>、寫回設定、
// 中文: 載入 ime/config.json 取 ImeConfig 預設清單。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import kotlinx.coroutines.*

@Suppress("SameParameterValue")
class SubtypeManager(
    private val context: Context,
    private val prefs: PrefHelper,
) : CoroutineScope by MainScope() {
    companion object {
        const val IME_CONFIG_FILE_PATH = "ime/config.json"
        const val SUBTYPE_LIST_STR_DELIMITER = ";"
    }

    var imeConfig: TaigiKeyboard.ImeConfig = TaigiKeyboard.ImeConfig(context.packageName)
    var subtypes: List<Subtype>
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
        set(v) {
            prefs.subtypes = v.joinToString(SUBTYPE_LIST_STR_DELIMITER)
        }

    init {
        launch(Dispatchers.IO) {
            imeConfig = loadImeConfig(IME_CONFIG_FILE_PATH)
        }
    }

    /**
     * Loads the [TaigiKeyboard.ImeConfig] from ime/config.json.
     *
     * @param path The path to to IME config file.
     * @returns The [TaigiKeyboard.ImeConfig] or a default config.
     */
    private fun loadImeConfig(path: String): TaigiKeyboard.ImeConfig {
        val rawJsonData: String =
            try {
                context.assets
                    .open(path)
                    .bufferedReader()
                    .use { it.readText() }
            } catch (e: Exception) {
                null
            } ?: return TaigiKeyboard.ImeConfig(context.packageName)
        val moshi =
            Moshi
                .Builder()
                .add(KotlinJsonAdapterFactory())
                .add(SubtypeLocaleAdapter.JsonAdapter())
                .build()
        val layoutAdapter = moshi.adapter(TaigiKeyboard.ImeConfig::class.java)
        return layoutAdapter.fromJson(rawJsonData) ?: TaigiKeyboard.ImeConfig(
            context.packageName,
        )
    }

    /**
     * Adds a given [subtypeToAdd] to the subtype list, if it does not exist.
     *
     * @param subtypeToAdd The subtype which should be added.
     * @returns True if the subtype was added, false otherwise. A return value of false indicates
     *  that the subtype already exists.
     */
    private fun addSubtype(subtypeToAdd: Subtype): Boolean {
        val subtypeList = subtypes.toMutableList()
        if (subtypeList.contains(subtypeToAdd)) {
            return false
        }
        subtypeList.add(subtypeToAdd)
        subtypes = subtypeList
        return true
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

    /**
     * Gets a subtype by the given [id].
     *
     * @param id The id of the subtype you want to get.
     * @returns The subtype or null, if no matching subtype could be found.
     */
    fun getSubtypeById(id: Int): Subtype? {
        for (subtype in subtypes) {
            if (subtype.id == id) {
                return subtype
            }
        }
        return null
    }

    /**
     * Removes a given [subtypeToRemove]. Nothing happens if the given [subtypeToRemove] does not
     * exist.
     *
     * @param subtypeToRemove The subtype which should be removed.
     */
    fun removeSubtype(subtypeToRemove: Subtype) {
        val subtypeList = subtypes.toMutableList()
        for (subtype in subtypeList) {
            if (subtype == subtypeToRemove) {
                subtypeList.remove(subtypeToRemove)
                break
            }
        }
        subtypes = subtypeList
        if (subtypeToRemove.id == prefs.activeSubtypeId) {
            getActiveSubtype()
        }
    }

    /**
     * Switch to the next subtype in the subtype list if possible.
     *
     * @returns The new active subtype or null if the determination process failed.
     */
    fun switchToNextSubtype(): Subtype? {
        val subtypeList = subtypes
        val activeSubtype = getActiveSubtype() ?: return null
        var triggerNextSubtype = false
        var newActiveSubtype: Subtype? = null
        for (subtype in subtypeList) {
            if (triggerNextSubtype) {
                triggerNextSubtype = false
                newActiveSubtype = subtype
            } else if (subtype == activeSubtype) {
                triggerNextSubtype = true
            }
        }
        if (triggerNextSubtype) {
            newActiveSubtype = subtypeList[0]
        }
        prefs.activeSubtypeId =
            when (newActiveSubtype) {
                null -> -1
                else -> newActiveSubtype.id
            }
        return newActiveSubtype
    }
}
