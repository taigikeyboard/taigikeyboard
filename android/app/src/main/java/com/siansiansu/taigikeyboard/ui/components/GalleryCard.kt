package com.siansiansu.taigikeyboard.ui.components

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Card-shelf building blocks shared by the Theme and Layout tabs: a titled horizontal shelf, a
// selectable preview card, a screenshot preview and a create-new card. Mirrors iOS GalleryCard.swift.

// Card metrics — 200.dp is an intentional divergence from iOS 240pt. The 585/395 aspect matches
// the generated Android keyboard screenshots (theme_*_preview + layout_*_preview;
// PreviewAssetGeneratorTest renders them at this width x 4 and this aspect), taller than iOS's
// 720/454 — the Android keyboard is taller, so the card follows the Android keyboard shape rather
// than the iOS card slot (forcing iOS 585/369 here clipped the screenshots' top tone-mark row).
internal const val GALLERY_CARD_WIDTH_DP = 200
internal const val GALLERY_PREVIEW_ASPECT = 585f / 395f
private val GALLERY_CARD_SPACING = 12.dp
private val GALLERY_CARD_SHAPE = RoundedCornerShape(10.dp)

// One shelf: a section header above a horizontally scrolling row of cards.
@Composable
fun GalleryShelf(
    title: String,
    content: @Composable RowScope.() -> Unit,
) {
    SectionHeader(title)
    Row(
        modifier =
            Modifier
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
        horizontalArrangement = Arrangement.spacedBy(GALLERY_CARD_SPACING),
        content = content,
    )
}

// A selectable card: a tappable preview with the selection marker (scrim + checkmark + border)
// above a title row. Only the preview is the tap target, so a trailing [accessory] (the custom
// theme overflow menu) never selects the card.
@Composable
fun GalleryCard(
    title: String,
    isSelected: Boolean,
    onClick: () -> Unit,
    preview: @Composable () -> Unit,
    accessory: @Composable () -> Unit = {},
) {
    val checkmarkScale by animateFloatAsState(
        targetValue = if (isSelected) 1f else 0f,
        animationSpec =
            spring(
                dampingRatio = Spring.DampingRatioMediumBouncy,
                stiffness = Spring.StiffnessMedium,
            ),
        label = "checkmarkScale",
    )
    val overlayAlpha by animateFloatAsState(
        targetValue = if (isSelected) 1f else 0f,
        label = "overlayAlpha",
    )

    Column(
        modifier = Modifier.width(GALLERY_CARD_WIDTH_DP.dp),
    ) {
        Surface(
            shape = GALLERY_CARD_SHAPE,
            color = MaterialTheme.colorScheme.surfaceContainerHigh,
            border =
                if (isSelected) {
                    BorderStroke(2.5.dp, MaterialTheme.colorScheme.primary)
                } else {
                    null
                },
            modifier = Modifier.clickable(onClick = onClick),
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(GALLERY_PREVIEW_ASPECT),
            ) {
                preview()

                if (overlayAlpha > 0f) {
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .alpha(overlayAlpha)
                                .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.25f)),
                    )
                }

                if (checkmarkScale > 0f) {
                    Box(
                        modifier =
                            Modifier
                                .align(Alignment.Center)
                                .scale(checkmarkScale)
                                .size(36.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Default.Check,
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.smallIconSize),
                            tint = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        Row(verticalAlignment = Alignment.CenterVertically) {
            GalleryCardTitle(title, Modifier.weight(1f))
            accessory()
        }
    }
}

// A card preview: the bundled screenshot when supplied, else a neutral placeholder fixed to the
// card aspect so cards never change height once screenshots land.
@Composable
fun GalleryScreenshot(
    @DrawableRes previewRes: Int?,
    title: String,
) {
    if (previewRes != null) {
        Image(
            painter = painterResource(previewRes),
            contentDescription = title,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop,
        )
    } else {
        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.surfaceContainerHighest),
            contentAlignment = Alignment.Center,
        ) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(
                    painter = painterResource(R.drawable.keyboard_24),
                    contentDescription = null,
                    modifier = Modifier.size(28.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(6.dp))
                Text(
                    text = title,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
        }
    }
}

// The leading card of a custom shelf: a neutral panel with a centered "+" glyph. With a
// [comingSoonBadge] (and no [onClick]) the panel is dimmed under that badge and not tappable.
@Composable
fun GalleryCreateCard(
    title: String,
    onClick: (() -> Unit)?,
    comingSoonBadge: String? = null,
) {
    Column(
        modifier = Modifier.width(GALLERY_CARD_WIDTH_DP.dp),
    ) {
        Surface(
            shape = GALLERY_CARD_SHAPE,
            color = MaterialTheme.colorScheme.surfaceContainerHigh,
            modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(GALLERY_PREVIEW_ASPECT),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = title,
                    modifier = Modifier.size(28.dp),
                    tint = MaterialTheme.colorScheme.onSurface,
                )
                if (comingSoonBadge != null) {
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .background(Color.Black.copy(alpha = 0.5f)),
                    )
                    Text(
                        text = comingSoonBadge,
                        fontWeight = FontWeight.Bold,
                        color = Color.White,
                        style = MaterialTheme.typography.labelLarge,
                        modifier =
                            Modifier
                                .clip(CircleShape)
                                .background(Color.Black.copy(alpha = 0.7f))
                                .padding(horizontal = 12.dp, vertical = 6.dp),
                    )
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        GalleryCardTitle(title)
    }
}

@Composable
private fun GalleryCardTitle(
    title: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = title,
        fontWeight = FontWeight.SemiBold,
        color = MaterialTheme.colorScheme.onSurface,
        textAlign = TextAlign.Center,
        style = MaterialTheme.typography.labelLarge,
        modifier = modifier,
    )
}
