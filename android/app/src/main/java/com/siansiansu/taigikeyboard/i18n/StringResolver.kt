// Resolves i18n keys for the active display language + the Compose provider that drives live-switch.

package com.siansiansu.taigikeyboard.i18n

import android.app.LocaleManager
import android.content.Context
import android.content.res.Configuration
import android.content.res.Resources
import android.os.Build
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalContext
import androidx.core.os.ConfigurationCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.generated.GeneratedTaigiStrings
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import java.util.Locale

/**
 * Resolves a [StringKey] to a display string for one [DisplayLanguage].
 *
 * Non-Compose by design so class-load-time call sites (data sources, static option lists) can
 * resolve without a Composition; the Compose layer ([stringRes]) is only a thin reader.
 *
 * [hanjiContext] is pinned to [BCP47_HANJI] so it always yields the default
 * (Hanji) resource set — the explicit anti-crash fallback when a generated map lacks a key.
 */
class StringResolver(
    private val hanjiContext: Context,
    private val activeContext: Context,
    private val language: DisplayLanguage,
) {
    /** The active display language, so plural-aware generated accessors can pick a count-based arm. */
    val displayLanguage: DisplayLanguage get() = language

    fun resolve(key: StringKey): String =
        when (language.resolution) {
            is StringResolution.Native -> activeContext.getString(key.resId)
            StringResolution.GeneratedMap -> GeneratedTaigiStrings.lookup(language, key) ?: hanjiContext.getString(key.resId)
            // SYSTEM/Automatic is resolved to an effective language in buildStringResolver before the
            // resolver is constructed, so this branch is unreachable — fail fast if it ever isn't.
            StringResolution.Automatic -> error("Automatic must be resolved to an effective language before the resolver")
        }

    /**
     * Locale used by [formatString] when interpolating numeric format args. Native languages use
     * their own locale; the no-OS-locale paths (TL/POJ) fall back to the Hanji locale. `%d` carries no
     * grouping, so this is locale-stable for current counts, but it keeps
     * the formatter honest if a grouped/`%,d` spec is ever authored.
     */
    val formattingLocale: Locale =
        when (val resolution = language.resolution) {
            is StringResolution.Native -> Locale.forLanguageTag(resolution.bcp47)
            else -> Locale.forLanguageTag(BCP47_HANJI)
        }
}

/**
 * Resolves a format key's template under the active language and interpolates [args]. Backs the
 * generated typed accessors in `StringResolverFormats.kt`, so call sites never touch a raw `%1$d`
 * template (plan D6 — forbid raw `%d` at call sites).
 */
fun StringResolver.formatString(
    key: StringKey,
    vararg args: Any,
): String = String.format(formattingLocale, resolve(key), *args)

/**
 * Interpolates a ready-made positional [template] under the active formatting locale. Backs the
 * plural-aware generated accessors, which select each count's plural arm at runtime. The codegen
 * emits flat string resources for every language and the TL/POJ display languages have no OS plural
 * locale at all, so one runtime arm-selector serves all five languages — a native `<plurals>` would
 * be a second, English-only mechanism (plan R3-2).
 */
fun StringResolver.formatTemplate(
    template: String,
    vararg args: Any,
): String = String.format(formattingLocale, template, *args)

// Copies the base configuration and overrides only the locale (mirrors florisboard
// FlorisAppActivity.kt:100). The returned context is used solely for getString(), which depends
// only on the locale qualifier — copying other qualifiers (fontScale / uiMode) is harmless because
// the strings have no such variants, and ProvideDisplayLanguage rebuilds the resolver on every
// base-Context / selected-language / device-locale change (remember(base, language, deviceSubtag)), so
// nothing goes stale across a configuration change.
private fun Context.localizedFor(bcp47: String): Context {
    val config = Configuration(resources.configuration)
    config.setLocale(Locale.forLanguageTag(bcp47))
    return createConfigurationContext(config)
}

// The DEVICE locale (immune to any app-level locale override), used only to resolve SYSTEM/Automatic.
// On API 33+ LocaleManager.getSystemLocales() ignores per-app locale overrides — unlike
// Locale.getDefault(); on <33 there is no per-app override, so the system Resources locale is the device
// locale. Returns the lowercased language subtag (e.g. "ja", "zh", "en"), or "" when no locale is present.
private fun deviceLanguageSubtag(context: Context): String {
    val locale =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.getSystemService(LocaleManager::class.java)?.systemLocales?.get(0)
        } else {
            null
        } ?: ConfigurationCompat.getLocales(Resources.getSystem().configuration).get(0)
    return locale?.language?.lowercase().orEmpty()
}

