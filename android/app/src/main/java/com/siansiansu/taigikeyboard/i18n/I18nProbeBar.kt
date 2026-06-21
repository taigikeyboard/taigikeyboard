// Debug-only language probe bar — proves live-switch across both D2 resolution paths (R2a-1).
//
// Renders only in debug builds. Switching HANJI / ENGLISH (native resource) <-> PSEUDO (generated
// map) recomposes the probe label live, exercising both halves of the hybrid with one tap. ENGLISH
// exercises the real authored English (P2 R3-1); PSEUDO is the layout-inflated probe, neutralized to
// HANJI in release (DisplayLanguage.fromTag) so a leftover probe selection can never surface it in
// production. ENGLISH is safe in release too, but the production picker that exposes it lands in R3-3.
// Removed once the picker lands (P2).

package com.siansiansu.taigikeyboard.i18n

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

private val PROBE_LANGUAGES = listOf(DisplayLanguage.HANJI, DisplayLanguage.ENGLISH, DisplayLanguage.PSEUDO)

@Composable
fun I18nProbeBar(prefs: PrefHelper) {
    if (!BuildConfig.DEBUG) return
    val current = LocalDisplayLanguage.current
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
    ) {
        Text(
            text = "i18n PROBE · ${L10n.probeAppHeaderTitle} · ${L10n.probeLiveSwitchDemo}",
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = FontWeight.SemiBold,
        )
        Row {
            PROBE_LANGUAGES.forEach { language ->
                TextButton(onClick = { prefs.displayLanguageTag = language.tag }) {
                    Text(if (language == current) "● ${language.tag}" else language.tag)
                }
            }
        }
    }
}
