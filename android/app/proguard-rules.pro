# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# Preserve line number information for debugging stack traces
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile

# Keep Kotlin metadata for Moshi reflection
-keepattributes RuntimeVisibleAnnotations,AnnotationDefault
-keepattributes Signature
-keep class kotlin.Metadata { *; }

# Keep parameter names for constructors (required for Moshi Kotlin reflection)
-keepattributes MethodParameters

# Keep InputMethodService implementation
-keep public class * extends android.inputmethodservice.InputMethodService {
    public *;
}

# Keep all public InputMethodService methods
-keep class com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard {
    public *;
}

# Keep all preference-related classes
-keep class * extends androidx.preference.PreferenceFragmentCompat
-keepclassmembers class * extends androidx.preference.PreferenceFragmentCompat {
    public *;
}

# Keep all Activity classes
-keep public class * extends androidx.appcompat.app.AppCompatActivity {
    public *;
}

# Keep ViewBinding classes
-keep class ** implements androidx.viewbinding.ViewBinding {
    public static ** inflate(android.view.LayoutInflater);
    public static ** bind(android.view.View);
}

# Moshi - Complete rules for Kotlin reflection support
# Keep Moshi adapters
-keep class com.squareup.moshi.** { *; }
-keep interface com.squareup.moshi.** { *; }
-dontwarn com.squareup.moshi.**

# Keep Kotlin reflection for Moshi
-keep class kotlin.reflect.** { *; }
-keep class kotlin.Metadata { *; }

# Keep all model classes used with Moshi (data classes for JSON parsing)
-keep class com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard$ImeConfig { *; }
-keep class com.siansiansu.taigikeyboard.ime.core.DefaultSubtype { *; }
-keep class com.siansiansu.taigikeyboard.ime.core.Subtype { *; }
-keep class com.siansiansu.taigikeyboard.ime.text.layout.** { *; }
-keep class com.siansiansu.taigikeyboard.ime.text.key.** { *; }
-keep class com.siansiansu.taigikeyboard.ime.media.** { *; }

# Keep @Json annotated fields and their constructors
-keepclassmembers class * {
    @com.squareup.moshi.Json <fields>;
}

# Keep constructors for all data classes that might be parsed by Moshi
-keepclassmembers class com.siansiansu.taigikeyboard.** {
    <init>(...);
}

# Keep names of @Json fields
-keepclassmembernames class com.siansiansu.taigikeyboard.** {
    @com.squareup.moshi.Json <fields>;
}

# Kotlin Coroutines
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}
-keepclassmembers class kotlinx.coroutines.** {
    volatile <fields>;
}

# Keep custom View classes
-keep public class * extends android.view.View {
    public <init>(android.content.Context);
    public <init>(android.content.Context, android.util.AttributeSet);
    public <init>(android.content.Context, android.util.AttributeSet, int);
}

# Preserve annotations
-keepattributes *Annotation*

# Remove logging in release builds
-assumenosideeffects class android.util.Log {
    public static *** d(...);
    public static *** v(...);
    public static *** i(...);
    public static *** w(...);
}
