package com.siansiansu.taigikeyboard.ui.tabs.tab2

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import kotlin.math.roundToInt
import android.graphics.Color as AndroidColor

// Bottom-sheet color picker with grid, spectrum, and RGB slider tabs

private val DEFAULT_PICKER_COLOR = AndroidColor.rgb(213, 214, 221)
private const val SWATCHES_PER_ROW = 12
private const val SPECTRUM_BITMAP_WIDTH = 200
private const val SPECTRUM_BITMAP_HEIGHT = 360
private const val TAB_GRID = 0
private const val TAB_SPECTRUM = 1
private const val TAB_SLIDERS = 2

private val SELECTOR_OUTER_RADIUS = 10.dp
private val SELECTOR_OUTER_STROKE = 1.5.dp
private val SELECTOR_INNER_RADIUS = 8.5.dp
private val SELECTOR_INNER_STROKE = 2.dp

private val SLIDER_LABEL_WIDTH = 40.dp
private val SLIDER_HEIGHT = 36.dp
private val GRADIENT_TRACK_HEIGHT = 10.dp
private val THUMB_RADIUS = 11.dp
private const val THUMB_SHADOW_COLOR = 0x30000000
private const val THUMB_INNER_RATIO = 0.7f

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ColorPickerDialog(
    title: String,
    currentColor: Int?,
    onDismiss: () -> Unit,
    onColorSelected: (Int?) -> Unit,
) {
    val initialHsv =
        remember {
            floatArrayOf(0f, 0f, 0f).also { hsv ->
                AndroidColor.colorToHSV(currentColor ?: DEFAULT_PICKER_COLOR, hsv)
            }
        }
    var hue by remember { mutableFloatStateOf(initialHsv[0]) }
    var saturation by remember { mutableFloatStateOf(initialHsv[1]) }
    var brightness by remember { mutableFloatStateOf(initialHsv[2]) }
    var hexInput by remember {
        mutableStateOf(String.format("#%06X", 0xFFFFFF and (currentColor ?: DEFAULT_PICKER_COLOR)))
    }
    var selectedTab by remember { mutableIntStateOf(TAB_SPECTRUM) }

    fun currentColorInt(): Int = AndroidColor.HSVToColor(floatArrayOf(hue, saturation, brightness))

    fun updateFromColor(color: Int) {
        val hsv = floatArrayOf(0f, 0f, 0f)
        AndroidColor.colorToHSV(color, hsv)
        hue = hsv[0]
        saturation = hsv[1]
        brightness = hsv[2]
        hexInput = String.format("#%06X", 0xFFFFFF and color)
    }

    fun updateHex() {
        hexInput = String.format("#%06X", 0xFFFFFF and currentColorInt())
    }

    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        tonalElevation = 6.dp,
    ) {
        Column(modifier = Modifier.padding(horizontal = 24.dp).padding(bottom = 24.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = title,
                    fontSize = 18.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f),
                )
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Close",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier =
                        Modifier
                            .size(24.dp)
                            .clickable(onClick = onDismiss),
                )
            }

            Spacer(Modifier.height(16.dp))

            val tabLabels =
                listOf(
                    Tab2Texts.colorPickerGrid,
                    Tab2Texts.colorPickerSpectrum,
                    Tab2Texts.colorPickerSliders,
                )
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                tabLabels.forEachIndexed { index, label ->
                    SegmentedButton(
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        shape = SegmentedButtonDefaults.itemShape(index, tabLabels.size),
                        icon = {},
                    ) { Text(label, fontSize = 13.sp) }
                }
            }

            Spacer(Modifier.height(16.dp))

            Box(modifier = Modifier.fillMaxWidth().height(280.dp)) {
                when (selectedTab) {
                    TAB_GRID -> {
                        ColorGridContent(
                            onColorSelected = {
                                updateFromColor(it)
                                onColorSelected(currentColorInt())
                            },
                        )
                    }

                    TAB_SPECTRUM -> {
                        ColorSpectrumContent(
                            hue = hue,
                            saturation = saturation,
                            brightness = brightness,
                            onHsvChanged = { h, s, v ->
                                hue = h
                                saturation = s
                                brightness = v
                                updateHex()
                                onColorSelected(currentColorInt())
                            },
                        )
                    }

                    TAB_SLIDERS -> {
                        ColorSlidersContent(
                            colorInt = currentColorInt(),
                            onColorChanged = {
                                updateFromColor(it)
                                onColorSelected(currentColorInt())
                            },
                        )
                    }
                }
            }

            Spacer(Modifier.height(16.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier =
                        Modifier
                            .size(40.dp)
                            .clip(CircleShape)
                            .background(Color(currentColorInt()))
                            .border(1.dp, Color.Gray, CircleShape),
                )
                Spacer(Modifier.width(12.dp))
                OutlinedTextField(
                    value = hexInput,
                    onValueChange = { input ->
                        hexInput = input
                        val hex = input.trim().removePrefix("#")
                        if (hex.length == 6 && hex.all { it.digitToIntOrNull(16) != null }) {
                            val parsed = AndroidColor.parseColor("#$hex")
                            val hsv = floatArrayOf(0f, 0f, 0f)
                            AndroidColor.colorToHSV(parsed, hsv)
                            hue = hsv[0]
                            saturation = hsv[1]
                            brightness = hsv[2]
                            onColorSelected(currentColorInt())
                        }
                    },
                    label = { Text("#RRGGBB") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Ascii),
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

private val presetColors: List<Int> =
    buildList {
        val hues = listOf(195f, 221f, 247f, 273f, 299f, 325f, 351f, 17f, 43f, 69f, 95f, 121f)
        // Row 1: Grayscale
        for (i in 0..11) {
            val v = 255 - (i * 255 / 11)
            add(AndroidColor.rgb(v, v, v))
        }
        // Rows 2-9: 12 hues x 8 lightness levels (light to dark)
        val levels =
            listOf(
                1.00f to 0.40f,
                1.00f to 0.60f,
                1.00f to 0.80f,
                1.00f to 1.00f,
                0.75f to 1.00f,
                0.50f to 1.00f,
                0.30f to 1.00f,
                0.15f to 1.00f,
            )
        for ((s, v) in levels) {
            for (h in hues) {
                add(AndroidColor.HSVToColor(floatArrayOf(h, s, v)))
            }
        }
    }

@Composable
private fun ColorGridContent(onColorSelected: (Int) -> Unit) {
    Column(modifier = Modifier.fillMaxSize()) {
        presetColors.chunked(SWATCHES_PER_ROW).forEach { row ->
            Row(modifier = Modifier.fillMaxWidth().weight(1f)) {
                row.forEach { color ->
                    Box(
                        modifier =
                            Modifier
                                .weight(1f)
                                .fillMaxSize()
                                .background(Color(color))
                                .clickable { onColorSelected(color) },
                    )
                }
            }
        }
    }
}

@Composable
private fun ColorSpectrumContent(
    hue: Float,
    saturation: Float,
    brightness: Float,
    onHsvChanged: (h: Float, s: Float, v: Float) -> Unit,
) {
    val spectrumBitmap =
        remember {
            val w = SPECTRUM_BITMAP_WIDTH
            val h = SPECTRUM_BITMAP_HEIGHT
            val bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
            val hsv = floatArrayOf(0f, 0f, 0f)
            for (x in 0 until w) {
                val ratio = x.toFloat() / (w - 1)
                // Left half: increasing saturation at full brightness
                // Right half: full saturation with decreasing brightness
                if (ratio <= 0.5f) {
                    hsv[1] = ratio * 2f
                    hsv[2] = 1f
                } else {
                    hsv[1] = 1f
                    hsv[2] = 1f - (ratio - 0.5f) * 2f
                }
                for (y in 0 until h) {
                    hsv[0] = y.toFloat() / (h - 1) * 360f
                    bitmap.setPixel(x, y, AndroidColor.HSVToColor(hsv))
                }
            }
            bitmap.asImageBitmap()
        }

    val selectorNormX =
        if (brightness >= 0.99f) {
            saturation * 0.5f
        } else {
            0.5f + (1f - brightness) * 0.5f
        }
    val selectorNormY = hue / 360f

    Canvas(
        modifier =
            Modifier
                .fillMaxSize()
                .clip(RoundedCornerShape(8.dp))
                .pointerInput(Unit) {
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        val update = { offset: Offset ->
                            val nx = (offset.x / size.width).coerceIn(0f, 1f)
                            val ny = (offset.y / size.height).coerceIn(0f, 1f)
                            val h = ny * 360f
                            val s: Float
                            val v: Float
                            if (nx <= 0.5f) {
                                s = nx * 2f
                                v = 1f
                            } else {
                                s = 1f
                                v = 1f - (nx - 0.5f) * 2f
                            }
                            onHsvChanged(h, s, v)
                        }
                        update(down.position)
                        down.consume()
                        do {
                            val event = awaitPointerEvent()
                            event.changes.forEach {
                                it.consume()
                                update(it.position)
                            }
                        } while (event.changes.any { it.pressed })
                    }
                },
    ) {
        drawImage(
            image = spectrumBitmap,
            srcOffset = IntOffset.Zero,
            srcSize = IntSize(spectrumBitmap.width, spectrumBitmap.height),
            dstOffset = IntOffset.Zero,
            dstSize = IntSize(size.width.roundToInt(), size.height.roundToInt()),
        )
        val sx = selectorNormX * size.width
        val sy = selectorNormY * size.height
        drawCircle(Color.Black, SELECTOR_OUTER_RADIUS.toPx(), Offset(sx, sy), style = Stroke(SELECTOR_OUTER_STROKE.toPx()))
        drawCircle(Color.White, SELECTOR_INNER_RADIUS.toPx(), Offset(sx, sy), style = Stroke(SELECTOR_INNER_STROKE.toPx()))
    }
}

@Composable
private fun ColorSlidersContent(
    colorInt: Int,
    onColorChanged: (Int) -> Unit,
) {
    val r = (colorInt shr 16) and 0xFF
    val g = (colorInt shr 8) and 0xFF
    val b = colorInt and 0xFF

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        GradientColorSlider(
            label = Tab2Texts.colorRed,
            value = r / 255f,
            trackColors =
                listOf(
                    Color(AndroidColor.rgb(0, g, b)),
                    Color(AndroidColor.rgb(255, g, b)),
                ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(AndroidColor.rgb((it * 255).toInt(), g, b)) },
        )
        GradientColorSlider(
            label = Tab2Texts.colorGreen,
            value = g / 255f,
            trackColors =
                listOf(
                    Color(AndroidColor.rgb(r, 0, b)),
                    Color(AndroidColor.rgb(r, 255, b)),
                ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(AndroidColor.rgb(r, (it * 255).toInt(), b)) },
        )
        GradientColorSlider(
            label = Tab2Texts.colorBlue,
            value = b / 255f,
            trackColors =
                listOf(
                    Color(AndroidColor.rgb(r, g, 0)),
                    Color(AndroidColor.rgb(r, g, 255)),
                ),
            thumbColor = Color(colorInt),
            onValueChange = { onColorChanged(AndroidColor.rgb(r, g, (it * 255).toInt())) },
        )
    }
}

