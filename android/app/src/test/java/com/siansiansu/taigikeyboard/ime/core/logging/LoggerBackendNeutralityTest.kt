package com.siansiansu.taigikeyboard.ime.core.logging

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.File

/**
 * Pins `docs/architecture/behavioral-invariants.md` §12 — every file
 * carrying the `// region Shared-Core Candidate` marker must route
 * logging through [LoggerBackend] only. Concrete platform loggers
 * (`android.util.Log`) must not appear in candidate source.
 *
 * Implemented as a filesystem-walk test rather than runtime reflection
 * so that the source-level import contract is checked directly — at
 * runtime, unused imports would be stripped by R8 and invisible to a
 * reflective scan.
 */
class LoggerBackendNeutralityTest {
    private val forbiddenImportPrefixes =
        listOf(
            "android.",
            "androidx.",
            "kotlinx.coroutines.",
            "com.google.android.",
            "com.squareup.moshi.",
            "android.database.",
            "android.content.",
            "android.util.",
            "android.os.",
        )

    // Kotlin stdlib + JDK base packages are allowed.
    private val allowedForbiddenlikeImports =
        listOf<String>()

    /**
     * Walks the main source tree, finds every file starting with the
     * `// region Shared-Core Candidate` header, and asserts it imports
     * none of the platform-only packages listed above. Fails loudly if
     * the marker sweep is stale (zero candidates found).
     */
    @Test
    fun test_INVARIANT_candidates_only_depend_on_logger_backend_protocol() {
        val srcRoot = locateMainSourceRoot()
        val candidates = collectSharedCoreCandidates(srcRoot)
        assertTrue(
            "filesystem walk must find Shared-Core Candidate files (roster stale?)",
            candidates.isNotEmpty(),
        )

        val violations = mutableListOf<String>()
        for (file in candidates) {
            val lines = file.readLines()
            for ((lineIndex, line) in lines.withIndex()) {
                val trimmed = line.trim()
                if (!trimmed.startsWith("import ")) continue
                val importPath = trimmed.removePrefix("import ").removeSuffix(";").trim()
                if (allowedForbiddenlikeImports.any { importPath.startsWith(it) }) continue
                val offender =
                    forbiddenImportPrefixes.firstOrNull { importPath.startsWith(it) }
                if (offender != null) {
                    violations += "${file.name}:${lineIndex + 1}  import $importPath"
                }
            }
        }

        if (violations.isNotEmpty()) {
            fail(
                "Shared-Core Candidate files must only depend on LoggerBackend for logging. " +
                    "Found forbidden imports:\n" + violations.joinToString("\n"),
            )
        }
    }

    /**
     * Candidate files that accept a [LoggerBackend] parameter MUST default
     * it to [NullLoggerBackend]. The default-to-null pattern is what
     * `rules/android-guidelines.md` §1.6 requires so shared-core call
     * paths that don't wire logging stay silent rather than depending on
     * a platform-specific backend.
     */
    @Test
    fun test_INVARIANT_null_logger_is_the_default_factory() {
        val srcRoot = locateMainSourceRoot()
        val candidates = collectSharedCoreCandidates(srcRoot)
        val violations = mutableListOf<String>()

        // Match publicly-visible `fun <name>(...)` declarations carrying a
        // `logger: LoggerBackend` parameter. Internal helpers marked
        // `private fun` / `internal fun` are excluded — they are never
        // reached from outside the candidate module and the null-logger
        // contract is about the PUBLIC surface area.
        val publicFunWithLogger =
            Regex(
                """(?s)(?<vis>(?:public\s+)?)fun\s+(?<name>\w+)\s*\([^)]*\blogger\s*:\s*LoggerBackend(?<suffix>[^,)]*)""",
            )
        for (file in candidates) {
            val contents = file.readText()
            for (match in publicFunWithLogger.findAll(contents)) {
                // Ensure we didn't land inside a `private`/`internal` modifier
                // by peeking 16 characters back at the match start.
                val startIndex = match.range.first
                val contextBefore =
                    contents.substring(maxOf(0, startIndex - 16), startIndex)
                if (contextBefore.contains("private") || contextBefore.contains("internal")) continue

                val name = match.groups["name"]?.value ?: "?"
                val suffix = match.groups["suffix"]?.value.orEmpty()
                if ("=" !in suffix) {
                    violations += "${file.name}: public fun $name — `LoggerBackend` parameter missing default"
                    continue
                }
                val defaultExpr =
                    suffix
                        .substringAfter('=')
                        .trim()
                        .trimEnd(',', ')')
                        .trim()
                if (defaultExpr != "NullLoggerBackend") {
                    violations +=
                        "${file.name}: public fun $name — LoggerBackend default is `$defaultExpr` (must be NullLoggerBackend)"
                }
            }
        }

        assertFalse(
            "Shared-Core Candidate files must default logger: LoggerBackend = NullLoggerBackend. " +
                "Violations:\n" + violations.joinToString("\n"),
            violations.isNotEmpty(),
        )
    }

    /**
     * Run from either the repo root or the `android/` / `android/app`
     * subdirectory — tests launched by Gradle typically set `user.dir`
     * to the project module, while tests launched from IDE run-configs
     * sometimes start at the repo root.
     */
    private fun locateMainSourceRoot(): File {
        val candidates =
            listOf(
                "src/main/java",
                "app/src/main/java",
                "android/app/src/main/java",
            )
        for (rel in candidates) {
            val f = File(rel)
            if (f.isDirectory) return f.absoluteFile
        }
        fail("Could not locate `src/main/java` from working dir ${File(".").absolutePath}")
        error("unreachable")
    }

    private fun collectSharedCoreCandidates(root: File): List<File> =
        root
            .walkTopDown()
            .filter { it.isFile && it.extension == "kt" }
            .filter { file ->
                // The marker must appear near the top of the file to count.
                file.bufferedReader().useLines { seq ->
                    seq.take(5).any { it.trim() == "// region Shared-Core Candidate" }
                }
            }.toList()
}
