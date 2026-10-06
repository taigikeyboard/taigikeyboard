package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.ResultDialog
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.resultMessage
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

// Learning Records — one row per kind (word frequency, learned phrases), each opening that kind's list,
// then Delete Learning Records: every learning store at once (next-word association included), custom words kept.
// [onClearAll] throws for the screen to report.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LearningRecordsMenuScreen(
    onNavigateBack: () -> Unit,
    onKind: (LearningRecordKind) -> Unit,
    onClearAll: suspend () -> Unit,
) {
    val scope = rememberCoroutineScope()
    // Read after the clear returns; tracks the live display language.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)
    var showClearDialog by remember { mutableStateOf(false) }
    // While the clear runs, a list opened now could read the rows it is emptying: the kind rows wait too.
    var isClearing by remember { mutableStateOf(false) }
    var clearResult by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = L10n.dictionaryLearningRecords,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = L10n.commonBack)
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp),
        ) {
            Spacer(Modifier.height(8.dp))
            SettingsCard {
                ActionRow(
                    label = kindLabel(LearningRecordKind.LEARNING_RECORD_KIND_FREQUENCY),
                    onClick = { if (!isClearing) onKind(LearningRecordKind.LEARNING_RECORD_KIND_FREQUENCY) },
                    trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                )
                SettingsDivider()
                ActionRow(
                    label = kindLabel(LearningRecordKind.LEARNING_RECORD_KIND_LEARNED_PHRASE),
                    onClick = { if (!isClearing) onKind(LearningRecordKind.LEARNING_RECORD_KIND_LEARNED_PHRASE) },
                    trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                )
            }
            Spacer(Modifier.height(16.dp))
            SettingsCard {
                ActionRow(
                    label = L10n.dictionaryClearLearningRecords,
                    onClick = { if (!isClearing) showClearDialog = true },
                    textColor = MaterialTheme.colorScheme.error,
                )
            }
        }
    }

    if (showClearDialog) {
        ConfirmationDialog(
            title = L10n.dictionaryClearLearningRecords,
            message = L10n.dictionaryClearLearningRecordsMessage,
            confirmLabel = L10n.commonDelete,
            dismissLabel = L10n.commonCancel,
            onConfirm = {
                showClearDialog = false
                isClearing = true
                scope.launch {
                    clearResult =
                        try {
                            onClearAll()
                            stringResolver.resolve(StringKey.DICTIONARY_CLEAR_LEARNING_RECORDS_DONE)
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            resultMessage(stringResolver.resolve(StringKey.DICTIONARY_CLEAR_LEARNING_RECORDS_FAILED), e.message)
                        } finally {
                            isClearing = false
                        }
                }
            },
            onDismiss = { showClearDialog = false },
        )
    }

    clearResult?.let { message ->
        ResultDialog(
            message = message,
            confirmLabel = L10n.commonOk,
            onDismiss = { clearResult = null },
        )
    }
}