@Composable
private fun GradientColorSlider(
    label: String,
    value: Float,
    trackColors: List<Color>,
    thumbColor: Color,
    onValueChange: (Float) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            fontSize = 15.sp,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.width(SLIDER_LABEL_WIDTH),
        )
        Canvas(
            modifier =
                Modifier
                    .weight(1f)
                    .height(SLIDER_HEIGHT)
                    .pointerInput(Unit) {
                        awaitEachGesture {
                            val down = awaitFirstDown()
                            val update = { offset: Offset ->
                                onValueChange((offset.x / size.width).coerceIn(0f, 1f))
                            }
                            update(down.position)
                            down.consume()
                            do {
                                val event = awaitPointerEvent()
                                event.changes.forEach {
                                    it.consume()
                                    update(it.position)
                                }
                            } while (event.changes.any { it.pressed })
                        }
                    },
        ) {
            val trackHeight = GRADIENT_TRACK_HEIGHT.toPx()
            val trackY = (size.height - trackHeight) / 2f
            val thumbRadius = THUMB_RADIUS.toPx()

            drawRoundRect(
                brush = Brush.horizontalGradient(trackColors),
                topLeft = Offset(0f, trackY),
                size = Size(size.width, trackHeight),
                cornerRadius = CornerRadius(trackHeight / 2f),
            )

            val thumbX = value * size.width
            val thumbCenter = Offset(thumbX, size.height / 2f)
            drawCircle(Color(THUMB_SHADOW_COLOR), thumbRadius + 2.dp.toPx(), thumbCenter)
            drawCircle(Color.White, thumbRadius, thumbCenter)
            drawCircle(thumbColor, thumbRadius * THUMB_INNER_RATIO, thumbCenter)
        }
        Text(
            text = "${(value * 255).toInt()}",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.width(36.dp),
            textAlign = TextAlign.End,
            style = MaterialTheme.typography.labelLarge,
        )
    }
}