/**
 * Builds a [StringResolver] for [language] from a base Context (the Native locale + the Hanji fallback).
 * [DisplayLanguage.SYSTEM] is resolved to its effective authored language from the device OS locale
 * first, so the resolver holds a concrete language — the English-only plural arm fires when SYSTEM→ENGLISH.
 */
fun buildStringResolver(
    base: Context,
    language: DisplayLanguage,
): StringResolver {
    // Only SYSTEM's effective language depends on the device locale; reading it for an explicit
    // selection would fire a LocaleManager binder IPC for a subtag effectiveLanguage immediately discards.
    val effective =
        if (language == DisplayLanguage.SYSTEM) language.effectiveLanguage(deviceLanguageSubtag(base)) else language
    val hanjiContext = base.localizedFor(BCP47_HANJI)
    val activeContext =
        when (val resolution = effective.resolution) {
            is StringResolution.Native -> base.localizedFor(resolution.bcp47)
            else -> hanjiContext
        }
    return StringResolver(hanjiContext, activeContext, effective)
}

/**
 * Non-Compose resolver for the currently-persisted display language — the counterpart to [stringRes]
 * for class-load data sources and Activity callbacks resolving outside a Composition. Reads the warmed
 * PrefHelper cache synchronously, so it reflects the language selected at call time.
 */
fun Context.currentStringResolver(): StringResolver =
    buildStringResolver(this, DisplayLanguage.fromTag(PrefHelper(this).displayLanguageTag))

/**
 * The EFFECTIVE display language — the concrete authored language whose strings render now (SYSTEM is
 * already resolved away). Drives the resolver + the plural arm. For the picker's selected-state and
 * trailing label (which must show "system" when the user picked Automatic), use [LocalSelectedDisplayLanguage].
 */
val LocalDisplayLanguage = staticCompositionLocalOf { DisplayLanguage.HANJI }

/**
 * The SELECTED display language as the user picked it — may be [DisplayLanguage.SYSTEM]. Drives the
 * Settings language picker's selected-row checkmark and the settings-row trailing label, which must
 * reflect the persisted selection (Automatic), not the language SYSTEM currently resolves to.
 */
val LocalSelectedDisplayLanguage = staticCompositionLocalOf { DisplayLanguage.HANJI }

val LocalStringResolver =
    staticCompositionLocalOf<StringResolver> {
        error("LocalStringResolver not provided — wrap content in ProvideDisplayLanguage { }")
    }

/**
 * Provides [language]'s resolver to the Compose tree. Changing [language] rebuilds the resolver and
 * recomposes every [stringRes] consumer with no Activity/IME recreate (plan D7 live-switch).
 *
 * [LocalSelectedDisplayLanguage] carries [language] as-picked (may be [DisplayLanguage.SYSTEM]) for the
 * picker; [LocalDisplayLanguage] carries the effective language for the resolver. The device language
 * subtag is part of the memo key so an OS-locale change rebuilds the resolver even when the SELECTED
 * value (`system`) is unchanged — the SYSTEM live-refresh on the next recompose / config change.
 */
@Composable
fun ProvideDisplayLanguage(
    language: DisplayLanguage,
    content: @Composable () -> Unit,
) {
    val base = LocalContext.current
    // Only SYSTEM's effective language depends on the device locale, so only SYSTEM reads it (a
    // LocaleManager binder IPC) and keys the resolver memo on it — an explicit selection neither pays
    // the IPC nor rebuilds spuriously when the OS locale changes. The resolver already carries the
    // effective language, so reuse it for LocalDisplayLanguage instead of recomputing.
    val localeMemoKey = if (language == DisplayLanguage.SYSTEM) deviceLanguageSubtag(base) else ""
    val resolver = remember(base, language, localeMemoKey) { buildStringResolver(base, language) }
    CompositionLocalProvider(
        LocalSelectedDisplayLanguage provides language,
        LocalDisplayLanguage provides resolver.displayLanguage,
        LocalStringResolver provides resolver,
    ) {
        content()
    }
}

/**
 * Activity-root overload: subscribes to the persisted display-language tag and re-provides the
 * subtree on every change, so the whole screen live-switches with no Activity recreate (plan D7).
 * Standalone Activities call `ProvideDisplayLanguage(prefs) { TaigiKeyboardTheme { ... } }`.
 */
@Composable
fun ProvideDisplayLanguage(
    prefs: PrefHelper,
    content: @Composable () -> Unit,
) {
    val tag by prefs.observeDisplayLanguage().collectAsStateWithLifecycle(initialValue = prefs.displayLanguageTag)
    ProvideDisplayLanguage(DisplayLanguage.fromTag(tag), content)
}

/** Compose accessor: resolves [key] under the current [LocalStringResolver]. */
@Composable
fun stringRes(key: StringKey): String = LocalStringResolver.current.resolve(key)
