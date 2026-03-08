import java.time.LocalDate
import java.time.format.DateTimeFormatter

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

// 產生日期字串 (yyyyMMdd)
val buildDate: String = LocalDate.now().format(DateTimeFormatter.ofPattern("yyyyMMdd"))

android {
    namespace = "com.siansiansu.taigikeyboard"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.siansiansu.taigikeyboard"
        minSdk = 28
        targetSdk = 35
        versionCode = 343
        versionName = "3.4.3"

        ndk {
            debugSymbolLevel = "FULL"
            // 支援的 CPU 架構
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }

        // CMake 設定
        externalNativeBuild {
            cmake {
                cppFlags += "-std=c++17"
            }
        }
    }

    // Native build 設定
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    buildFeatures {
        viewBinding = true
        buildConfig = true
        compose = true
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // 確保 release 建置包含完整符號檔供 Google Play 使用
            ndk {
                debugSymbolLevel = "FULL"
            }
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// 設定輸出檔名：TaigiKeyboard-{versionName}-{yyyyMMdd}
// 例如：TaigiKeyboard-3.3.9-20251231-release.aab
base {
    archivesName.set("TaigiKeyboard-${android.defaultConfig.versionName}-$buildDate")
}

// Task 用於顯示目前的 versionCode（用於驗證）
tasks.register("printVersionCode") {
    doLast {
        val versionCode = android.defaultConfig.versionCode
        val versionName = android.defaultConfig.versionName
        println("==================================")
        println("Current versionCode: $versionCode")
        println("Current versionName: $versionName")
        println("==================================")
    }
}

dependencies {
    // AndroidX 核心
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("androidx.preference:preference-ktx:1.2.1")
    implementation("com.google.android.material:material:1.13.0")
    implementation("androidx.activity:activity-ktx:1.12.4")

    // Lifecycle（Compose 需要）
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.10.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")

    // Compose BOM（統一版本管理）
    val composeBom = platform("androidx.compose:compose-bom:2026.01.01")
    implementation(composeBom)

    // Compose 核心元件
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.activity:activity-compose:1.12.4")
    implementation("androidx.compose.material:material-icons-extended")

    // Compose 偵錯工具
    debugImplementation("androidx.compose.ui:ui-tooling")

    // Flexbox（現有依賴）
    implementation("com.google.android.flexbox:flexbox:3.0.0")

    // Moshi JSON
    implementation("com.squareup.moshi:moshi-kotlin:1.15.2")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")

    // DataStore
    implementation("androidx.datastore:datastore-preferences:1.2.0")

    // Kotlinx Serialization
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.10.0")

    // JUnit 單元測試
    testImplementation("junit:junit:4.13.2")
}
