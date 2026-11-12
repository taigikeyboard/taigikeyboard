---
name: dogfood
description: Give manual test cases for a just-fixed bug, built from random real sentences in corpus/taigi-typing. Use after a fix lands or before a dogfood pass, when you want something real to type on a device. Fast and read-only — greps the corpus, never builds, runs tests or edits files. Args: a description of the sentences wanted ("khinsiann after a verb", "reduplication", "放重利"), a TL form, or nothing (the fix at HEAD decides).
disable-model-invocation: false
context: fork
model: haiku
---

# Dogfood sentences

```sh
python3 .claude/skills/dogfood/pick.py '<pattern>' [count]   # count = 5
```

`pattern` is a case-insensitive regex matched against the TL **and** the hanji.
Prints random 教典 sentences: hanji, TL, keystrokes, `dic_url`.

1. **Pattern.** Translate the user's description into one regex — that is your job,
   the script only greps. No argument → `git show --stat HEAD` + its commit message
   gives the typed form the fix changed. Examples:

   | Description | pattern |
   |---|---|
   | khinsiann (neutral-tone) boundary | `--` · `--lâi` |
   | hyphen inside a word | `-tāng-` · `siong-` |
   | reduplication | `(\w+)-\1` |
   | nasalization | `nn|ⁿ` |
   | one word / Hanji | `放重利` |
   | long sentence | `.{40,}` |

   No hit → widen (`pang-tang-lai` → `-tang-`).
2. **Pick.** Run the script. Take 2–4 sentences; keep the whole sentence's keys but
   test the span that carries the form.
3. **Print.** One block each, nothing else:

```
N. <hanji sentence>
   拍: <keys>  ·  期待: <what the fix renders>  ·  控制: <near-miss keys> → <unchanged>
   <dic_url>
```

Expected value comes from the commit message / `docs/architecture/behavioral-invariants.md §n`.
Not stated there → say so, don't guess.

Engine diff → first line: `make build` → `make -C macos install` (Windows reinstall,
mobile rebuild). 教典 sentences are CC BY-ND: quote verbatim.
