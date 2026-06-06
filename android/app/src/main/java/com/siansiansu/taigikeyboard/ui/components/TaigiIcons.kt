/*
 * Copyright 2025 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Icon definitions extracted from androidx.compose.material:material-icons-extended
// to eliminate the large transitive dependency. Path data is identical to the originals.
//
// Icons in this file (21 total):
//   Filled: EmojiEmotions, EmojiEvents, EmojiFlags, EmojiFoodBeverage, EmojiNature,
//           EmojiObjects, EmojiPeople, EmojiSymbols, EmojiTransportation
//   Outlined: ContentCopy, FileDownload, FileUpload, FormatSize, Language,
//             SpaceBar, Translate, Vibration, ViewStream
//   AutoMirrored.Outlined: MenuBook, OpenInNew, VolumeUp

package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.materialIcon
import androidx.compose.material.icons.materialPath
import androidx.compose.ui.graphics.vector.ImageVector

// =============================================================================
// Filled icons
// =============================================================================

public val Icons.Filled.EmojiEmotions: ImageVector
    get() {
        if (_emojiEmotions != null) {
            return _emojiEmotions!!
        }
        _emojiEmotions =
            materialIcon(name = "Filled.EmojiEmotions") {
                materialPath {
                    moveTo(11.99f, 2.0f)
                    curveTo(6.47f, 2.0f, 2.0f, 6.48f, 2.0f, 12.0f)
                    curveToRelative(0.0f, 5.52f, 4.47f, 10.0f, 9.99f, 10.0f)
                    curveTo(17.52f, 22.0f, 22.0f, 17.52f, 22.0f, 12.0f)
                    curveTo(22.0f, 6.48f, 17.52f, 2.0f, 11.99f, 2.0f)
                    close()
                    moveTo(8.5f, 8.0f)
                    curveTo(9.33f, 8.0f, 10.0f, 8.67f, 10.0f, 9.5f)
                    reflectiveCurveTo(9.33f, 11.0f, 8.5f, 11.0f)
                    reflectiveCurveTo(7.0f, 10.33f, 7.0f, 9.5f)
                    reflectiveCurveTo(7.67f, 8.0f, 8.5f, 8.0f)
                    close()
                    moveTo(12.0f, 18.0f)
                    curveToRelative(-2.28f, 0.0f, -4.22f, -1.66f, -5.0f, -4.0f)
                    horizontalLineToRelative(10.0f)
                    curveTo(16.22f, 16.34f, 14.28f, 18.0f, 12.0f, 18.0f)
                    close()
                    moveTo(15.5f, 11.0f)
                    curveToRelative(-0.83f, 0.0f, -1.5f, -0.67f, -1.5f, -1.5f)
                    reflectiveCurveTo(14.67f, 8.0f, 15.5f, 8.0f)
                    reflectiveCurveTo(17.0f, 8.67f, 17.0f, 9.5f)
                    reflectiveCurveTo(16.33f, 11.0f, 15.5f, 11.0f)
                    close()
                }
            }
        return _emojiEmotions!!
    }

private var _emojiEmotions: ImageVector? = null

public val Icons.Filled.EmojiEvents: ImageVector
    get() {
        if (_emojiEvents != null) {
            return _emojiEvents!!
        }
        _emojiEvents =
            materialIcon(name = "Filled.EmojiEvents") {
                materialPath {
                    moveTo(19.0f, 5.0f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineTo(3.0f)
                    horizontalLineTo(7.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineTo(5.0f)
                    curveTo(3.9f, 5.0f, 3.0f, 5.9f, 3.0f, 7.0f)
                    verticalLineToRelative(1.0f)
                    curveToRelative(0.0f, 2.55f, 1.92f, 4.63f, 4.39f, 4.94f)
                    curveToRelative(0.63f, 1.5f, 1.98f, 2.63f, 3.61f, 2.96f)
                    verticalLineTo(19.0f)
                    horizontalLineTo(7.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(10.0f)
                    verticalLineToRelative(-2.0f)
                    horizontalLineToRelative(-4.0f)
                    verticalLineToRelative(-3.1f)
                    curveToRelative(1.63f, -0.33f, 2.98f, -1.46f, 3.61f, -2.96f)
                    curveTo(19.08f, 12.63f, 21.0f, 10.55f, 21.0f, 8.0f)
                    verticalLineTo(7.0f)
                    curveTo(21.0f, 5.9f, 20.1f, 5.0f, 19.0f, 5.0f)
                    close()
                    moveTo(5.0f, 8.0f)
                    verticalLineTo(7.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(3.82f)
                    curveTo(5.84f, 10.4f, 5.0f, 9.3f, 5.0f, 8.0f)
                    close()
                    moveTo(19.0f, 8.0f)
                    curveToRelative(0.0f, 1.3f, -0.84f, 2.4f, -2.0f, 2.82f)
                    verticalLineTo(7.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(8.0f)
                    close()
                }
            }
        return _emojiEvents!!
    }

private var _emojiEvents: ImageVector? = null

public val Icons.Filled.EmojiFlags: ImageVector
    get() {
        if (_emojiFlags != null) {
            return _emojiFlags!!
        }
        _emojiFlags =
            materialIcon(name = "Filled.EmojiFlags") {
                materialPath {
                    moveTo(14.0f, 9.0f)
                    lineToRelative(-1.0f, -2.0f)
                    horizontalLineTo(7.0f)
                    verticalLineTo(5.72f)
                    curveTo(7.6f, 5.38f, 8.0f, 4.74f, 8.0f, 4.0f)
                    curveToRelative(0.0f, -1.1f, -0.9f, -2.0f, -2.0f, -2.0f)
                    reflectiveCurveTo(4.0f, 2.9f, 4.0f, 4.0f)
                    curveToRelative(0.0f, 0.74f, 0.4f, 1.38f, 1.0f, 1.72f)
                    verticalLineTo(21.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(-4.0f)
                    horizontalLineToRelative(5.0f)
                    lineToRelative(1.0f, 2.0f)
                    horizontalLineToRelative(7.0f)
                    verticalLineTo(9.0f)
                    horizontalLineTo(14.0f)
                    close()
                    moveTo(18.0f, 17.0f)
                    horizontalLineToRelative(-4.0f)
                    lineToRelative(-1.0f, -2.0f)
                    horizontalLineTo(7.0f)
                    verticalLineTo(9.0f)
                    horizontalLineToRelative(5.0f)
                    lineToRelative(1.0f, 2.0f)
                    horizontalLineToRelative(5.0f)
                    verticalLineTo(17.0f)
                    close()
                }
            }
        return _emojiFlags!!
    }

private var _emojiFlags: ImageVector? = null

public val Icons.Filled.EmojiFoodBeverage: ImageVector
    get() {
        if (_emojiFoodBeverage != null) {
            return _emojiFoodBeverage!!
        }
        _emojiFoodBeverage =
            materialIcon(name = "Filled.EmojiFoodBeverage") {
                materialPath {
                    moveTo(20.0f, 3.0f)
                    horizontalLineTo(9.0f)
                    verticalLineToRelative(2.4f)
                    lineToRelative(1.81f, 1.45f)
                    curveTo(10.93f, 6.94f, 11.0f, 7.09f, 11.0f, 7.24f)
                    verticalLineToRelative(4.26f)
                    curveToRelative(0.0f, 0.28f, -0.22f, 0.5f, -0.5f, 0.5f)
                    horizontalLineToRelative(-4.0f)
                    curveTo(6.22f, 12.0f, 6.0f, 11.78f, 6.0f, 11.5f)
                    verticalLineTo(7.24f)
                    curveToRelative(0.0f, -0.15f, 0.07f, -0.3f, 0.19f, -0.39f)
                    lineTo(8.0f, 5.4f)
                    verticalLineTo(3.0f)
                    horizontalLineTo(4.0f)
                    verticalLineToRelative(10.0f)
                    curveToRelative(0.0f, 2.21f, 1.79f, 4.0f, 4.0f, 4.0f)
                    horizontalLineToRelative(6.0f)
                    curveToRelative(2.21f, 0.0f, 4.0f, -1.79f, 4.0f, -4.0f)
                    verticalLineToRelative(-3.0f)
                    horizontalLineToRelative(2.0f)
                    curveToRelative(1.11f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineTo(5.0f)
                    curveTo(22.0f, 3.89f, 21.11f, 3.0f, 20.0f, 3.0f)
                    close()
                    moveTo(20.0f, 8.0f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineTo(5.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(8.0f)
                    close()
                }
                materialPath {
                    moveTo(4.0f, 19.0f)
                    horizontalLineToRelative(16.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-16.0f)
                    close()
                }
            }
        return _emojiFoodBeverage!!
    }

private var _emojiFoodBeverage: ImageVector? = null

public val Icons.Filled.EmojiNature: ImageVector
    get() {
        if (_emojiNature != null) {
            return _emojiNature!!
        }
        _emojiNature =
            materialIcon(name = "Filled.EmojiNature") {
                materialPath {
                    moveTo(21.94f, 4.88f)
                    curveTo(21.76f, 4.35f, 21.25f, 4.0f, 20.68f, 4.0f)
                    curveToRelative(-0.03f, 0.0f, -0.06f, 0.0f, -0.09f, 0.0f)
                    horizontalLineTo(19.6f)
                    lineToRelative(-0.31f, -0.97f)
                    curveTo(19.15f, 2.43f, 18.61f, 2.0f, 18.0f, 2.0f)
                    horizontalLineToRelative(0.0f)
                    curveToRelative(-0.61f, 0.0f, -1.15f, 0.43f, -1.29f, 1.04f)
                    lineTo(16.4f, 4.0f)
                    horizontalLineToRelative(-0.98f)
                    curveToRelative(-0.03f, 0.0f, -0.06f, 0.0f, -0.09f, 0.0f)
                    curveToRelative(-0.57f, 0.0f, -1.08f, 0.35f, -1.26f, 0.88f)
                    curveToRelative(-0.19f, 0.56f, 0.04f, 1.17f, 0.56f, 1.48f)
                    lineToRelative(0.87f, 0.52f)
                    lineTo(15.1f, 8.12f)
                    curveToRelative(-0.23f, 0.58f, -0.04f, 1.25f, 0.45f, 1.62f)
                    curveTo(15.78f, 9.91f, 16.06f, 10.0f, 16.33f, 10.0f)
                    curveToRelative(0.31f, 0.0f, 0.61f, -0.11f, 0.86f, -0.32f)
                    lineTo(18.0f, 8.98f)
                    lineToRelative(0.81f, 0.7f)
                    curveTo(19.06f, 9.89f, 19.36f, 10.0f, 19.67f, 10.0f)
                    curveToRelative(0.27f, 0.0f, 0.55f, -0.09f, 0.78f, -0.26f)
                    curveToRelative(0.5f, -0.37f, 0.68f, -1.04f, 0.45f, -1.62f)
                    lineToRelative(-0.39f, -1.24f)
                    lineToRelative(0.87f, -0.52f)
                    curveTo(21.89f, 6.05f, 22.12f, 5.44f, 21.94f, 4.88f)
                    close()
                    moveTo(18.0f, 7.0f)
                    curveToRelative(-0.55f, 0.0f, -1.0f, -0.45f, -1.0f, -1.0f)
                    curveToRelative(0.0f, -0.55f, 0.45f, -1.0f, 1.0f, -1.0f)
                    reflectiveCurveToRelative(1.0f, 0.45f, 1.0f, 1.0f)
                    curveTo(19.0f, 6.55f, 18.55f, 7.0f, 18.0f, 7.0f)
                    close()
                }
                materialPath {
                    moveTo(13.49f, 10.51f)
                    curveToRelative(-0.43f, -0.43f, -0.94f, -0.73f, -1.49f, -0.93f)
                    verticalLineTo(8.0f)
                    horizontalLineToRelative(-1.0f)
                    verticalLineToRelative(1.38f)
                    curveToRelative(-0.11f, -0.01f, -0.23f, -0.03f, -0.34f, -0.03f)
                    curveToRelative(-1.02f, 0.0f, -2.05f, 0.39f, -2.83f, 1.17f)
                    curveToRelative(-0.16f, 0.16f, -0.3f, 0.34f, -0.43f, 0.53f)
                    lineTo(6.0f, 10.52f)
                    curveToRelative(-1.56f, -0.55f, -3.28f, 0.27f, -3.83f, 1.82f)
                    curveToRelative(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)
                    curveToRelative(-0.27f, 0.75f, -0.23f, 1.57f, 0.12f, 2.29f)
                    curveToRelative(0.23f, 0.48f, 0.58f, 0.87f, 1.0f, 1.16f)
                    curveToRelative(-0.38f, 1.35f, -0.06f, 2.85f, 1.0f, 3.91f)
                    curveToRelative(1.06f, 1.06f, 2.57f, 1.38f, 3.91f, 1.0f)
                    curveToRelative(0.29f, 0.42f, 0.68f, 0.77f, 1.16f, 1.0f)
                    curveTo(9.78f, 21.9f, 10.21f, 22.0f, 10.65f, 22.0f)
                    curveToRelative(0.34f, 0.0f, 0.68f, -0.06f, 1.01f, -0.17f)
                    curveToRelative(0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f)
                    curveToRelative(1.56f, -0.55f, 2.38f, -2.27f, 1.82f, -3.85f)
                    lineToRelative(-0.52f, -1.37f)
                    curveToRelative(0.18f, -0.13f, 0.36f, -0.27f, 0.53f, -0.43f)
                    curveToRelative(0.87f, -0.87f, 1.24f, -2.04f, 1.14f, -3.17f)
                    horizontalLineTo(16.0f)
                    verticalLineToRelative(-1.0f)
                    horizontalLineToRelative(-1.59f)
                    curveTo(14.22f, 11.46f, 13.92f, 10.95f, 13.49f, 10.51f)
                    close()
                    moveTo(4.67f, 14.29f)
                    curveToRelative(-0.25f, -0.09f, -0.45f, -0.27f, -0.57f, -0.51f)
                    reflectiveCurveToRelative(-0.13f, -0.51f, -0.04f, -0.76f)
                    curveToRelative(0.19f, -0.52f, 0.76f, -0.79f, 1.26f, -0.61f)
                    lineToRelative(3.16f, 1.19f)
                    curveTo(7.33f, 14.2f, 5.85f, 14.71f, 4.67f, 14.29f)
                    close()
                    moveTo(10.99f, 19.94f)
                    curveToRelative(-0.25f, 0.09f, -0.52f, 0.08f, -0.76f, -0.04f)
                    curveToRelative(-0.24f, -0.11f, -0.42f, -0.32f, -0.51f, -0.57f)
                    curveToRelative(-0.42f, -1.18f, 0.09f, -2.65f, 0.7f, -3.8f)
                    lineToRelative(1.18f, 3.13f)
                    curveTo(11.78f, 19.18f, 11.51f, 19.76f, 10.99f, 19.94f)
                    close()
                    moveTo(12.2f, 14.6f)
                    lineToRelative(-0.61f, -1.61f)
                    curveToRelative(0.0f, -0.01f, -0.01f, -0.02f, -0.02f, -0.03f)
                    curveToRelative(-0.02f, -0.04f, -0.04f, -0.08f, -0.06f, -0.12f)
                    curveToRelative(-0.02f, -0.04f, -0.04f, -0.07f, -0.07f, -0.11f)
                    curveToRelative(-0.03f, -0.03f, -0.06f, -0.06f, -0.09f, -0.09f)
                    curveToRelative(-0.03f, -0.03f, -0.06f, -0.06f, -0.09f, -0.09f)
                    curveToRelative(-0.03f, -0.03f, -0.07f, -0.05f, -0.11f, -0.07f)
                    curveToRelative(-0.04f, -0.02f, -0.07f, -0.05f, -0.12f, -0.06f)
                    curveToRelative(-0.01f, 0.0f, -0.02f, -0.01f, -0.03f, -0.02f)
                    lineTo(9.4f, 11.8f)
                    curveToRelative(0.36f, -0.29f, 0.79f, -0.46f, 1.26f, -0.46f)
                    curveToRelative(0.53f, 0.0f, 1.04f, 0.21f, 1.41f, 0.59f)
                    curveTo(12.8f, 12.66f, 12.84f, 13.81f, 12.2f, 14.6f)
                    close()
                }
            }
        return _emojiNature!!
    }

private var _emojiNature: ImageVector? = null

public val Icons.Filled.EmojiObjects: ImageVector
    get() {
        if (_emojiObjects != null) {
            return _emojiObjects!!
        }
        _emojiObjects =
            materialIcon(name = "Filled.EmojiObjects") {
                materialPath {
                    moveTo(12.0f, 3.0f)
                    curveToRelative(-0.46f, 0.0f, -0.93f, 0.04f, -1.4f, 0.14f)
                    curveTo(7.84f, 3.67f, 5.64f, 5.9f, 5.12f, 8.66f)
                    curveToRelative(-0.48f, 2.61f, 0.48f, 5.01f, 2.22f, 6.56f)
                    curveTo(7.77f, 15.6f, 8.0f, 16.13f, 8.0f, 16.69f)
                    verticalLineTo(19.0f)
                    curveToRelative(0.0f, 1.1f, 0.9f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(0.28f)
                    curveToRelative(0.35f, 0.6f, 0.98f, 1.0f, 1.72f, 1.0f)
                    reflectiveCurveToRelative(1.38f, -0.4f, 1.72f, -1.0f)
                    horizontalLineTo(14.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineToRelative(-2.31f)
                    curveToRelative(0.0f, -0.55f, 0.22f, -1.09f, 0.64f, -1.46f)
                    curveTo(18.09f, 13.95f, 19.0f, 12.08f, 19.0f, 10.0f)
                    curveTo(19.0f, 6.13f, 15.87f, 3.0f, 12.0f, 3.0f)
                    close()
                    moveTo(14.0f, 19.0f)
                    horizontalLineToRelative(-4.0f)
                    verticalLineToRelative(-1.0f)
                    horizontalLineToRelative(4.0f)
                    verticalLineTo(19.0f)
                    close()
                    moveTo(14.0f, 17.0f)
                    horizontalLineToRelative(-4.0f)
                    verticalLineToRelative(-1.0f)
                    horizontalLineToRelative(4.0f)
                    verticalLineTo(17.0f)
                    close()
                    moveTo(12.5f, 11.41f)
                    verticalLineTo(14.0f)
                    horizontalLineToRelative(-1.0f)
                    verticalLineToRelative(-2.59f)
                    lineTo(9.67f, 9.59f)
                    lineToRelative(0.71f, -0.71f)
                    lineTo(12.0f, 10.5f)
                    lineToRelative(1.62f, -1.62f)
                    lineToRelative(0.71f, 0.71f)
                    lineTo(12.5f, 11.41f)
                    close()
                }
            }
        return _emojiObjects!!
    }

private var _emojiObjects: ImageVector? = null

public val Icons.Filled.EmojiPeople: ImageVector
    get() {
        if (_emojiPeople != null) {
            return _emojiPeople!!
        }
        _emojiPeople =
            materialIcon(name = "Filled.EmojiPeople") {
                materialPath {
                    moveTo(12.0f, 4.0f)
                    moveToRelative(-2.0f, 0.0f)
                    arcToRelative(2.0f, 2.0f, 0.0f, true, true, 4.0f, 0.0f)
                    arcToRelative(2.0f, 2.0f, 0.0f, true, true, -4.0f, 0.0f)
                }
                materialPath {
                    moveTo(15.89f, 8.11f)
                    curveTo(15.5f, 7.72f, 14.83f, 7.0f, 13.53f, 7.0f)
                    curveToRelative(-0.21f, 0.0f, -1.42f, 0.0f, -2.54f, 0.0f)
                    curveTo(8.24f, 6.99f, 6.0f, 4.75f, 6.0f, 2.0f)
                    horizontalLineTo(4.0f)
                    curveToRelative(0.0f, 3.16f, 2.11f, 5.84f, 5.0f, 6.71f)
                    verticalLineTo(22.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(-6.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(6.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(10.05f)
                    lineTo(18.95f, 14.0f)
                    lineToRelative(1.41f, -1.41f)
                    lineTo(15.89f, 8.11f)
                    close()
                }
            }
        return _emojiPeople!!
    }

private var _emojiPeople: ImageVector? = null

public val Icons.Filled.EmojiSymbols: ImageVector
    get() {
        if (_emojiSymbols != null) {
            return _emojiSymbols!!
        }
        _emojiSymbols =
            materialIcon(name = "Filled.EmojiSymbols") {
                materialPath {
                    moveTo(3.0f, 2.0f)
                    horizontalLineToRelative(8.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-8.0f)
                    close()
                }
                materialPath {
                    moveTo(6.0f, 11.0f)
                    lineToRelative(2.0f, 0.0f)
                    lineToRelative(0.0f, -4.0f)
                    lineToRelative(3.0f, 0.0f)
                    lineToRelative(0.0f, -2.0f)
                    lineToRelative(-8.0f, 0.0f)
                    lineToRelative(0.0f, 2.0f)
                    lineToRelative(3.0f, 0.0f)
                    close()
                }
                materialPath {
                    moveTo(12.404f, 20.182f)
                    lineToRelative(7.778f, -7.778f)
                    lineToRelative(1.414f, 1.414f)
                    lineToRelative(-7.778f, 7.778f)
                    close()
                }
                materialPath {
                    moveTo(14.5f, 14.5f)
                    moveToRelative(-1.5f, 0.0f)
                    arcToRelative(1.5f, 1.5f, 0.0f, true, true, 3.0f, 0.0f)
                    arcToRelative(1.5f, 1.5f, 0.0f, true, true, -3.0f, 0.0f)
                }
                materialPath {
                    moveTo(19.5f, 19.5f)
                    moveToRelative(-1.5f, 0.0f)
                    arcToRelative(1.5f, 1.5f, 0.0f, true, true, 3.0f, 0.0f)
                    arcToRelative(1.5f, 1.5f, 0.0f, true, true, -3.0f, 0.0f)
                }
                materialPath {
                    moveTo(15.5f, 11.0f)
                    curveToRelative(1.38f, 0.0f, 2.5f, -1.12f, 2.5f, -2.5f)
                    verticalLineTo(4.0f)
                    horizontalLineToRelative(3.0f)
                    verticalLineTo(2.0f)
                    horizontalLineToRelative(-4.0f)
                    verticalLineToRelative(4.51f)
                    curveTo(16.58f, 6.19f, 16.07f, 6.0f, 15.5f, 6.0f)
                    curveTo(14.12f, 6.0f, 13.0f, 7.12f, 13.0f, 8.5f)
                    curveTo(13.0f, 9.88f, 14.12f, 11.0f, 15.5f, 11.0f)
                    close()
                }
                materialPath {
                    moveTo(9.74f, 15.96f)
                    lineToRelative(-1.41f, 1.41f)
                    lineToRelative(-0.71f, -0.71f)
                    lineToRelative(0.35f, -0.35f)
                    curveToRelative(0.98f, -0.98f, 0.98f, -2.56f, 0.0f, -3.54f)
                    curveToRelative(-0.49f, -0.49f, -1.13f, -0.73f, -1.77f, -0.73f)
                    curveToRelative(-0.64f, 0.0f, -1.28f, 0.24f, -1.77f, 0.73f)
                    curveToRelative(-0.98f, 0.98f, -0.98f, 2.56f, 0.0f, 3.54f)
                    lineToRelative(0.35f, 0.35f)
                    lineToRelative(-1.06f, 1.06f)
                    curveToRelative(-0.98f, 0.98f, -0.98f, 2.56f, 0.0f, 3.54f)
                    curveTo(4.22f, 21.76f, 4.86f, 22.0f, 5.5f, 22.0f)
                    reflectiveCurveToRelative(1.28f, -0.24f, 1.77f, -0.73f)
                    lineToRelative(1.06f, -1.06f)
                    lineToRelative(1.41f, 1.41f)
                    lineToRelative(1.41f, -1.41f)
                    lineToRelative(-1.41f, -1.41f)
                    lineToRelative(1.41f, -1.41f)
                    lineTo(9.74f, 15.96f)
                    close()
                    moveTo(5.85f, 14.2f)
                    curveToRelative(0.12f, -0.12f, 0.26f, -0.15f, 0.35f, -0.15f)
                    reflectiveCurveToRelative(0.23f, 0.03f, 0.35f, 0.15f)
                    curveToRelative(0.19f, 0.2f, 0.19f, 0.51f, 0.0f, 0.71f)
                    lineToRelative(-0.35f, 0.35f)
                    lineTo(5.85f, 14.9f)
                    curveTo(5.66f, 14.71f, 5.66f, 14.39f, 5.85f, 14.2f)
                    close()
                    moveTo(5.85f, 19.85f)
                    curveTo(5.73f, 19.97f, 5.59f, 20.0f, 5.5f, 20.0f)
                    reflectiveCurveToRelative(-0.23f, -0.03f, -0.35f, -0.15f)
                    curveToRelative(-0.19f, -0.19f, -0.19f, -0.51f, 0.0f, -0.71f)
                    lineToRelative(1.06f, -1.06f)
                    lineToRelative(0.71f, 0.71f)
                    lineTo(5.85f, 19.85f)
                    close()
                }
            }
        return _emojiSymbols!!
    }

private var _emojiSymbols: ImageVector? = null

public val Icons.Filled.EmojiTransportation: ImageVector
    get() {
        if (_emojiTransportation != null) {
            return _emojiTransportation!!
        }
        _emojiTransportation =
            materialIcon(name = "Filled.EmojiTransportation") {
                materialPath {
                    moveTo(20.57f, 10.66f)
                    curveTo(20.43f, 10.26f, 20.05f, 10.0f, 19.6f, 10.0f)
                    horizontalLineToRelative(-7.19f)
                    curveToRelative(-0.46f, 0.0f, -0.83f, 0.26f, -0.98f, 0.66f)
                    lineTo(10.0f, 14.77f)
                    lineToRelative(0.01f, 5.51f)
                    curveToRelative(0.0f, 0.38f, 0.31f, 0.72f, 0.69f, 0.72f)
                    horizontalLineToRelative(0.62f)
                    curveTo(11.7f, 21.0f, 12.0f, 20.62f, 12.0f, 20.24f)
                    verticalLineTo(19.0f)
                    horizontalLineToRelative(8.0f)
                    verticalLineToRelative(1.24f)
                    curveToRelative(0.0f, 0.38f, 0.31f, 0.76f, 0.69f, 0.76f)
                    horizontalLineToRelative(0.61f)
                    curveToRelative(0.38f, 0.0f, 0.69f, -0.34f, 0.69f, -0.72f)
                    lineTo(22.0f, 18.91f)
                    verticalLineToRelative(-4.14f)
                    lineTo(20.57f, 10.66f)
                    close()
                    moveTo(12.41f, 11.0f)
                    horizontalLineToRelative(7.19f)
                    lineToRelative(1.03f, 3.0f)
                    horizontalLineToRelative(-9.25f)
                    lineTo(12.41f, 11.0f)
                    close()
                    moveTo(12.0f, 17.0f)
                    curveToRelative(-0.55f, 0.0f, -1.0f, -0.45f, -1.0f, -1.0f)
                    reflectiveCurveToRelative(0.45f, -1.0f, 1.0f, -1.0f)
                    reflectiveCurveToRelative(1.0f, 0.45f, 1.0f, 1.0f)
                    reflectiveCurveTo(12.55f, 17.0f, 12.0f, 17.0f)
                    close()
                    moveTo(20.0f, 17.0f)
                    curveToRelative(-0.55f, 0.0f, -1.0f, -0.45f, -1.0f, -1.0f)
                    reflectiveCurveToRelative(0.45f, -1.0f, 1.0f, -1.0f)
                    reflectiveCurveToRelative(1.0f, 0.45f, 1.0f, 1.0f)
                    reflectiveCurveTo(20.55f, 17.0f, 20.0f, 17.0f)
                    close()
                }
                materialPath {
                    moveTo(14.0f, 9.0f)
                    lineToRelative(1.0f, 0.0f)
                    lineToRelative(0.0f, -6.0f)
                    lineToRelative(-8.0f, 0.0f)
                    lineToRelative(0.0f, 5.0f)
                    lineToRelative(-5.0f, 0.0f)
                    lineToRelative(0.0f, 13.0f)
                    lineToRelative(1.0f, 0.0f)
                    lineToRelative(0.0f, -12.0f)
                    lineToRelative(5.0f, 0.0f)
                    lineToRelative(0.0f, -5.0f)
                    lineToRelative(6.0f, 0.0f)
                    close()
                }
                materialPath {
                    moveTo(5.0f, 11.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                }
                materialPath {
                    moveTo(10.0f, 5.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                }
                materialPath {
                    moveTo(5.0f, 15.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                }
                materialPath {
                    moveTo(5.0f, 19.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                }
            }
        return _emojiTransportation!!
    }

private var _emojiTransportation: ImageVector? = null

// =============================================================================
// Outlined icons
// =============================================================================

public val Icons.Outlined.ContentCopy: ImageVector
    get() {
        if (_contentCopy != null) {
            return _contentCopy!!
        }
        _contentCopy =
            materialIcon(name = "Outlined.ContentCopy") {
                materialPath {
                    moveTo(16.0f, 1.0f)
                    lineTo(4.0f, 1.0f)
                    curveToRelative(-1.1f, 0.0f, -2.0f, 0.9f, -2.0f, 2.0f)
                    verticalLineToRelative(14.0f)
                    horizontalLineToRelative(2.0f)
                    lineTo(4.0f, 3.0f)
                    horizontalLineToRelative(12.0f)
                    lineTo(16.0f, 1.0f)
                    close()
                    moveTo(19.0f, 5.0f)
                    lineTo(8.0f, 5.0f)
                    curveToRelative(-1.1f, 0.0f, -2.0f, 0.9f, -2.0f, 2.0f)
                    verticalLineToRelative(14.0f)
                    curveToRelative(0.0f, 1.1f, 0.9f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(11.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    lineTo(21.0f, 7.0f)
                    curveToRelative(0.0f, -1.1f, -0.9f, -2.0f, -2.0f, -2.0f)
                    close()
                    moveTo(19.0f, 21.0f)
                    lineTo(8.0f, 21.0f)
                    lineTo(8.0f, 7.0f)
                    horizontalLineToRelative(11.0f)
                    verticalLineToRelative(14.0f)
                    close()
                }
            }
        return _contentCopy!!
    }

private var _contentCopy: ImageVector? = null

public val Icons.Outlined.FileDownload: ImageVector
    get() {
        if (_fileDownload != null) {
            return _fileDownload!!
        }
        _fileDownload =
            materialIcon(name = "Outlined.FileDownload") {
                materialPath {
                    moveTo(18.0f, 15.0f)
                    verticalLineToRelative(3.0f)
                    horizontalLineTo(6.0f)
                    verticalLineToRelative(-3.0f)
                    horizontalLineTo(4.0f)
                    verticalLineToRelative(3.0f)
                    curveToRelative(0.0f, 1.1f, 0.9f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(12.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineToRelative(-3.0f)
                    horizontalLineTo(18.0f)
                    close()
                    moveTo(17.0f, 11.0f)
                    lineToRelative(-1.41f, -1.41f)
                    lineTo(13.0f, 12.17f)
                    verticalLineTo(4.0f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineToRelative(8.17f)
                    lineTo(8.41f, 9.59f)
                    lineTo(7.0f, 11.0f)
                    lineToRelative(5.0f, 5.0f)
                    lineTo(17.0f, 11.0f)
                    close()
                }
            }
        return _fileDownload!!
    }

private var _fileDownload: ImageVector? = null

public val Icons.Outlined.FileUpload: ImageVector
    get() {
        if (_fileUpload != null) {
            return _fileUpload!!
        }
        _fileUpload =
            materialIcon(name = "Outlined.FileUpload") {
                materialPath {
                    moveTo(18.0f, 15.0f)
                    verticalLineToRelative(3.0f)
                    horizontalLineTo(6.0f)
                    verticalLineToRelative(-3.0f)
                    horizontalLineTo(4.0f)
                    verticalLineToRelative(3.0f)
                    curveToRelative(0.0f, 1.1f, 0.9f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(12.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineToRelative(-3.0f)
                    horizontalLineTo(18.0f)
                    close()
                    moveTo(7.0f, 9.0f)
                    lineToRelative(1.41f, 1.41f)
                    lineTo(11.0f, 7.83f)
                    verticalLineTo(16.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(7.83f)
                    lineToRelative(2.59f, 2.58f)
                    lineTo(17.0f, 9.0f)
                    lineToRelative(-5.0f, -5.0f)
                    lineTo(7.0f, 9.0f)
                    close()
                }
            }
        return _fileUpload!!
    }

private var _fileUpload: ImageVector? = null

public val Icons.Outlined.FormatSize: ImageVector
    get() {
        if (_formatSize != null) {
            return _formatSize!!
        }
        _formatSize =
            materialIcon(name = "Outlined.FormatSize") {
                materialPath {
                    moveTo(9.0f, 4.0f)
                    verticalLineToRelative(3.0f)
                    horizontalLineToRelative(5.0f)
                    verticalLineToRelative(12.0f)
                    horizontalLineToRelative(3.0f)
                    lineTo(17.0f, 7.0f)
                    horizontalLineToRelative(5.0f)
                    lineTo(22.0f, 4.0f)
                    lineTo(9.0f, 4.0f)
                    close()
                    moveTo(3.0f, 12.0f)
                    horizontalLineToRelative(3.0f)
                    verticalLineToRelative(7.0f)
                    horizontalLineToRelative(3.0f)
                    verticalLineToRelative(-7.0f)
                    horizontalLineToRelative(3.0f)
                    lineTo(12.0f, 9.0f)
                    lineTo(3.0f, 9.0f)
                    verticalLineToRelative(3.0f)
                    close()
                }
            }
        return _formatSize!!
    }

private var _formatSize: ImageVector? = null

// §34/S22 — 顯示羅馬字 toggle icon. Classic Material "abc" glyph,
// transcribed verbatim from materialiconsoutlined/abc/24px.svg (viewBox
// 0 0 24 24) so winding matches the source exactly. Mirrors iOS SF Symbol "abc".
public val Icons.Outlined.Abc: ImageVector
    get() {
        if (_abc != null) {
            return _abc!!
        }
        _abc =
            materialIcon(name = "Outlined.Abc") {
                materialPath {
                    moveTo(21.0f, 11.0f)
                    horizontalLineToRelative(-1.5f)
                    verticalLineToRelative(-0.5f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineToRelative(3.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(13.0f)
                    horizontalLineTo(21.0f)
                    verticalLineToRelative(1.0f)
                    curveToRelative(0.0f, 0.55f, -0.45f, 1.0f, -1.0f, 1.0f)
                    horizontalLineToRelative(-3.0f)
                    curveToRelative(-0.55f, 0.0f, -1.0f, -0.45f, -1.0f, -1.0f)
                    verticalLineToRelative(-4.0f)
                    curveToRelative(0.0f, -0.55f, 0.45f, -1.0f, 1.0f, -1.0f)
                    horizontalLineToRelative(3.0f)
                    curveToRelative(0.55f, 0.0f, 1.0f, 0.45f, 1.0f, 1.0f)
                    verticalLineTo(11.0f)
                    close()
                    moveTo(8.0f, 10.0f)
                    verticalLineToRelative(5.0f)
                    horizontalLineTo(6.5f)
                    verticalLineToRelative(-1.5f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineTo(15.0f)
                    horizontalLineTo(3.0f)
                    verticalLineToRelative(-5.0f)
                    curveToRelative(0.0f, -0.55f, 0.45f, -1.0f, 1.0f, -1.0f)
                    horizontalLineToRelative(3.0f)
                    curveTo(7.55f, 9.0f, 8.0f, 9.45f, 8.0f, 10.0f)
                    close()
                    moveTo(6.5f, 10.5f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineTo(12.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(10.5f)
                    close()
                    moveTo(13.5f, 12.0f)
                    curveToRelative(0.55f, 0.0f, 1.0f, 0.45f, 1.0f, 1.0f)
                    verticalLineToRelative(1.0f)
                    curveToRelative(0.0f, 0.55f, -0.45f, 1.0f, -1.0f, 1.0f)
                    horizontalLineToRelative(-4.0f)
                    verticalLineTo(9.0f)
                    horizontalLineToRelative(4.0f)
                    curveToRelative(0.55f, 0.0f, 1.0f, 0.45f, 1.0f, 1.0f)
                    verticalLineToRelative(1.0f)
                    curveTo(14.5f, 11.55f, 14.05f, 12.0f, 13.5f, 12.0f)
                    close()
                    moveTo(11.0f, 10.5f)
                    verticalLineToRelative(0.75f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(10.5f)
                    horizontalLineTo(11.0f)
                    close()
                    moveTo(13.0f, 12.75f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineToRelative(0.75f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(12.75f)
                    close()
                }
            }
        return _abc!!
    }

private var _abc: ImageVector? = null

public val Icons.Outlined.Language: ImageVector
    get() {
        if (_language != null) {
            return _language!!
        }
        _language =
            materialIcon(name = "Outlined.Language") {
                materialPath {
                    moveTo(11.99f, 2.0f)
                    curveTo(6.47f, 2.0f, 2.0f, 6.48f, 2.0f, 12.0f)
                    reflectiveCurveToRelative(4.47f, 10.0f, 9.99f, 10.0f)
                    curveTo(17.52f, 22.0f, 22.0f, 17.52f, 22.0f, 12.0f)
                    reflectiveCurveTo(17.52f, 2.0f, 11.99f, 2.0f)
                    close()
                    moveTo(18.92f, 8.0f)
                    horizontalLineToRelative(-2.95f)
                    curveToRelative(-0.32f, -1.25f, -0.78f, -2.45f, -1.38f, -3.56f)
                    curveToRelative(1.84f, 0.63f, 3.37f, 1.91f, 4.33f, 3.56f)
                    close()
                    moveTo(12.0f, 4.04f)
                    curveToRelative(0.83f, 1.2f, 1.48f, 2.53f, 1.91f, 3.96f)
                    horizontalLineToRelative(-3.82f)
                    curveToRelative(0.43f, -1.43f, 1.08f, -2.76f, 1.91f, -3.96f)
                    close()
                    moveTo(4.26f, 14.0f)
                    curveTo(4.1f, 13.36f, 4.0f, 12.69f, 4.0f, 12.0f)
                    reflectiveCurveToRelative(0.1f, -1.36f, 0.26f, -2.0f)
                    horizontalLineToRelative(3.38f)
                    curveToRelative(-0.08f, 0.66f, -0.14f, 1.32f, -0.14f, 2.0f)
                    reflectiveCurveToRelative(0.06f, 1.34f, 0.14f, 2.0f)
                    lineTo(4.26f, 14.0f)
                    close()
                    moveTo(5.08f, 16.0f)
                    horizontalLineToRelative(2.95f)
                    curveToRelative(0.32f, 1.25f, 0.78f, 2.45f, 1.38f, 3.56f)
                    curveToRelative(-1.84f, -0.63f, -3.37f, -1.9f, -4.33f, -3.56f)
                    close()
                    moveTo(8.03f, 8.0f)
                    lineTo(5.08f, 8.0f)
                    curveToRelative(0.96f, -1.66f, 2.49f, -2.93f, 4.33f, -3.56f)
                    curveTo(8.81f, 5.55f, 8.35f, 6.75f, 8.03f, 8.0f)
                    close()
                    moveTo(12.0f, 19.96f)
                    curveToRelative(-0.83f, -1.2f, -1.48f, -2.53f, -1.91f, -3.96f)
                    horizontalLineToRelative(3.82f)
                    curveToRelative(-0.43f, 1.43f, -1.08f, 2.76f, -1.91f, 3.96f)
                    close()
                    moveTo(14.34f, 14.0f)
                    lineTo(9.66f, 14.0f)
                    curveToRelative(-0.09f, -0.66f, -0.16f, -1.32f, -0.16f, -2.0f)
                    reflectiveCurveToRelative(0.07f, -1.35f, 0.16f, -2.0f)
                    horizontalLineToRelative(4.68f)
                    curveToRelative(0.09f, 0.65f, 0.16f, 1.32f, 0.16f, 2.0f)
                    reflectiveCurveToRelative(-0.07f, 1.34f, -0.16f, 2.0f)
                    close()
                    moveTo(14.59f, 19.56f)
                    curveToRelative(0.6f, -1.11f, 1.06f, -2.31f, 1.38f, -3.56f)
                    horizontalLineToRelative(2.95f)
                    curveToRelative(-0.96f, 1.65f, -2.49f, 2.93f, -4.33f, 3.56f)
                    close()
                    moveTo(16.36f, 14.0f)
                    curveToRelative(0.08f, -0.66f, 0.14f, -1.32f, 0.14f, -2.0f)
                    reflectiveCurveToRelative(-0.06f, -1.34f, -0.14f, -2.0f)
                    horizontalLineToRelative(3.38f)
                    curveToRelative(0.16f, 0.64f, 0.26f, 1.31f, 0.26f, 2.0f)
                    reflectiveCurveToRelative(-0.1f, 1.36f, -0.26f, 2.0f)
                    horizontalLineToRelative(-3.38f)
                    close()
                }
            }
        return _language!!
    }

private var _language: ImageVector? = null

public val Icons.Outlined.SpaceBar: ImageVector
    get() {
        if (_spaceBar != null) {
            return _spaceBar!!
        }
        _spaceBar =
            materialIcon(name = "Outlined.SpaceBar") {
                materialPath {
                    moveTo(18.0f, 9.0f)
                    verticalLineToRelative(4.0f)
                    horizontalLineTo(6.0f)
                    verticalLineTo(9.0f)
                    horizontalLineTo(4.0f)
                    verticalLineToRelative(6.0f)
                    horizontalLineToRelative(16.0f)
                    verticalLineTo(9.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                }
            }
        return _spaceBar!!
    }

private var _spaceBar: ImageVector? = null

public val Icons.Outlined.Translate: ImageVector
    get() {
        if (_translate != null) {
            return _translate!!
        }
        _translate =
            materialIcon(name = "Outlined.Translate") {
                materialPath {
                    moveTo(12.87f, 15.07f)
                    lineToRelative(-2.54f, -2.51f)
                    lineToRelative(0.03f, -0.03f)
                    curveToRelative(1.74f, -1.94f, 2.98f, -4.17f, 3.71f, -6.53f)
                    lineTo(17.0f, 6.0f)
                    lineTo(17.0f, 4.0f)
                    horizontalLineToRelative(-7.0f)
                    lineTo(10.0f, 2.0f)
                    lineTo(8.0f, 2.0f)
                    verticalLineToRelative(2.0f)
                    lineTo(1.0f, 4.0f)
                    verticalLineToRelative(1.99f)
                    horizontalLineToRelative(11.17f)
                    curveTo(11.5f, 7.92f, 10.44f, 9.75f, 9.0f, 11.35f)
                    curveTo(8.07f, 10.32f, 7.3f, 9.19f, 6.69f, 8.0f)
                    horizontalLineToRelative(-2.0f)
                    curveToRelative(0.73f, 1.63f, 1.73f, 3.17f, 2.98f, 4.56f)
                    lineToRelative(-5.09f, 5.02f)
                    lineTo(4.0f, 19.0f)
                    lineToRelative(5.0f, -5.0f)
                    lineToRelative(3.11f, 3.11f)
                    lineToRelative(0.76f, -2.04f)
                    close()
                    moveTo(18.5f, 10.0f)
                    horizontalLineToRelative(-2.0f)
                    lineTo(12.0f, 22.0f)
                    horizontalLineToRelative(2.0f)
                    lineToRelative(1.12f, -3.0f)
                    horizontalLineToRelative(4.75f)
                    lineTo(21.0f, 22.0f)
                    horizontalLineToRelative(2.0f)
                    lineToRelative(-4.5f, -12.0f)
                    close()
                    moveTo(15.88f, 17.0f)
                    lineToRelative(1.62f, -4.33f)
                    lineTo(19.12f, 17.0f)
                    horizontalLineToRelative(-3.24f)
                    close()
                }
            }
        return _translate!!
    }

private var _translate: ImageVector? = null

public val Icons.Outlined.Vibration: ImageVector
    get() {
        if (_vibration != null) {
            return _vibration!!
        }
        _vibration =
            materialIcon(name = "Outlined.Vibration") {
                materialPath {
                    moveTo(0.0f, 15.0f)
                    horizontalLineToRelative(2.0f)
                    lineTo(2.0f, 9.0f)
                    lineTo(0.0f, 9.0f)
                    verticalLineToRelative(6.0f)
                    close()
                    moveTo(3.0f, 17.0f)
                    horizontalLineToRelative(2.0f)
                    lineTo(5.0f, 7.0f)
                    lineTo(3.0f, 7.0f)
                    verticalLineToRelative(10.0f)
                    close()
                    moveTo(22.0f, 9.0f)
                    verticalLineToRelative(6.0f)
                    horizontalLineToRelative(2.0f)
                    lineTo(24.0f, 9.0f)
                    horizontalLineToRelative(-2.0f)
                    close()
                    moveTo(19.0f, 17.0f)
                    horizontalLineToRelative(2.0f)
                    lineTo(21.0f, 7.0f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineToRelative(10.0f)
                    close()
                    moveTo(16.5f, 3.0f)
                    horizontalLineToRelative(-9.0f)
                    curveTo(6.67f, 3.0f, 6.0f, 3.67f, 6.0f, 4.5f)
                    verticalLineToRelative(15.0f)
                    curveToRelative(0.0f, 0.83f, 0.67f, 1.5f, 1.5f, 1.5f)
                    horizontalLineToRelative(9.0f)
                    curveToRelative(0.83f, 0.0f, 1.5f, -0.67f, 1.5f, -1.5f)
                    verticalLineToRelative(-15.0f)
                    curveToRelative(0.0f, -0.83f, -0.67f, -1.5f, -1.5f, -1.5f)
                    close()
                    moveTo(16.0f, 19.0f)
                    lineTo(8.0f, 19.0f)
                    lineTo(8.0f, 5.0f)
                    horizontalLineToRelative(8.0f)
                    verticalLineToRelative(14.0f)
                    close()
                }
            }
        return _vibration!!
    }

private var _vibration: ImageVector? = null

public val Icons.Outlined.ViewStream: ImageVector
    get() {
        if (_viewStream != null) {
            return _viewStream!!
        }
        _viewStream =
            materialIcon(name = "Outlined.ViewStream") {
                materialPath {
                    moveTo(3.0f, 7.0f)
                    verticalLineToRelative(10.0f)
                    curveToRelative(0.0f, 1.1f, 0.9f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(14.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineTo(7.0f)
                    curveToRelative(0.0f, -1.1f, -0.9f, -2.0f, -2.0f, -2.0f)
                    horizontalLineTo(5.0f)
                    curveTo(3.9f, 5.0f, 3.0f, 5.9f, 3.0f, 7.0f)
                    close()
                    moveTo(19.0f, 17.0f)
                    horizontalLineTo(5.0f)
                    verticalLineToRelative(-4.0f)
                    horizontalLineToRelative(14.0f)
                    verticalLineTo(17.0f)
                    close()
                    moveTo(5.0f, 11.0f)
                    verticalLineTo(7.0f)
                    horizontalLineToRelative(14.0f)
                    verticalLineToRelative(4.0f)
                    horizontalLineTo(5.0f)
                    close()
                }
            }
        return _viewStream!!
    }

private var _viewStream: ImageVector? = null

// =============================================================================
// AutoMirrored.Outlined icons
// =============================================================================

public val Icons.AutoMirrored.Outlined.MenuBook: ImageVector
    get() {
        if (_menuBook != null) {
            return _menuBook!!
        }
        _menuBook =
            materialIcon(name = "AutoMirrored.Outlined.MenuBook", autoMirror = true) {
                materialPath {
                    moveTo(21.0f, 5.0f)
                    curveToRelative(-1.11f, -0.35f, -2.33f, -0.5f, -3.5f, -0.5f)
                    curveToRelative(-1.95f, 0.0f, -4.05f, 0.4f, -5.5f, 1.5f)
                    curveToRelative(-1.45f, -1.1f, -3.55f, -1.5f, -5.5f, -1.5f)
                    reflectiveCurveTo(2.45f, 4.9f, 1.0f, 6.0f)
                    verticalLineToRelative(14.65f)
                    curveToRelative(0.0f, 0.25f, 0.25f, 0.5f, 0.5f, 0.5f)
                    curveToRelative(0.1f, 0.0f, 0.15f, -0.05f, 0.25f, -0.05f)
                    curveTo(3.1f, 20.45f, 5.05f, 20.0f, 6.5f, 20.0f)
                    curveToRelative(1.95f, 0.0f, 4.05f, 0.4f, 5.5f, 1.5f)
                    curveToRelative(1.35f, -0.85f, 3.8f, -1.5f, 5.5f, -1.5f)
                    curveToRelative(1.65f, 0.0f, 3.35f, 0.3f, 4.75f, 1.05f)
                    curveToRelative(0.1f, 0.05f, 0.15f, 0.05f, 0.25f, 0.05f)
                    curveToRelative(0.25f, 0.0f, 0.5f, -0.25f, 0.5f, -0.5f)
                    verticalLineTo(6.0f)
                    curveTo(22.4f, 5.55f, 21.75f, 5.25f, 21.0f, 5.0f)
                    close()
                    moveTo(21.0f, 18.5f)
                    curveToRelative(-1.1f, -0.35f, -2.3f, -0.5f, -3.5f, -0.5f)
                    curveToRelative(-1.7f, 0.0f, -4.15f, 0.65f, -5.5f, 1.5f)
                    verticalLineTo(8.0f)
                    curveToRelative(1.35f, -0.85f, 3.8f, -1.5f, 5.5f, -1.5f)
                    curveToRelative(1.2f, 0.0f, 2.4f, 0.15f, 3.5f, 0.5f)
                    verticalLineTo(18.5f)
                    close()
                }
                materialPath {
                    moveTo(17.5f, 10.5f)
                    curveToRelative(0.88f, 0.0f, 1.73f, 0.09f, 2.5f, 0.26f)
                    verticalLineTo(9.24f)
                    curveTo(19.21f, 9.09f, 18.36f, 9.0f, 17.5f, 9.0f)
                    curveToRelative(-1.7f, 0.0f, -3.24f, 0.29f, -4.5f, 0.83f)
                    verticalLineToRelative(1.66f)
                    curveTo(14.13f, 10.85f, 15.7f, 10.5f, 17.5f, 10.5f)
                    close()
                }
                materialPath {
                    moveTo(13.0f, 12.49f)
                    verticalLineToRelative(1.66f)
                    curveToRelative(1.13f, -0.64f, 2.7f, -0.99f, 4.5f, -0.99f)
                    curveToRelative(0.88f, 0.0f, 1.73f, 0.09f, 2.5f, 0.26f)
                    verticalLineTo(11.9f)
                    curveToRelative(-0.79f, -0.15f, -1.64f, -0.24f, -2.5f, -0.24f)
                    curveTo(15.8f, 11.66f, 14.26f, 11.96f, 13.0f, 12.49f)
                    close()
                }
                materialPath {
                    moveTo(17.5f, 14.33f)
                    curveToRelative(-1.7f, 0.0f, -3.24f, 0.29f, -4.5f, 0.83f)
                    verticalLineToRelative(1.66f)
                    curveToRelative(1.13f, -0.64f, 2.7f, -0.99f, 4.5f, -0.99f)
                    curveToRelative(0.88f, 0.0f, 1.73f, 0.09f, 2.5f, 0.26f)
                    verticalLineToRelative(-1.52f)
                    curveTo(19.21f, 14.41f, 18.36f, 14.33f, 17.5f, 14.33f)
                    close()
                }
            }
        return _menuBook!!
    }

private var _menuBook: ImageVector? = null

public val Icons.AutoMirrored.Outlined.OpenInNew: ImageVector
    get() {
        if (_openInNew != null) {
            return _openInNew!!
        }
        _openInNew =
            materialIcon(name = "AutoMirrored.Outlined.OpenInNew", autoMirror = true) {
                materialPath {
                    moveTo(19.0f, 19.0f)
                    horizontalLineTo(5.0f)
                    verticalLineTo(5.0f)
                    horizontalLineToRelative(7.0f)
                    verticalLineTo(3.0f)
                    horizontalLineTo(5.0f)
                    curveToRelative(-1.11f, 0.0f, -2.0f, 0.9f, -2.0f, 2.0f)
                    verticalLineToRelative(14.0f)
                    curveToRelative(0.0f, 1.1f, 0.89f, 2.0f, 2.0f, 2.0f)
                    horizontalLineToRelative(14.0f)
                    curveToRelative(1.1f, 0.0f, 2.0f, -0.9f, 2.0f, -2.0f)
                    verticalLineToRelative(-7.0f)
                    horizontalLineToRelative(-2.0f)
                    verticalLineToRelative(7.0f)
                    close()
                    moveTo(14.0f, 3.0f)
                    verticalLineToRelative(2.0f)
                    horizontalLineToRelative(3.59f)
                    lineToRelative(-9.83f, 9.83f)
                    lineToRelative(1.41f, 1.41f)
                    lineTo(19.0f, 6.41f)
                    verticalLineTo(10.0f)
                    horizontalLineToRelative(2.0f)
                    verticalLineTo(3.0f)
                    horizontalLineToRelative(-7.0f)
                    close()
                }
            }
        return _openInNew!!
    }

private var _openInNew: ImageVector? = null

public val Icons.AutoMirrored.Outlined.VolumeUp: ImageVector
    get() {
        if (_volumeUp != null) {
            return _volumeUp!!
        }
        _volumeUp =
            materialIcon(name = "AutoMirrored.Outlined.VolumeUp", autoMirror = true) {
                materialPath {
                    moveTo(3.0f, 9.0f)
                    verticalLineToRelative(6.0f)
                    horizontalLineToRelative(4.0f)
                    lineToRelative(5.0f, 5.0f)
                    lineTo(12.0f, 4.0f)
                    lineTo(7.0f, 9.0f)
                    lineTo(3.0f, 9.0f)
                    close()
                    moveTo(10.0f, 8.83f)
                    verticalLineToRelative(6.34f)
                    lineTo(7.83f, 13.0f)
                    lineTo(5.0f, 13.0f)
                    verticalLineToRelative(-2.0f)
                    horizontalLineToRelative(2.83f)
                    lineTo(10.0f, 8.83f)
                    close()
                    moveTo(16.5f, 12.0f)
                    curveToRelative(0.0f, -1.77f, -1.02f, -3.29f, -2.5f, -4.03f)
                    verticalLineToRelative(8.05f)
                    curveToRelative(1.48f, -0.73f, 2.5f, -2.25f, 2.5f, -4.02f)
                    close()
                    moveTo(14.0f, 3.23f)
                    verticalLineToRelative(2.06f)
                    curveToRelative(2.89f, 0.86f, 5.0f, 3.54f, 5.0f, 6.71f)
                    reflectiveCurveToRelative(-2.11f, 5.85f, -5.0f, 6.71f)
                    verticalLineToRelative(2.06f)
                    curveToRelative(4.01f, -0.91f, 7.0f, -4.49f, 7.0f, -8.77f)
                    curveToRelative(0.0f, -4.28f, -2.99f, -7.86f, -7.0f, -8.77f)
                    close()
                }
            }
        return _volumeUp!!
    }

private var _volumeUp: ImageVector? = null
