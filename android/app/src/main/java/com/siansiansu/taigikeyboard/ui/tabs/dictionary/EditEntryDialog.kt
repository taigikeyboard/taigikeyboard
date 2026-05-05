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
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.DictionaryTexts
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
                if (isEditing) DictionaryTexts.editEntry else DictionaryTexts.addEntry,
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = roman,
                    onValueChange = { roman = it },
                    label = { Text(DictionaryTexts.romanLabel) },
                    placeholder = { Text(DictionaryTexts.romanPlaceholder) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = hanzi,
                    onValueChange = { hanzi = it },
                    label = { Text(DictionaryTexts.hanziLabel) },
                    placeholder = { Text(DictionaryTexts.hanziPlaceholder) },
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
                Text(DictionaryTexts.save)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(CommonTexts.cancel)
            }
        },
    )
}
