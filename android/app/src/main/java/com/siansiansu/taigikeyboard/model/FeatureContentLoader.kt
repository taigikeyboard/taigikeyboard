package com.siansiansu.taigikeyboard.model

import android.content.Context
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
            title = obj.getJSONObject("title").getString("hanji"),
            icon =
                PlatformIcon(
                    ios = iconObj.getString("ios"),
                    android = iconObj.getString("android"),
                ),
            summary = if (obj.has("summary")) obj.getJSONObject("summary").getString("hanji") else null,
            paragraphs = parseParagraphs(obj.getJSONArray("paragraphs")),
        )
    }

    private fun parseParagraphs(array: JSONArray): List<FeatureParagraph> =
        (0 until array.length()).map { i ->
            val obj = array.getJSONObject(i)
            FeatureParagraph(
                text = obj.getJSONObject("text").getString("hanji"),
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
                    text = obj.getJSONObject("text").getString("hanji"),
                    url = obj.getString("url"),
                )
            }

            "navigation" -> {
                val iconObj = obj.getJSONObject("icon")
                ParagraphAttachment.Navigation(
                    text = obj.getJSONObject("text").getString("hanji"),
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
}
