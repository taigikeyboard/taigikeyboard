package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import java.util.UUID

// Dialog for adding or editing a custom dictionary entry
@Composable
internal fun EditEntryDialog(
    entry: CustomDictionaryService.Entry?,
    onDismiss: () -> Unit,
    onSave: (CustomDictionaryService.Entry) -> Unit,
) {
    var roman by remember(entry) { mutableStateOf(entry?.roman ?: "") }
    var hanzi by remember(entry) { mutableStateOf(entry?.hanzi ?: "") }
    val isEditing = entry != null
    val canSave = roman.isNotBlank() && hanzi.isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                if (isEditing) L10n.dictionaryEditEntry else L10n.dictionaryAddEntry,
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = roman,
                    onValueChange = { roman = it },
                    label = { Text(L10n.dictionaryRomanLabel) },
                    placeholder = { Text(L10n.dictionaryRomanPlaceholder) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = hanzi,
                    onValueChange = { hanzi = it },
                    label = { Text(L10n.dictionaryHanziLabel) },
                    placeholder = { Text(L10n.dictionaryHanziPlaceholder) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val saved =
                        CustomDictionaryService.Entry(
                            id = entry?.id ?: UUID.randomUUID().toString(),
                            roman = roman.trim(),
                            hanzi = hanzi.trim(),
                        )
                    onSave(saved)
                },
                enabled = canSave,
            ) {
                Text(L10n.dictionarySave)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(L10n.commonCancel)
            }
        },
    )
}
