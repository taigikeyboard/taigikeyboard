package com.siansiansu.taigikeyboard.content

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

// Loads Tab1 content from bundled JSON assets.
object FeatureContentLoader {
    private const val FEATURES_ASSET = "tab1-features.json"
    private const val FAQS_ASSET = "tab1-faq.json"
    private const val JSON_KEY_FEATURES = "features"
    private const val JSON_KEY_FAQS = "faqs"
    private const val JSON_KEY_HANJI = "hanji"

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
            title = obj.getJSONObject("title").getString(JSON_KEY_HANJI),
            icon = parseIcon(obj.getJSONObject("icon")),
            summary = if (obj.has("summary")) obj.getJSONObject("summary").getString(JSON_KEY_HANJI) else null,
            paragraphs = parseParagraphs(obj.getJSONArray("paragraphs")),
        )

    private fun parseIcon(obj: JSONObject): PlatformIcon =
        PlatformIcon(
            ios = obj.getString("ios"),
            android = obj.getString("android"),
        )

    private fun parseParagraphs(array: JSONArray): List<FeatureParagraph> =
        (0 until array.length()).map { i ->
            val obj = array.getJSONObject(i)
            FeatureParagraph(
                text = obj.getJSONObject("text").getString(JSON_KEY_HANJI),
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
                    text = obj.getJSONObject("text").getString(JSON_KEY_HANJI),
                    url = obj.getString("url"),
                )
            }

            "navigation" -> {
                ParagraphAttachment.Navigation(
                    text = obj.getJSONObject("text").getString(JSON_KEY_HANJI),
                    destination = obj.getString("destination"),
                    icon = parseIcon(obj.getJSONObject("icon")),
                )
            }

            else -> {
                throw IllegalArgumentException("Unknown attachment type: $type")
            }
        }
}
