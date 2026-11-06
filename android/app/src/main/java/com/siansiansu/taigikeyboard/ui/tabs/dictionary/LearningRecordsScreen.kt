package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.engine.proto.LearningRecord
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordOrder
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ui.components.EmptyListCard
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.LoadingRow
import com.siansiansu.taigikeyboard.ui.components.NoticeCard
import com.siansiansu.taigikeyboard.ui.components.ResultDialog
import com.siansiansu.taigikeyboard.ui.components.RetryCard
import com.siansiansu.taigikeyboard.ui.components.SegmentedChoiceRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.resultMessage

/** The orders the picker offers; the first is the default. */
private val ORDERS =
    listOf(
        LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_USED,
        LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_RECENT,
    )

/** Load the next page once the last visible item is this close to the end of the list. */
private const val LOAD_MORE_THRESHOLD = 10

// Learning Records — what the keyboard learned: correct one row's count, delete one row, add a
// row's word to the custom dictionary
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LearningRecordsScreen(
    viewModel: LearningRecordsViewModel,
    onNavigateBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    var editingRecord by remember { mutableStateOf<LearningRecord?>(null) }

    // Back on the page after typing elsewhere: what the keyboard learned meanwhile is listed. The
    // first resume is skipped — the view model's init already loads.
    var hasResumed by remember { mutableStateOf(false) }
    LifecycleResumeEffect(viewModel) {
        if (hasResumed) viewModel.refresh() else hasResumed = true
        onPauseOrDispose {}
    }

    val listState = rememberLazyListState()
    val isNearEnd by remember {
        derivedStateOf {
            val layout = listState.layoutInfo
            val lastVisible = layout.visibleItemsInfo.lastOrNull()?.index ?: return@derivedStateOf false
            lastVisible >= layout.totalItemsCount - LOAD_MORE_THRESHOLD
        }
    }
    // Re-evaluated after every load and query change, so staying at the end still pages on; a
    // failed page waits for the retry row.
    LaunchedEffect(isNearEnd, state.isLoading, state.hasReadFailed, state.records.size, state.order, state.filter) {
        if (isNearEnd && !state.isLoading && !state.hasReadFailed) viewModel.loadMore()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = kindLabel(viewModel.kind),
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
                    .padding(padding),
        ) {
            LazyColumn(
                state = listState,
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(horizontal = 20.dp),
            ) {
                // Order
                item {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        text = L10n.dictionaryLearningRecordsOrder,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    SettingsCard {
                        SegmentedChoiceRow(
                            labels =
                                ORDERS.map {
                                    when (it) {
                                        LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_RECENT -> L10n.dictionaryLearningRecordsOrderMostRecent
                                        else -> L10n.dictionaryLearningRecordsOrderMostUsed
                                    }
                                },
                            selectedIndex = ORDERS.indexOf(state.order),
                            onSelect = { viewModel.selectOrder(ORDERS[it]) },
                            modifier = Modifier.padding(16.dp),
                        )
                    }
                    Spacer(Modifier.height(16.dp))
                }

                if (state.records.isEmpty()) {
                    item {
                        when {
                            state.isLoading -> SettingsCard { LoadingRow() }
                            // A failed read claims neither "nothing learned yet" nor "no results".
                            state.hasReadFailed -> RetryCard(text = L10n.dictionaryLearningRecordsReadFailed, onRetry = viewModel::retry)
                            state.total == 0 -> EmptyListCard(L10n.dictionaryLearningRecordsEmpty)
                            else -> NoticeCard(L10n.dictionaryNoResults)
                        }
                    }
                } else {
                    itemsIndexed(
                        items = state.records,
                        key = { _, record -> record.id },
                    ) { index, record ->
                        RecordRow(
                            record = record,
                            onEdit = { editingRecord = record },
                            onDelete = { viewModel.delete(record) },
                            onAddToCustomDictionary =
                                if (record.canAddToCustomDictionary) {
                                    { viewModel.addToCustomDictionary(record) }
                                } else {
                                    null
                                },
                        )
                        if (index < state.records.lastIndex) {
                            HorizontalDivider(
                                modifier = Modifier.padding(horizontal = 20.dp),
                                color = MaterialTheme.colorScheme.outlineVariant,
                            )
                        }
                    }
                    when {
                        state.isLoading -> item { LoadingRow() }
                        state.hasReadFailed -> item { RetryCard(text = L10n.dictionaryLearningRecordsReadFailed, onRetry = viewModel::retry) }
                    }
                }

                item { Spacer(Modifier.height(40.dp)) }
            }

            // Filter (anchored at bottom) — the engine filters, once typing settles
            FilterSearchBar(
                value = state.filter,
                onValueChange = viewModel::updateFilter,
                placeholder = L10n.dictionarySearchPlaceholder,
            )
        }
    }

    editingRecord?.let { record ->
        EditCountDialog(
            record = record,
            onDismiss = { editingRecord = null },
            onSave = { count ->
                viewModel.setCount(record, count)
                editingRecord = null
            },
        )
    }

    state.message?.let { message ->
        ResultDialog(
            message = messageText(message),
            confirmLabel = L10n.commonOk,
            onDismiss = viewModel::dismissMessage,
        )
    }
}

