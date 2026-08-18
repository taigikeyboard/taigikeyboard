import java.time.LocalDate
import java.time.format.DateTimeFormatter

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("jacoco")
    id("com.diffplug.spotless")
}

spotless {
    kotlin {
        target("src/**/*.kt")
        targetExclude("**/build/**", "**/generated/**")
        ktlint("1.5.0")
    }
    kotlinGradle {
        target("*.gradle.kts")
        ktlint("1.5.0")
    }
}

jacoco {
    toolVersion = "0.8.15"
}

// 產生日期字串 (yyyyMMdd)
val buildDate: String = LocalDate.now().format(DateTimeFormatter.ofPattern("yyyyMMdd"))

android {
    namespace = "com.siansiansu.taigikeyboard"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.siansiansu.taigikeyboard"
        // Android 11+. Raised from 28 (2026-08-17): API 28/29 bundle SQLite
        // 3.22, which predates UPSERT (`ON CONFLICT … DO UPDATE`, SQLite 3.24)
        // — the syntax all three user-data DBs use on every write, wrapped in
        // fire-and-forget catches, so learning failed silently there. API 30
        // bundles 3.28, which also clears window functions (3.25). Play Console
        // at the time: Android 9 = 3 installs (<1%), Android 10 in the same
        // band, against ~869 total.
        minSdk = 30
        targetSdk = 36
        // versionCode = Unix epoch minutes — auto-monotonic, never collides
        // across test uploads (only collision risk = same-minute rebuild,
        // not realistic since one release AAB build takes >1 min).
        // Today ≈ 29_637_600, well above the previous Play floor
        // (3_050_704) and well under Play's 2_100_000_000 hard cap
        // (~5970 years headroom). versionName stays SemVer, managed manually.
        versionCode = (System.currentTimeMillis() / 60_000L).toInt()
        versionName = "3.6.5"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            debugSymbolLevel = "FULL"
            // v3.5.7 — Rust shared-core ships arm64-v8a + armeabi-v7a;
            // recovers 3,806 32-bit ARM devices flagged by Play Console
            // after the D9.2 arm64-only cut. Play 64-bit policy met.
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }

    buildFeatures {
        viewBinding = true
        buildConfig = true
        compose = true
    }

    // The in-app display-language picker is independent of the device locale. Keep every native
    // language resource in the base APK so English/Japanese remain available offline instead of
    // falling back to the default Hanji resources when Google Play omits their language splits.
    // ABI and density splitting retain their App Bundle defaults.
    bundle {
        language {
            enableSplit = false
        }
    }

    // Bundle the shared emoji set straight from the in-repo taigi-emojis data pipeline.
    // $rootDir = android/ ; the data lives at the repo root → ../taigi-emojis/dist/emoji.json
    // lands at the assets root. Single source of truth, no copied file to drift.
    sourceSets["main"].assets.srcDir(file("$rootDir/../taigi-emojis/dist"))

    buildTypes {
        debug {
            // A9 — enable unit-test coverage so Jacoco .exec data and the
            // debug class tree line up. Without this, `jacocoCoverageVerify`
            // reports 0% because the default .exec file references
            // instrumented class IDs that don't match
            // `tmp/kotlin-classes/debug`.
            enableUnitTestCoverage = true
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
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

    lint {
        disable += "MissingTranslation"
        disable += "UnusedResources"
        disable += "GradleDependency"
        disable += "OldTargetApi"
        disable += "AndroidGradlePluginVersion"
        // Skip lint on src/test/ + src/androidTest/. CI Android job otherwise
        // spends ~2 min lint-analyzing test code (lintAnalyzeDebugUnitTest +
        // lintAnalyzeDebugAndroidTest); main-source lint covers production code.
        ignoreTestSources = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// Output name: TaigiKeyboard-{versionName}-{versionCode}-{yyyyMMdd}
// Example: TaigiKeyboard-3.6.4-29637600-20260808-release.aab
base {
    archivesName.set(
        "TaigiKeyboard-${android.defaultConfig.versionName}-${android.defaultConfig.versionCode}-$buildDate",
    )
}

// Fail the build if committed i18n generated output (res/values/strings_i18n.xml + the
// i18n/generated Kotlin) is stale vs the i18n/*.json sources. A Make-only gate cannot protect
// Android Studio / archive / direct `assemble`, so this hooks into preBuild. Run `make i18n` to fix.
tasks.register<Exec>("checkI18nGenerated") {
    val repoRoot = rootProject.projectDir.parentFile
    workingDir = repoRoot
    commandLine("python3", "tools/i18n/check.py")
    // Declare inputs/outputs so Gradle skips the python run on builds that touched neither the
    // i18n sources, the generator, nor the committed generated output (otherwise it runs every build).
    inputs.dir(repoRoot.resolve("i18n"))
    inputs.file(repoRoot.resolve("tools/i18n/check.py"))
    inputs.file(repoRoot.resolve("tools/i18n/i18n_lib.py"))
    inputs.dir(layout.projectDirectory.dir("src/main/java/com/siansiansu/taigikeyboard/i18n/generated"))
    inputs.file(layout.projectDirectory.file("src/main/res/values/strings_i18n.xml"))
    outputs.upToDateWhen { true }
}

tasks.named("preBuild") {
    dependsOn("checkI18nGenerated")
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
    // core-ktx capped at 1.18.0: 1.19.0 declares minCompileSdk=37 (Android 17) in its
    // aar-metadata, which fails the build against this module's compileSdk 36. Raise the
    // cap only together with compileSdk.
    implementation("androidx.appcompat:appcompat:1.8.0")
    implementation("androidx.core:core-ktx:1.18.0")
    implementation("androidx.preference:preference-ktx:1.2.1")
    implementation("com.google.android.material:material:1.14.0")
    implementation("androidx.activity:activity-ktx:1.13.0")

    // Lifecycle（Compose 需要）
    // Held at 2.10.0: lifecycle-runtime-compose 2.11.0 declares minCompileSdk=37, and the
    // lifecycle group publishes constraints that force every artifact in it to the same
    // version — so the whole group is capped by its strictest member.
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.10.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")

    // Compose BOM（統一版本管理）
    // Capped at 2026.06.01 (compose-ui 1.11.4) for the same reason as core-ktx: the
    // 2026.08.00 BOM ships compose-ui 1.12.0 with minCompileSdk=37.
    val composeBom = platform("androidx.compose:compose-bom:2026.06.01")
    implementation(composeBom)

    // Compose 核心元件
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.compose.material:material-icons-core")

    // Compose 偵錯工具
    debugImplementation("androidx.compose.ui:ui-tooling")

    // Flexbox（現有依賴）
    implementation("com.google.android.flexbox:flexbox:3.0.0")

    // Moshi JSON
    implementation("com.squareup.moshi:moshi-kotlin:1.15.2")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")

    // DataStore
    implementation("androidx.datastore:datastore-preferences:1.2.1")

    // JUnit 單元測試
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
    // Pure-JVM SQLite for SQL-structure tests (the custom-dict cross-mode JOIN
    // query). Android's SQLiteDatabase is unavailable in JVM unit tests
    // (testOptions.unitTests.isReturnDefaultValues = true), so exercise the
    // exact production SQL string against an in-memory JDBC DB instead.
    testImplementation("org.xerial:sqlite-jdbc:3.53.2.1")
    // Real org.json for JVM unit tests — Android's bundled org.json is stubbed
    // (testOptions.unitTests.isReturnDefaultValues = true), so theme/color JSON
    // round-trip tests need the actual implementation on the test classpath.
    testImplementation("org.json:json:20260814")

    // D9.2 — Rust shared-core protobuf runtime + AndroidX test for FFI bridge.
    // Pinned to the Java artifact that pairs with the `protoc` emitting the
    // committed gencode (`libprotoc 35.1` ↔ Java `4.35.1`). Lite gencode does not
    // self-validate the runtime version, so a mismatch surfaces as a compile error
    // on symbols the older runtime lacks — keep regeneration and this bump in the
    // same commit.
    implementation("com.google.protobuf:protobuf-javalite:4.35.1")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
}

// A9 — invariant coverage gate. Runs against `testDebugUnitTest` only;
// release coverage is not required because R8 churn would drift the
// class-level numbers.
val topTenCandidateClassPatterns =
    listOf(
        "com/siansiansu/taigikeyboard/ime/dictionary/TaigiPhonetics*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/InputNormalizer*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/CandidateProcessor*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/ToneConverter*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/SuggestionCaseTransformer*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/ToneRestoration*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/TPSConverter*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/TaigiUnicode*.class",
        "com/siansiansu/taigikeyboard/ime/core/nextword/NextWordScorer*.class",
        "com/siansiansu/taigikeyboard/ime/dictionary/CustomDictionaryDerivation*.class",
    )

tasks.register<JacocoReport>("jacocoTestReport") {
    group = "verification"
    description = "Line coverage for top-10 shared-core candidates (A9 gate ≥70%)."
    dependsOn("testDebugUnitTest")

    reports {
        xml.required.set(true)
        html.required.set(true)
    }

    val buildDir = layout.buildDirectory.get().asFile
    classDirectories.setFrom(
        // AGP 8.x writes Kotlin debug classes under
        // `intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes`.
        // The old `tmp/kotlin-classes/debug` path is empty in current AGP.
        fileTree("$buildDir/intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes") {
            include(topTenCandidateClassPatterns)
        },
    )
    sourceDirectories.setFrom(files("src/main/java"))
    executionData.setFrom(
        fileTree(buildDir) {
            include(
                "jacoco/testDebugUnitTest.exec",
                "outputs/unit_test_code_coverage/debugUnitTest/*.exec",
            )
        },
    )
}

tasks.register<JacocoCoverageVerification>("jacocoCoverageVerify") {
    group = "verification"
    description = "Enforce ≥70% class-level line coverage on top-10 shared-core candidates."
    dependsOn("jacocoTestReport")

    val buildDir = layout.buildDirectory.get().asFile
    classDirectories.setFrom(
        // AGP 8.x writes Kotlin debug classes under
        // `intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes`.
        // The old `tmp/kotlin-classes/debug` path is empty in current AGP.
        fileTree("$buildDir/intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes") {
            include(topTenCandidateClassPatterns)
        },
    )
    sourceDirectories.setFrom(files("src/main/java"))
    executionData.setFrom(
        fileTree(buildDir) {
            include(
                "jacoco/testDebugUnitTest.exec",
                "outputs/unit_test_code_coverage/debugUnitTest/*.exec",
            )
        },
    )

    violationRules {
        rule {
            element = "CLASS"
            limit {
                counter = "LINE"
                value = "COVEREDRATIO"
                minimum = "0.70".toBigDecimal()
            }
        }
    }
}
