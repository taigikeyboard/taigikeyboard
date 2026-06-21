// Resolves i18n keys for the active display language + the Compose provider that drives live-switch.

package com.siansiansu.taigikeyboard.i18n

import android.content.Context
import android.content.res.Configuration
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.generated.GeneratedPseudoStrings
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
            StringResolution.Pseudo -> GeneratedPseudoStrings.lookup(key) ?: hanjiContext.getString(key.resId)
        }

    /**
     * Locale used by [formatString] when interpolating numeric format args. Native languages use
     * their own locale; the no-OS-locale paths (TL/POJ) and the debug Pseudo probe fall back to the
     * Hanji locale. `%d` carries no grouping, so this is locale-stable for current counts, but it keeps
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
// base-Context change (remember(base, language)), so nothing goes stale across a configuration change.
private fun Context.localizedFor(bcp47: String): Context {
    val config = Configuration(resources.configuration)
    config.setLocale(Locale.forLanguageTag(bcp47))
    return createConfigurationContext(config)
}

/** Builds a [StringResolver] for [language] from a base Context (the Native locale + the Hanji fallback). */
fun buildStringResolver(
    base: Context,
    language: DisplayLanguage,
): StringResolver {
    val hanjiContext = base.localizedFor(BCP47_HANJI)
    val activeContext =
        when (val resolution = language.resolution) {
            is StringResolution.Native -> base.localizedFor(resolution.bcp47)
            else -> hanjiContext
        }
    return StringResolver(hanjiContext, activeContext, language)
}

/**
 * Non-Compose resolver for the currently-persisted display language — the counterpart to [stringRes]
 * for class-load data sources and Activity callbacks resolving outside a Composition. Reads the warmed
 * PrefHelper cache synchronously, so it reflects the language selected at call time.
 */
fun Context.currentStringResolver(): StringResolver =
    buildStringResolver(this, DisplayLanguage.fromTag(PrefHelper(this).displayLanguageTag))

/** The active display language, so debug/probe UI can read the current selection. */
val LocalDisplayLanguage = staticCompositionLocalOf { DisplayLanguage.HANJI }

val LocalStringResolver =
    staticCompositionLocalOf<StringResolver> {
        error("LocalStringResolver not provided — wrap content in ProvideDisplayLanguage { }")
    }

/**
 * Provides [language]'s resolver to the Compose tree. Changing [language] rebuilds the resolver and
 * recomposes every [stringRes] consumer with no Activity/IME recreate (plan D7 live-switch).
 */
@Composable
fun ProvideDisplayLanguage(
    language: DisplayLanguage,
    content: @Composable () -> Unit,
) {
    val base = LocalContext.current
    val resolver = remember(base, language) { buildStringResolver(base, language) }
    CompositionLocalProvider(
        LocalDisplayLanguage provides language,
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
