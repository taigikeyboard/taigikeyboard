// Process-wide decoded theme-photo cache, keyed by the theme image file name.

package com.siansiansu.taigikeyboard.ime.core

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.util.LruCache
import androidx.annotation.MainThread
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext

/** Which decode of a theme photo a render site needs. Mirrors iOS ThemeImageVariant. */
enum class ThemeImageVariant {
    /** The stored 1280 px photo — the keyboard and the editor preview. */
    FULL,

    /** A downsample to [THUMBNAIL_LONG_EDGE] — theme cards and row thumbnails. */
    THUMBNAIL,
    ;

    companion object {
        /** Longest edge of a thumbnail decode — covers a theme card at 3×. */
        const val THUMBNAIL_LONG_EDGE = 480
    }
}

/**
 * Decodes theme photos from the [ThemeImageStore] directory OFF the main thread and keeps them
 * decoded as hardware bitmaps (pixels in GPU memory, not the Java heap; both the View
 * `drawBitmap` and Compose `drawImage` paths accept it). Render sites read [cached] (lookup
 * only, no I/O) and fall back to [load], which decodes on [Dispatchers.IO]; concurrent loads of
 * one photo share a single decode. Full photos and thumbnails are cached separately; the full
 * cache holds two (the keyboard's + the editor's — the app and the IME share this
 * process). A replaced photo gets a new file name, so a stale entry is never served. One
 * instance lives on `CompositionRoot`. Mirrors iOS ThemeImageCache.
 */
class ThemeImageCache(
    val store: ThemeImageStore,
) {
    // Counted, not sized: every stored photo is capped at MAX_LONG_EDGE (~6.5 MB decoded), so two
    // entries also bound the bytes. Thumbnails vary in size and are sized by bytes.
    private val fullCache = LruCache<String, Bitmap>(FULL_COUNT_LIMIT)
    private val thumbnailCache =
        object : LruCache<String, Bitmap>(THUMBNAIL_BYTES_LIMIT) {
            override fun sizeOf(
                key: String,
                value: Bitmap,
            ): Int = value.byteCount
        }

    // Main-thread only (see [load]); the decode itself runs on IO.
    private val inFlight = HashMap<Pair<String, ThemeImageVariant>, CompletableDeferred<Bitmap?>>()

    /** The already-decoded photo, or null. Never touches the disk. */
    fun cached(
        file: String,
        variant: ThemeImageVariant,
    ): Bitmap? = cache(variant).get(file)

    /**
     * The decoded photo, decoding it on [Dispatchers.IO] on a miss; null when the file is
     * missing / undecodable. The first caller runs the decode; concurrent callers await it. The
     * decode + publish run [NonCancellable] (tens of ms) so a caller leaving composition still
     * fills the cache and releases the waiters — no cache-owned scope needed.
     */
    @MainThread
    suspend fun load(
        file: String,
        variant: ThemeImageVariant,
    ): Bitmap? {
        cached(file, variant)?.let { return it }
        val key = file to variant
        inFlight[key]?.let { return it.await() }
        val pending = CompletableDeferred<Bitmap?>()
        inFlight[key] = pending
        return withContext(NonCancellable) {
            val decoded = withContext(Dispatchers.IO) { decode(file, variant) }
            // Back on the caller's (main) dispatcher: [inFlight] stays main-confined.
            decoded?.let { cache(variant).put(file, it) }
            inFlight.remove(key)
            pending.complete(decoded)
            decoded
        }
    }

    /**
     * Removes every stored photo no theme in [themes] references and drops their decoded
     * bitmaps. Wired as the store's mutation hook and run when the editor closes.
     */
    fun sweep(themes: List<UserTheme>) {
        val referenced = themes.referencedPhotoFiles()
        store.sweep(referenced)
        for (cache in listOf(fullCache, thumbnailCache)) {
            cache
                .snapshot()
                .keys
                .filter { it !in referenced }
                .forEach { cache.remove(it) }
        }
    }

    private fun cache(variant: ThemeImageVariant): LruCache<String, Bitmap> =
        when (variant) {
            ThemeImageVariant.FULL -> fullCache
            ThemeImageVariant.THUMBNAIL -> thumbnailCache
        }

    private fun decode(
        file: String,
        variant: ThemeImageVariant,
    ): Bitmap? {
        val path = store.file(file)
        return when (variant) {
            ThemeImageVariant.FULL -> {
                val options = BitmapFactory.Options().apply { inPreferredConfig = Bitmap.Config.HARDWARE }
                BitmapFactory.decodeFile(path.path, options)
            }
            ThemeImageVariant.THUMBNAIL ->
                try {
                    ImageDecoder.decodeBitmap(ImageDecoder.createSource(path)) { decoder, info, _ ->
                        val (width, height) =
                            ThemeImageStore.targetSize(info.size.width, info.size.height, ThemeImageVariant.THUMBNAIL_LONG_EDGE)
                        decoder.setTargetSize(width, height)
                    }
                } catch (e: Exception) {
                    null
                }
        }
    }

    companion object {
        private const val FULL_COUNT_LIMIT = 2
        private const val THUMBNAIL_BYTES_LIMIT = 8 * 1024 * 1024
    }
}

/**
 * The decoded theme photo for [file] (null file, missing file, or not decoded yet → null). A
 * cache hit is returned on the first composition; a miss decodes off the main thread and
 * recomposes when it lands. Mirrors iOS ThemePhotoImage.
 */
@Composable
fun rememberThemePhoto(
    file: String?,
    variant: ThemeImageVariant,
): Bitmap? {
    val context = LocalContext.current
    val cache = remember(context) { CompositionRoot.shared(context).themeImages }
    var bitmap by remember(file, variant) { mutableStateOf(file?.let { cache.cached(it, variant) }) }
    LaunchedEffect(file, variant) {
        if (bitmap == null && file != null) bitmap = cache.load(file, variant)
    }
    return bitmap
}
