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
// The emoji-category icons live with the emoji keyboard (ime/media/emoji/EmojiCategoryIcons.kt).
//
// Icons in this file (5 total):
//   Outlined: ContentCopy, FileDownload, FileUpload
//   AutoMirrored.Outlined: MenuBook, OpenInNew

package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.materialIcon
import androidx.compose.material.icons.materialPath
import androidx.compose.ui.graphics.vector.ImageVector

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
