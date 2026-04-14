package com.siansiansu.taigikeyboard.model

import android.content.Context
import com.siansiansu.taigikeyboard.localization.LocalizedText
import org.json.JSONArray
import org.json.JSONObject

// Loads Tab1 content from bundled JSON assets.
object FeatureContentLoader {
    @Volatile private var cachedFeatures: List<FeatureContent>? = null

    @Volatile private var cachedFAQs: List<FeatureContent>? = null

    fun loadFeatures(context: Context): List<FeatureContent> {
        cachedFeatures?.let { return it }
        val json =
            context.assets
                .open("tab1-features.json")
                .bufferedReader()
                .use { it.readText() }
        val features = parseContentArray(JSONObject(json), "features")
        cachedFeatures = features
        return features
    }

    fun loadFAQs(context: Context): List<FeatureContent> {
        cachedFAQs?.let { return it }
        val json =
            context.assets
                .open("tab1-faq.json")
                .bufferedReader()
                .use { it.readText() }
        val faqs = parseContentArray(JSONObject(json), "faqs")
        cachedFAQs = faqs
        return faqs
    }

    private fun parseContentArray(
        root: JSONObject,
        key: String,
    ): List<FeatureContent> {
        val array = root.getJSONArray(key)
        return (0 until array.length()).map { parseFeature(array.getJSONObject(it)) }
    }

    private fun parseFeature(obj: JSONObject): FeatureContent {
        val iconObj = obj.getJSONObject("icon")
        return FeatureContent(
            id = obj.getString("id"),
            title = parseLocalizedText(obj.getJSONObject("title")),
            icon =
                PlatformIcon(
                    ios = iconObj.getString("ios"),
                    android = iconObj.getString("android"),
                ),
            summary = if (obj.has("summary")) parseLocalizedText(obj.getJSONObject("summary")) else null,
            paragraphs = parseParagraphs(obj.getJSONArray("paragraphs")),
        )
    }

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
                    interval = obj.optDouble("interval", 2.0),
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
                val iconObj = obj.getJSONObject("icon")
                ParagraphAttachment.Navigation(
                    text = parseLocalizedText(obj.getJSONObject("text")),
                    destination = obj.getString("destination"),
                    icon =
                        PlatformIcon(
                            ios = iconObj.getString("ios"),
                            android = iconObj.getString("android"),
                        ),
                )
            }

            else -> {
                throw IllegalArgumentException("Unknown attachment type: $type")
            }
        }

    private fun parseLocalizedText(obj: JSONObject): LocalizedText {
        val hanji = obj.getString("hanji")
        return LocalizedText(
            hanji = hanji,
            poj = obj.optString("poj", hanji),
            tl = obj.optString("tl", hanji),
        )
    }
}