// The kind's row on the Learning Records page and its list's title.
@Composable
internal fun kindLabel(kind: LearningRecordKind): String =
    when (kind) {
        LearningRecordKind.LEARNING_RECORD_KIND_LEARNED_PHRASE -> L10n.dictionaryLearningRecordsPhrases
        else -> L10n.dictionaryLearningRecordsFrequency
    }

@Composable
private fun messageText(message: LearningRecordsMessage): String =
    when (message) {
        LearningRecordsMessage.Gone -> L10n.dictionaryLearningRecordGone
        LearningRecordsMessage.AddedToCustomDictionary -> L10n.dictionaryLearningRecordsAddedToCustomDictionary
        is LearningRecordsMessage.ReadFailed -> resultMessage(L10n.dictionaryLearningRecordsReadFailed, message.detail)
        is LearningRecordsMessage.WriteFailed -> resultMessage(L10n.dictionaryLearningRecordsWriteFailed, message.detail)
    }

@Composable
private fun RecordRow(
    record: LearningRecord,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    /** Shown only when non-null: the engine says the row's word can be added (`canAddToCustomDictionary`). */
    onAddToCustomDictionary: (() -> Unit)? = null,
) {
    val lastUsed = learningRecordLastUsedLabel(record.lastUsedMs)
    val usage =
        buildString {
            append("${L10n.dictionaryLearningRecordsCount} ${record.count}")
            if (lastUsed.isNotEmpty()) append(" · ${L10n.dictionaryLearningRecordsLastUsed} $lastUsed")
        }
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface)
                .heightIn(min = 48.dp)
                .padding(start = 20.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .clickable(onClick = onEdit)
                    .padding(vertical = 12.dp),
        ) {
            Text(
                text = record.text,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )
            if (record.tl.isNotEmpty()) {
                Text(
                    text = record.tl,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
            Text(
                text = usage,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
            )
        }
        onAddToCustomDictionary?.let { onAdd ->
            IconButton(onClick = onAdd) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = L10n.dictionaryLearningRecordsAddToCustomDictionary,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        IconButton(onClick = onDelete) {
            Icon(
                imageVector = Icons.Default.Delete,
                contentDescription = L10n.commonDelete,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// Edit one row's count: a whole number from 1.
@Composable
private fun EditCountDialog(
    record: LearningRecord,
    onDismiss: () -> Unit,
    onSave: (Long) -> Unit,
) {
    var countText by remember(record) { mutableStateOf(record.count.toString()) }
    val count = countText.toLongOrNull()?.takeIf { it in 1..MAX_LEARNING_RECORD_COUNT }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(L10n.dictionaryLearningRecordsEditCount) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Column {
                    Text(record.text, style = MaterialTheme.typography.bodyLarge)
                    if (record.tl.isNotEmpty()) {
                        Text(
                            text = record.tl,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                }
                OutlinedTextField(
                    value = countText,
                    onValueChange = { typed -> countText = typed.filter(Char::isDigit).take(MAX_LEARNING_RECORD_COUNT.toString().length) },
                    label = { Text(L10n.dictionaryLearningRecordsCount) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { count?.let(onSave) },
                enabled = count != null,
            ) {
                Text(L10n.commonSave)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(L10n.commonCancel)
            }
        },
    )
}
