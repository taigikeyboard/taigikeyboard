package com.siansiansu.taigikeyboard.content

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

// Loads Home tab content from bundled JSON assets.
object FeatureContentLoader {
    private const val FEATURES_ASSET = "features.json"
    private const val FAQS_ASSET = "faq.json"
    private const val JSON_KEY_FEATURES = "features"
    private const val JSON_KEY_FAQS = "faqs"
    private const val JSON_KEY_HANJI = "hanji"
    private const val JSON_KEY_TAILO = "tailo"
    private const val JSON_KEY_POJ = "poj"
    private const val JSON_KEY_JA = "ja"
    private const val JSON_KEY_EN = "en"

    @Volatile private var cachedFeatures: List<FeatureContent>? = null

    @Volatile private var cachedFAQs: List<FeatureContent>? = null

    fun loadFeatures(context: Context): List<FeatureContent> {
        cachedFeatures?.let { return it }
        return loadAndParseAsset(context, FEATURES_ASSET, JSON_KEY_FEATURES)
            .also { cachedFeatures = it }
    }

    fun loadFAQs(context: Context): List<FeatureContent> {
        cachedFAQs?.let { return it }
        return loadAndParseAsset(context, FAQS_ASSET, JSON_KEY_FAQS)
            .also { cachedFAQs = it }
    }

    private fun loadAndParseAsset(
        context: Context,
        assetFileName: String,
        rootKey: String,
    ): List<FeatureContent> {
        val json =
            context.assets
                .open(assetFileName)
                .bufferedReader()
                .use { it.readText() }
        return parseContentArray(JSONObject(json), rootKey)
    }

    private fun parseContentArray(
        root: JSONObject,
        key: String,
    ): List<FeatureContent> {
        val array = root.getJSONArray(key)
        return (0 until array.length()).map { parseFeature(array.getJSONObject(it)) }
    }

    private fun parseFeature(obj: JSONObject): FeatureContent =
        FeatureContent(
            id = obj.getString("id"),
            title = parseLocalizedText(obj.getJSONObject("title")),
            icon = parseIcon(obj.getJSONObject("icon")),
            summary = if (obj.has("summary")) parseLocalizedText(obj.getJSONObject("summary")) else null,
            paragraphs = parseParagraphs(obj.getJSONArray("paragraphs")),
        )

    // Parses a localized-text object: hanji is required; the other languages are absent until C2
    // authoring fills them.
    private fun parseLocalizedText(obj: JSONObject): LocalizedContentText =
        LocalizedContentText(
            hanji = obj.getString(JSON_KEY_HANJI),
            tailo = obj.optStringOrNull(JSON_KEY_TAILO),
            poj = obj.optStringOrNull(JSON_KEY_POJ),
            ja = obj.optStringOrNull(JSON_KEY_JA),
            en = obj.optStringOrNull(JSON_KEY_EN),
        )

    // Returns the authored value (incl. an explicit empty string) or null when the key is absent/JSON
    // null. NOT optString — optString collapses absent and "" to "", losing the unauthored signal a
    // language fallback needs. Extension form mirrors the JSONObject.optIntOrNull / optFloatOrNull idiom.
    private fun JSONObject.optStringOrNull(key: String): String? =
        if (has(key) && !isNull(key)) getString(key) else null

    private fun parseIcon(obj: JSONObject): PlatformIcon =
        PlatformIcon(
            ios = obj.getString("ios"),
            android = obj.getString("android"),
        )

    private fun parseParagraphs(array: JSONArray): List<FeatureParagraph> =
        (0 until array.length()).map { i ->
            val obj = array.getJSONObject(i)
            FeatureParagraph(
                text = parseLocalizedText(obj.getJSONObject("text")),
                attachment = if (obj.has("attachment")) parseAttachment(obj.getJSONObject("attachment")) else null,
            )
        }

    private fun parseAttachment(obj: JSONObject): ParagraphAttachment =
        when (val type = obj.getString("type")) {
            "slideshow" -> {
                val images = obj.getJSONArray("images")
                ParagraphAttachment.Slideshow(
                    images = (0 until images.length()).map { images.getString(it) },
                    interval = obj.optDouble("interval", DEFAULT_SLIDESHOW_INTERVAL_SECONDS),
                )
            }

            "image" -> {
                ParagraphAttachment.Image(name = obj.getString("name"))
            }

            "link" -> {
                ParagraphAttachment.Link(
                    text = parseLocalizedText(obj.getJSONObject("text")),
                    url = obj.getString("url"),
                )
            }

            "navigation" -> {
                ParagraphAttachment.Navigation(
                    text = parseLocalizedText(obj.getJSONObject("text")),
                    destination = obj.getString("destination"),
                    icon = parseIcon(obj.getJSONObject("icon")),
                )
            }

            else -> {
                throw IllegalArgumentException("Unknown attachment type: $type")
            }
        }
}
