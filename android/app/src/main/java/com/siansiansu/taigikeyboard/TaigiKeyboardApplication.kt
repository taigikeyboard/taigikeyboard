package com.siansiansu.taigikeyboard

import android.app.Application
import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionaryConstants
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import java.io.File
import java.io.FileOutputStream

// Application root — owns the process-wide service graph + prefs.
class TaigiKeyboardApplication : Application() {
    lateinit var prefs: PrefHelper
        private set

    lateinit var compositionRoot: CompositionRoot
        private set

    // Process-lifetime scope — never cancelled. Used only for fire-and-forget
    // work that must NOT be tied to an IME service lifecycle (prefs migration
    // survives cold IME starts; custom-dict seed runs at most once).
    val applicationScope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate() {
        super.onCreate()

        prefs = PrefHelper(this)
        prefs.warmUp()
        compositionRoot = CompositionRoot.shared(this)

        // D9.2 — register the Rust shared-core logger sink so any
        // `log::warn!` etc. from `librust_taigi.so` reaches the same
        // `LoggerBackend` the rest of the app uses.
        RustEngineBridge.install(compositionRoot.logger)

        // Deferred boot work — kept off the Application.onCreate main thread
        // per the 2026-04 Android state audit §A7 (keep Application.onCreate cheap).
        // Services are idempotent, so a duplicate call from a legacy caller
        // would be harmless during the A7 migration window.
        applicationScope.launch {
            prefs.migrateFromSharedPreferences()
        }
        applicationScope.launch {
            compositionRoot.customDict.seedDefaultEntryIfEmpty()
        }
        // Copy bundled assets to filesDir then install the Rust shared-core
        // lexicon engine. Idempotent.
        applicationScope.launch {
            installLexiconEngine()
        }
        // Best-effort warmup of `user_frequency.db` so the Continuous-input
        // fetch path can apply persisted boost as early as possible. The
        // Continuous fetch path never lazy-inits the freq DB itself, so
        // without this warmup a fresh session would ignore
        // `user_frequency.db` indefinitely until the user committed
        // something. Fire-and-forget — `ComposingManager
        // .fetchContinuousCandidates` keeps an `isConnected()` cold-start
        // guard for the race window before this Task lands. Mirrors iOS
        // `setupCoreServices`'s `ensureInitialized` Task.
        applicationScope.launch {
            val tag = "UserFreqWarmup"
            try {
                compositionRoot.userFreq.ensureInitialized()
                compositionRoot.logger.i(tag, "[INIT] User frequency DB warmed")
            } catch (e: Exception) {
                compositionRoot.logger.w(tag, "[INIT] User frequency DB warmup failed", e)
            }
        }
    }

    /**
     * Copy bundled `dictionary.fst`, `dictionary.bin`, `association.bin`
     * and (v3.5.8 Phase 2) `syllables.fst` from `assets/` to `filesDir/`
     * (mmap requires a real file handle — APK-internal asset entries are
     * not directly mmap-able), then call `LexiconBridge.install(...)` once
     * with absolute paths.
     *
     * Reinstall on app version bump: re-copies the assets and triggers
     * an atomic-on-success swap inside the engine (D-9 in the audit).
     */
    private fun installLexiconEngine() {
        val versionFile = File(filesDir, "dictionary_app_version.txt")
        val currentVersion = BuildConfig.VERSION_CODE
        val lastCopiedVersion = if (versionFile.exists()) {
            versionFile.readText().trim().toIntOrNull() ?: 0
        } else {
            0
        }
        val needsCopy = currentVersion > lastCopiedVersion

        val filesToCopy = listOf(
            "dictionary.fst",
            DictionaryConstants.DICT_BIN_NAME,
            DictionaryConstants.ASSOC_BIN_NAME,
            "syllables.fst",
        )
        for (fileName in filesToCopy) {
            val destFile = File(filesDir, fileName)
            if (needsCopy || !destFile.exists()) {
                try {
                    assets.open(fileName).use { input ->
                        FileOutputStream(destFile).use { output ->
                            input.copyTo(output)
                        }
                    }
                } catch (e: Exception) {
                    compositionRoot.logger.e("LexiconInstall", "[COPY] $fileName failed", e)
                    compositionRoot.lexiconReady.completeExceptionally(e)
                    return
                }
            }
        }
        if (needsCopy) {
            versionFile.writeText(currentVersion.toString())
        }

        val stats = LexiconBridge.install(
            triePath = File(filesDir, "dictionary.fst").absolutePath,
            dictionaryBinPath = File(filesDir, DictionaryConstants.DICT_BIN_NAME).absolutePath,
            associationBinPath = File(filesDir, DictionaryConstants.ASSOC_BIN_NAME).absolutePath,
            dictionaryVersion = currentVersion.toUInt(),
            syllableInventoryPath = File(filesDir, "syllables.fst").absolutePath,
        )
        if (stats != null) {
            compositionRoot.logger.i(
                "LexiconInstall",
                "[INSTALL] dict=${stats.dictionaryRecordCount} fst=${stats.prefixIndexEntryCount} v=$currentVersion",
            )
            compositionRoot.lexiconReady.complete(Unit)
        } else {
            compositionRoot.logger.w("LexiconInstall", "[INSTALL] returned null")
            compositionRoot.lexiconReady.completeExceptionally(
                IllegalStateException("LexiconBridge.install returned null"),
            )
        }
    }
}
