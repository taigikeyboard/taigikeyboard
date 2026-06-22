# Round C2 — in-app content i18n draft review sheet (2026-06-22)

**Status: REVIEW-PENDING. Mechanical/translated first pass — NOT yet linguistically verified.**

## Why this exists

Round C1 made the in-app help content (`content/tab1-features.json` + `tab1-faq.json`) display-language-aware (`LocalizedContentText`, hanji + optional tailo/poj/en/ja, fallback hanji). C2 authors those 4 extra languages for all **80** localizable strings so the Home/FAQ content renders in the user's chosen display language. tailo/poj are **DEBUG-selectable only** and ride the same REVIEW-PENDING gate as the i18n namespace drafts (`2026-06-22-i18n-tl-draft-review.md` / `-i18n-poj-draft-review.md`); en/ja are also drafts pending review. **No promotion / release before this sheet is reviewed.**

## How the draft was produced

- **Tâi-lô (CP#3 — authoritative-source-only)**: mechanical greedy longest-match segmentation over the project's MOE-derived `dictionary/output/dictionary.csv` (`hanzi → tl`, `kautian_main`, frequency). NO reading was invented. Latin / numeric / 方音符號 (ㄅㄆㄇ…) / punctuation pass through verbatim. The same drafter as the i18n labels (`/tmp/draft_tailo.py`), walking the content tree.
- **POJ**: DERIVED from the drafted tailo via the canonical taigi-converter bridge (`make content-derive-poj` → `tools/i18n/derive_content_poj.py`). Deterministic transliteration; **do NOT hand-edit poj** — fix the `tailo` then re-derive (`make content-derive-poj`, lockstep).
- **English / 日本語**: hand-translated from the hanji source (drafts, review-pending). Glossary: 齒盤=keyboard/キーボード, 白話字=Pe̍h-ōe-jī (POJ), 台羅=Tâi-lô (TL), 方音符號=Taiwanese Phonetic Symbols (TPS)/方音符号, 漢字=Hanji/漢字, 候選詞=candidate/候補, 詞庫/辭典=dictionary/辞書, 教育部=Ministry of Education/教育部.

## Provenance

- `dictionary/output/dictionary.csv` SHA-256: `9bcfb42fc636b8fd3887317c43ec5711e849fdebf213939a7ff83a55fc4eaf1f`
- taigi-converter: `77dbef01c990acdad6d68695596b6eefbf999495 (taigi-converter v0.1.4-6-g77dbef0)`
- Generated 2026-06-22. Re-run `make content-derive-poj` after any tailo correction; re-run `make content-poj-test` to gate well-formed/complete/canonical content.

## ⚠ Review gate

- **Review ALL 80 strings, not only LOW/MED.** Full-sentence segmentation, context 多音 (一字多音), word boundaries, compound hyphenation and tone-sandhi are NOT resolved mechanically — a HIGH row can still be wrong. Tâi-lô confidence is markedly rougher than the short i18n labels because these are full sentences.
- Known systematic draft errors observed (examples, not exhaustive): greedy match grabbing a Mandarin-loan compound (`若干` → `jio̍k-kan`, should be `若`+`干焦` kan-na); context 多音 (`重拍` → `tāng`, should be `tîng`); mis-segmentation of `全羅馬字` and `好了後`. Fix readings in the source (`content/*.json` `tailo`), then `make content-derive-poj`.
- en/ja are translation drafts — verify terminology + nuance.

## Metalinguistic-literal protection (poj)

A few strings TEACH romanization and embed literal spelling/keystroke examples the TL→POJ converter would wrongly transliterate (e.g. `「oo」`→`「o͘」` erases the TL-vs-POJ contrast; `'gau5-tsa2'`→`'gâu-chá'`). These spans are kept verbatim in the derived POJ via `POJ_PROTECT` in `derive_content_poj.py` (path-keyed, minimal; spelling examples protected in their QUOTED form so a bare `oo` cannot freeze the `oo` inside a real word like `tsoo`). Protected: `features[inputModes].paragraphs[1].text` (`「oo」`); `features[romanInput].paragraphs[3].text` (`'oo'`); `features[romanInput].paragraphs[5].text` (`'gau5-tsa2'`, `'gau5tsa2'`); `features[accuracyTips].paragraphs[0].text` (`gau5tsa2`); `features[accuracyTips].paragraphs[1].text` (`phang1`). Confirm during review that these are the only literals needing protection.

## Tâi-lô confidence distribution

`{'LOW': 45, 'MED': 31, 'HIGH': 4}` (HIGH = clean single-reading match; MED = multi-`kautian_main` word, 正音-leaning pick; LOW = single-char fallback used somewhere). MED/LOW = needs-eyeball, NOT necessarily wrong.

## Flag legend

- `X=>r:AMB` — word X had multiple `kautian_main` readings; draft picked `r`.
- `X=>r:1char` — single-char fallback (no compound dict entry matched); may need compound hyphenation.
- `X:MISS` — not in dictionary.

---


## `inputModes`

### `features[inputModes].title` — Tâi-lô conf: LOW

- 漢字: 4 種拍字方式
- Tâi-lô: 4 tsíng phah-jī hong-sik
- POJ: 4 chéng phah-jī hong-sek
- English: 4 input methods
- 日本語: 4種類の入力方式
- flags: 種=>tsíng:1char; 拍字=>phah-jī:AMB

### `features[inputModes].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 台語齒盤有 4 種拍字方式：「白話字」、「台羅」、「方音符號」佮「英文」，佇齒盤面頂工具列通切換。
- Tâi-lô: tâi-gí khí-puânn ū 4 tsíng phah-jī hong-sik：「pe̍h-uē-jī」、「tâi-lô」、「hong-im-hû-hō」 kah「ing-bûn」， tī khí-puânn bīn-tíng kang-khū-lia̍t thang tshiat-uānn。
- POJ: tâi-gí khí-pôaⁿ ū 4 chéng phah-jī hong-sek：「pe̍h-ōe-jī」、「tâi-lô」、「hong-im-hû-hō」 kah「eng-bûn」， tī khí-pôaⁿ bīn-téng kang-khū-lia̍t thang chhiat-ōaⁿ。
- English: Taigi Keyboard has 4 input methods: Pe̍h-ōe-jī, Tâi-lô, Taiwanese Phonetic Symbols, and English. You can switch between them on the toolbar at the top of the keyboard.
- 日本語: 台語キーボードには「白話字」「台羅」「方音符号」「英語」の4つの入力方式があり、キーボード上部のツールバーで切り替えられます。
- flags: 台語=>tâi-gí:AMB; 有=>ū:1char; 種=>tsíng:1char; 拍字=>phah-jī:AMB; 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 佇=>tī:1char; 工具列=>kang-khū-lia̍t:AMB; 通=>thang:1char

### `features[inputModes].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 「白話字」（POJ）佮「台羅」（TL）攏是羅馬字，差別佇聲調符號佮部份拼法無仝款，譬論「ch」（白話字）佮「ts」（台羅），「o͘」（白話字）佮「oo」（台羅）。若已經慣勢 1 種著用彼種，無特別 ê 偏好用台羅著好，因為台羅是目前教育部 ê 正式標準。
- Tâi-lô: 「pe̍h-uē-jī」（POJ） kah「tâi-lô」（TL） lóng sī lô-má-jī， tsha-pia̍t tī siann-tiāu hû-hō kah pōo-hūn ping-huat bô-kāng-khuán， phì-lūn「ch」（pe̍h-uē-jī） kah「ts」（tâi-lô），「o͘」（pe̍h-uē-jī） kah「oo」（tâi-lô）。 nā í-king kuàn-sì 1 tsíng-tio̍h iōng hit tsióng， bô ti̍k-pia̍t ê phian-hó iōng tâi-lô tio̍h hó， in-uī tâi-lô sī ba̍k-tsîng kàu-io̍k-pōo ê tsìng-sik phiau-tsún。
- POJ: 「pe̍h-ōe-jī」（POJ） kah「tâi-lô」（TL） lóng sī lô-má-jī， chha-pia̍t tī siaⁿ-tiāu hû-hō kah pō͘-hūn peng-hoat bô-kāng-khoán， phì-lūn「ch」（pe̍h-ōe-jī） kah「ts」（tâi-lô），「o͘」（pe̍h-ōe-jī） kah「oo」（tâi-lô）。 nā í-keng koàn-sì 1 chéng-tio̍h iōng hit chióng， bô te̍k-pia̍t ê phian-hó iōng tâi-lô tio̍h hó， in-ūi tâi-lô sī ba̍k-chêng kàu-io̍k-pō͘ ê chèng-sek phiau-chún。
- English: Both Pe̍h-ōe-jī (POJ) and Tâi-lô (TL) are romanizations; they differ in their tone marks and some spellings, e.g. “ch” (POJ) vs “ts” (TL), and “o͘” (POJ) vs “oo” (TL). If you are already used to one, use that one; with no particular preference, Tâi-lô is a good default, as it is the current official standard of the Ministry of Education.
- 日本語: 「白話字」（POJ）と「台羅」（TL）はどちらもローマ字表記で、声調記号と一部の綴りが異なります。例えば「ch」（白話字）と「ts」（台羅）、「o͘」（白話字）と「oo」（台羅）です。すでにどちらかに慣れているならそれを、特にこだわりがなければ台羅をおすすめします。台羅は現在の教育部の正式な標準だからです。
- flags: 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 羅馬字=>lô-má-jī:AMB; 佇=>tī:1char; 佮=>kah:1char; 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 若=>nā:1char; 用=>iōng:1char; 無=>bô:1char; 用=>iōng:1char; 著=>tio̍h:1char; 好=>hó:1char; 是=>sī:1char; 目前=>ba̍k-tsîng:AMB; 標準=>phiau-tsún:AMB

### `features[inputModes].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 「方音符號」（TPS）類似注音輸入法，民間嘛有人講「台語注音」，用台語方音符號拍字，會切去使用專用齒盤佈局。拍好符號了後，齒盤會自動轉做台羅去揣字，所以辭典結果佮台羅模式仝款。
- Tâi-lô: 「hong-im-hû-hō」（TPS） luī-sū tsù-im-su-ji̍p huat， bîn-kan mā-ū lâng-kóng「tâi-gí tsù-im」， iōng tâi-gí hong-im-hû-hō phah-jī， huē tshiat khì sú-iōng tsuan-iōng khí-puânn pòo-kio̍k。 phah hó hû-hō liáu-āu， khí-puânn huē tsū-tōng tńg tsuè tâi-lô khì tshuē jī， sóo-í sû-tián kiat-kó kah tâi-lô bôo-sik kāng-khuán。
- POJ: 「hong-im-hû-hō」（TPS） lūi-sū chù-im-su-ji̍p hoat， bîn-kan mā-ū lâng-kóng「tâi-gí chù-im」， iōng tâi-gí hong-im-hû-hō phah-jī， hōe chhiat khì sú-iōng choan-iōng khí-pôaⁿ pò͘-kio̍k。 phah hó hû-hō liáu-āu， khí-pôaⁿ hōe chū-tōng tńg chòe tâi-lô khì chhōe jī， só͘-í sû-tián kiat-kó kah tâi-lô bô͘-sek kāng-khoán。
- English: Taiwanese Phonetic Symbols (TPS) is similar to Zhuyin input, also informally called “Taiwanese Zhuyin.” You type with Taiwanese phonetic symbols on a dedicated keyboard layout. After you enter the symbols, the keyboard automatically converts them to Tâi-lô to look up characters, so the dictionary results are the same as in Tâi-lô mode.
- 日本語: 「方音符号」（TPS）は注音入力に似ており、民間では「台語注音」とも呼ばれます。専用のキーボード配列で台湾語の方音符号を入力します。符号を入力すると、キーボードが自動的に台羅へ変換して字を検索するため、辞書の結果は台羅モードと同じです。
- flags: 法=>huat:1char; 台語=>tâi-gí:AMB; 用=>iōng:1char; 台語=>tâi-gí:AMB; 拍字=>phah-jī:AMB; 會=>huē:1char; 切=>tshiat:1char; 去=>khì:1char; 拍=>phah:1char; 好=>hó:1char; 會=>huē:1char; 轉=>tńg:1char; 做=>tsuè:1char; 去=>khì:1char; 揣=>tshuē:1char; 字=>jī:1char; 佮=>kah:1char

### `features[inputModes].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 「英文」模式是一般 ê 英文齒盤，干焦基本 ê 拼音檢查爾，拍英文 ê 時陣切過來用較方便，袂觸發台語 ê 選字佮聲調功能。
- Tâi-lô: 「ing-bûn」 bôo-sik sī it-puann ê ing-bûn khí-puânn， kan-na ki-pún ê phing-im kiám-tsa nī， phah ing-bûn ê sî-sūn tshiat kuè--lâi iōng khah hong-piān， buē tshiok-huat tâi-gí ê suán jī kah siann-tiāu kong-lîng。
- POJ: 「eng-bûn」 bô͘-sek sī it-poaⁿ ê eng-bûn khí-pôaⁿ， kan-na ki-pún ê pheng-im kiám-cha nī， phah eng-bûn ê sî-sūn chhiat kòe--lâi iōng khah hong-piān， bōe chhiok-hoat tâi-gí ê soán jī kah siaⁿ-tiāu kong-lêng。
- English: English mode is an ordinary English keyboard with only basic spell-checking. Switch to it when typing English for convenience; it will not trigger Taigi candidate selection or tone features.
- 日本語: 「英語」モードは一般的な英語キーボードで、基本的なスペルチェックのみを行います。英語を打つときに切り替えると便利で、台湾語の候補選択や声調機能は作動しません。
- flags: 是=>sī:1char; 干焦=>kan-na:AMB; 爾=>nī:1char; 拍=>phah:1char; 時陣=>sî-sūn:AMB; 切=>tshiat:1char; 用=>iōng:1char; 較=>khah:1char; 袂=>buē:1char; 台語=>tâi-gí:AMB; 選=>suán:1char; 字=>jī:1char; 佮=>kah:1char


## `romanInput`

### `features[romanInput].title` — Tâi-lô conf: MED

- 漢字: 羅馬字輸入
- Tâi-lô: lô-má-jī su-ji̍p
- POJ: lô-má-jī su-ji̍p
- English: Romanization input
- 日本語: ローマ字入力
- flags: 羅馬字=>lô-má-jī:AMB; 輸入=>su-ji̍p:AMB

### `features[romanInput].paragraphs[0].text` — Tâi-lô conf: LOW

- 漢字: 拍羅馬字佮拍英文仝款，先拍字母，紲落來拍聲調數字（1-8），齒盤會自動加聲調符號。譬論用 'tong' 做例：拍 '2' 變做 'tóng'（黨），拍 '5' 變做 'tông'（同），拍 '7' 變做 'tōng'（洞）。
- Tâi-lô: phah lô-má-jī kah phah ing-bûn kāng-khuán， sian phah-jī bú， suà--lo̍h-lâi phah siann-tiāu sòo-jī（1-8）， khí-puânn huē tsū-tōng ka siann-tiāu hû-hō。 phì-lūn iōng 'tong' tsuè lē： phah '2' piàn-tsò 'tóng'（tóng）， phah '5' piàn-tsò 'tông'（tông）， phah '7' piàn-tsò 'tōng'（tōng）。
- POJ: phah lô-má-jī kah phah eng-bûn kāng-khoán， sian phah-jī bú， sòa--lo̍h-lâi phah siaⁿ-tiāu sò͘-jī（1-8）， khí-pôaⁿ hōe chū-tōng ka siaⁿ-tiāu hû-hō。 phì-lūn iōng 'tong' chòe lē： phah '2' piàn-chò 'tóng'（tóng）， phah '5' piàn-chò 'tông'（tông）， phah '7' piàn-chò 'tōng'（tōng）。
- English: Typing romanization is like typing English: type the letters first, then the tone number (1–8), and the keyboard adds the tone mark automatically. Take 'tong' for example: typing '2' gives 'tóng' (黨), '5' gives 'tông' (同), and '7' gives 'tōng' (洞).
- 日本語: ローマ字の入力は英語と同じで、まず字母を打ち、続けて声調数字（1〜8）を打つと、キーボードが自動的に声調記号を付けます。例えば 'tong' の場合、'2' を打つと 'tóng'（黨）、'5' で 'tông'（同）、'7' で 'tōng'（洞）になります。
- flags: 拍=>phah:1char; 羅馬字=>lô-má-jī:AMB; 佮=>kah:1char; 拍=>phah:1char; 先=>sian:1char; 拍字=>phah-jī:AMB; 母=>bú:1char; 拍=>phah:1char; 數字=>sòo-jī:AMB; 會=>huē:1char; 加=>ka:1char; 用=>iōng:1char; 做=>tsuè:1char; 例=>lē:1char; 拍=>phah:1char; 變做=>piàn-tsò:AMB; 黨=>tóng:1char; 拍=>phah:1char; 變做=>piàn-tsò:AMB; 同=>tông:1char; 拍=>phah:1char; 變做=>piàn-tsò:AMB; 洞=>tōng:1char

### `features[romanInput].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 台語有 8 个基本聲調。第 1 聲（陰平）佮第 4 聲（陰入）無聲調符號，拍數字齒盤會直接顯示數字，袂加符號。第 4 聲佮第 8 聲是入聲，韻尾一定是 -p、-t、-k 抑是 -h，譬論 'tok'（督）佮 'to̍k'（毒）。
- Tâi-lô: tâi-gí ū 8 ê ki-pún siann-tiāu。 tē 1 siann（im pîng） kah tē 4 siann（im ji̍p） bô-siann tiāu-hō hō， phah sòo-jī khí-puânn huē ti̍t-tsiap hián-sī sòo-jī， buē ka hû-hō。 tē 4 siann kah tē 8 siann sī ji̍p-siann， ūn-bué it-tīng sī -p、-t、-k ah-sī -h， phì-lūn 'tok'（tok） kah 'to̍k'（to̍k）。
- POJ: tâi-gí ū 8 ê ki-pún siaⁿ-tiāu。 tē 1 siaⁿ（im pêng） kah tē 4 siaⁿ（im ji̍p） bô-siaⁿ tiāu-hō hō， phah sò͘-jī khí-pôaⁿ hōe ti̍t-chiap hián-sī sò͘-jī， bōe ka hû-hō。 tē 4 siaⁿ kah tē 8 siaⁿ sī ji̍p-siaⁿ， ūn-bóe it-tēng sī -p、-t、-k ah-sī -h， phì-lūn 'tok'（tok） kah 'to̍k'（to̍k）。
- English: Taiwanese has 8 basic tones. The 1st tone (yin-level) and 4th tone (yin-entering) have no tone mark, so typing their numbers shows the digit directly with no mark added. The 4th and 8th tones are entering tones whose coda must be -p, -t, -k, or -h, e.g. 'tok' (督) and 'to̍k' (毒).
- 日本語: 台湾語には8つの基本声調があります。第1声（陰平）と第4声（陰入）は声調記号がなく、数字を打つとそのまま数字が表示され、記号は付きません。第4声と第8声は入声で、末子音は必ず -p、-t、-k または -h です。例えば 'tok'（督）と 'to̍k'（毒）です。
- flags: 台語=>tâi-gí:AMB; 有=>ū:1char; 个=>ê:1char; 第=>tē:1char; 聲=>siann:1char; 陰=>im:1char; 平=>pîng:1char; 佮=>kah:1char; 第=>tē:1char; 聲=>siann:1char; 陰=>im:1char; 入=>ji̍p:1char; 號=>hō:1char; 拍=>phah:1char; 數字=>sòo-jī:AMB; 會=>huē:1char; 數字=>sòo-jī:AMB; 袂=>buē:1char; 加=>ka:1char; 第=>tē:1char; 聲=>siann:1char; 佮=>kah:1char; 第=>tē:1char; 聲=>siann:1char; 是=>sī:1char; 入聲=>ji̍p-siann:AMB; 韻尾=>ūn-bué:AMB; 是=>sī:1char; 抑是=>ah-sī:AMB; 督=>tok:1char; 佮=>kah:1char; 毒=>to̍k:1char

### `features[romanInput].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 若聲調拍毋著，揤刪除揤仔會使修正：揤 1 改會先刪掉聲調符號，閣揤 1 改才會刪掉字母，所以拍毋著 ê 聲調免規个字重拍，刪除修正著好。
- Tâi-lô: nā siann-tiāu phah m̄-tio̍h， tshi̍h san-tî tshi̍h á ē-sái siu-tsìng： tshi̍h 1 kái huē sian san tiāu siann-tiāu hû-hō， koh tshi̍h 1 kái tsiah ē san tiāu jī-bú， sóo-í phah m̄-tio̍h ê siann-tiāu bián kui-ê jī tāng phah， san-tî siu-tsìng tio̍h hó。
- POJ: nā siaⁿ-tiāu phah m̄-tio̍h， chhi̍h san-tî chhi̍h á ē-sái siu-chèng： chhi̍h 1 kái hōe sian san tiāu siaⁿ-tiāu hû-hō， koh chhi̍h 1 kái chiah ē san tiāu jī-bú， só͘-í phah m̄-tio̍h ê siaⁿ-tiāu bián kui-ê jī tāng phah， san-tî siu-chèng tio̍h hó。
- English: If you mistype a tone, you can fix it with the delete key: pressing it once removes the tone mark first, and pressing it again removes the letter. So a wrong tone does not require retyping the whole syllable—just delete to correct it.
- 日本語: 声調を打ち間違えたら、削除キーで直せます。1回押すとまず声調記号が消え、もう1回押すと字母が消えます。声調を間違えても字全体を打ち直す必要はなく、削除で修正できます。
- flags: 若=>nā:1char; 拍=>phah:1char; 揤=>tshi̍h:1char; 刪除=>san-tî:AMB; 揤=>tshi̍h:1char; 仔=>á:1char; 會使=>ē-sái:AMB; 揤=>tshi̍h:1char; 改=>kái:1char; 會=>huē:1char; 先=>sian:1char; 刪=>san:1char; 掉=>tiāu:1char; 閣=>koh:1char; 揤=>tshi̍h:1char; 改=>kái:1char; 刪=>san:1char; 掉=>tiāu:1char; 字母=>jī-bú:AMB; 拍=>phah:1char; 免=>bián:1char; 字=>jī:1char; 重=>tāng:1char; 拍=>phah:1char; 刪除=>san-tî:AMB; 著=>tio̍h:1char; 好=>hó:1char

### `features[romanInput].paragraphs[3].text` — Tâi-lô conf: MED

- 漢字: 白話字有特殊組合：連紲拍 2 个 'o' 會變做 'o͘'，連紲拍 2 个 'n' 會變做 'ⁿ'，佇設定有開關通切。台羅免特殊處理，'oo' 佮 'nn' 直接顯示。
- Tâi-lô: pe̍h-uē-jī ū ti̍k-sû tsoo-ha̍p： liân-suà phah 2 ê 'o' huē piàn-tsò 'o͘'， liân-suà phah 2 ê 'n' huē piàn-tsò 'ⁿ'， tī siat-tīng ū khai-kuan thang tshiat。 tâi-lô bián ti̍k-sû tshú-lí，'oo' kah 'nn' ti̍t-tsiap hián-sī。
- POJ: pe̍h-ōe-jī ū te̍k-sû cho͘-ha̍p： liân-sòa phah 2 ê 'o' hōe piàn-chò 'o͘'， liân-sòa phah 2 ê 'n' hōe piàn-chò 'ⁿ'， tī siat-tēng ū khai-koan thang chhiat。 tâi-lô bián te̍k-sû chhú-lí，'oo' kah 'nn' ti̍t-chiap hián-sī。
- English: Pe̍h-ōe-jī has special combinations: typing two 'o' in a row becomes 'o͘', and typing two 'n' in a row becomes 'ⁿ'; there is a toggle for this in settings. Tâi-lô needs no special handling—'oo' and 'nn' display directly.
- 日本語: 白話字には特殊な組み合わせがあります。'o' を続けて2回打つと 'o͘' に、'n' を続けて2回打つと 'ⁿ' になります。設定に切り替えスイッチがあります。台羅では特殊な処理は不要で、'oo' と 'nn' はそのまま表示されます。
- flags: 白話字=>pe̍h-uē-jī:AMB; 有=>ū:1char; 拍=>phah:1char; 个=>ê:1char; 會=>huē:1char; 變做=>piàn-tsò:AMB; 拍=>phah:1char; 个=>ê:1char; 會=>huē:1char; 變做=>piàn-tsò:AMB; 佇=>tī:1char; 有=>ū:1char; 通=>thang:1char; 切=>tshiat:1char; 免=>bián:1char; 佮=>kah:1char

### `features[romanInput].paragraphs[4].text` — Tâi-lô conf: LOW

- 漢字: 拍好羅馬字了後，齒盤會自動揣辭典，候選欄第 1 个位是你拍 ê 字，後壁 ê 是候選詞。拍字無一定愛加聲調，但是有加結果較精準。
- Tâi-lô: phah hó lô-má-jī liáu-āu， khí-puânn huē tsū-tōng tshuē sû-tián， hāu-suán nuâ tē 1 ê uī sī lí phah ê jī， āu-piah ê sī hāu-suán sû。 phah-jī bô-it-tīng ài ka siann-tiāu， tān-sī ū ka kiat-kó khah tsing-tsún。
- POJ: phah hó lô-má-jī liáu-āu， khí-pôaⁿ hōe chū-tōng chhōe sû-tián， hāu-soán nôa tē 1 ê ūi sī lí phah ê jī， āu-piah ê sī hāu-soán sû。 phah-jī bô-it-tēng ài ka siaⁿ-tiāu， tān-sī ū ka kiat-kó khah cheng-chún。
- English: After you type the romanization, the keyboard looks up the dictionary automatically. The first slot in the candidate bar is what you typed; the rest are candidate words. You do not have to add tones, but adding them makes the results more accurate.
- 日本語: ローマ字を入力すると、キーボードが自動的に辞書を検索します。候補欄の最初の位置は入力した文字そのもので、その後ろが候補語です。声調は必須ではありませんが、付けると結果がより正確になります。
- flags: 拍=>phah:1char; 好=>hó:1char; 羅馬字=>lô-má-jī:AMB; 會=>huē:1char; 揣=>tshuē:1char; 欄=>nuâ:1char; 第=>tē:1char; 个=>ê:1char; 位=>uī:1char; 是=>sī:1char; 你=>lí:1char; 拍=>phah:1char; 字=>jī:1char; 是=>sī:1char; 詞=>sû:1char; 拍字=>phah-jī:AMB; 愛=>ài:1char; 加=>ka:1char; 有=>ū:1char; 加=>ka:1char; 較=>khah:1char

### `features[romanInput].paragraphs[5].text` — Tâi-lô conf: LOW

- 漢字: 用「gâu-tsá」（𠢕早）做例，以下拍法攏揣會著：'gau5-tsa2'（加連劃）、'gau5tsa2'（無連劃）、'gautsa'（免拍聲調）、'gt'（頭字母縮寫）。拍法愈完整，結果愈精準。
- Tâi-lô: iōng「gâu-tsá」（gâu-tsá） tsuè lē， í-hā phah huat láng tshuē huē tio̍h：'gau5-tsa2'（ka liân-ue̍h）、'gau5tsa2'（bô liân-ue̍h）、'gautsa'（bián phah siann-tiāu）、'gt'（thâu-jī bú sok-siá）。 phah huat jú uân-tsíng， kiat-kó jú tsing-tsún。
- POJ: iōng「gâu-chá」（gâu-chá） chòe lē， í-hā phah hoat láng chhōe hōe tio̍h：'gau5-tsa2'（ka liân-oe̍h）、'gau5tsa2'（bô liân-oe̍h）、'gautsa'（bián phah siaⁿ-tiāu）、'gt'（thâu-jī bú sok-siá）。 phah hoat jú oân-chéng， kiat-kó jú cheng-chún。
- English: Take “gâu-tsá” (𠢕早) as an example; all of these spellings will find it: 'gau5-tsa2' (with hyphen), 'gau5tsa2' (without hyphen), 'gautsa' (no tones), 'gt' (initial-letter abbreviation). The more complete the spelling, the more accurate the result.
- 日本語: 「gâu-tsá」（𠢕早）を例にすると、次のどの打ち方でも見つかります。'gau5-tsa2'（ハイフンあり）、'gau5tsa2'（ハイフンなし）、'gautsa'（声調なし）、'gt'（頭文字の略字）。打ち方が完全なほど結果は正確になります。
- flags: 用=>iōng:1char; 做=>tsuè:1char; 例=>lē:1char; 拍=>phah:1char; 法=>huat:1char; 攏=>láng:1char; 揣=>tshuē:1char; 會=>huē:1char; 著=>tio̍h:1char; 加=>ka:1char; 連劃=>liân-ue̍h:AMB; 無=>bô:1char; 連劃=>liân-ue̍h:AMB; 免=>bián:1char; 拍=>phah:1char; 母=>bú:1char; 拍=>phah:1char; 法=>huat:1char; 愈=>jú:1char; 愈=>jú:1char


## `accuracyTips`

### `features[accuracyTips].title` — Tâi-lô conf: MED

- 漢字: 按怎拍字拍較準
- Tâi-lô: án-nuá phah-jī phah khah tsún
- POJ: án-nóa phah-jī phah khah chún
- English: How to type more accurately
- 日本語: より正確に入力するコツ
- flags: 按怎=>án-nuá:AMB; 拍字=>phah-jī:AMB; 拍=>phah:1char; 較=>khah:1char; 準=>tsún:1char

### `features[accuracyTips].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 拍字若愈完整，結果就愈準。譬論 gâu-tsá，若是拍 gautsa 抑是 gau5tsa2，候選詞會比干焦拍 gt 閣較準。
- Tâi-lô: phah-jī nā jú uân-tsíng， kiat-kó tsiū jú tsún。 phì-lūn gâu-tsá， nā-sī phah gautsa ah-sī gau5tsa2， hāu-suán sû huē pí kan-na phah gt koh-khah tsún。
- POJ: phah-jī nā jú oân-chéng， kiat-kó chiū jú chún。 phì-lūn gâu-chá， nā-sī phah gautsa ah-sī gau5tsa2， hāu-soán sû hōe pí kan-na phah gt koh-khah chún。
- English: The more complete your input, the more accurate the results. For gâu-tsá, for example, typing gautsa or gau5tsa2 gives more accurate candidates than just typing gt.
- 日本語: 入力が完全なほど結果は正確になります。例えば gâu-tsá なら、gautsa や gau5tsa2 と打つほうが、gt だけより候補が正確になります。
- flags: 拍字=>phah-jī:AMB; 若=>nā:1char; 愈=>jú:1char; 就=>tsiū:1char; 愈=>jú:1char; 準=>tsún:1char; 拍=>phah:1char; 抑是=>ah-sī:AMB; 詞=>sû:1char; 會=>huē:1char; 比=>pí:1char; 干焦=>kan-na:AMB; 拍=>phah:1char; 準=>tsún:1char

### `features[accuracyTips].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 【聲調 1、4 愛拍出來】教育部輸入法免拍第 1 聲佮第 4 聲，台語齒盤雖然嘛會使免拍，但是若有拍聲調，結果會較準。以「phang 芳」這个字做例，拍 phang1 會正確揣第一聲 ê 字，若干焦拍 phang，會出現誠濟結果。
- Tâi-lô: 【 siann-tiāu 1、4 ài phah-tshut lâi】 kàu-io̍k-pōo su-ji̍p-hoat bián phah tē 1 siann kah tē 4 siann， tâi-gí khí-puânn sui-jiân mā ē-sái bián phah， tān-sī nā-ū phah siann-tiāu， kiat-kó huē khah tsún。 í「phang phang」 tsit ê jī tsuè lē， phah phang1 huē tsìng-khak tshuē tē-it siann ê jī， jio̍k-kan ta phah phang， huē tshut-hiān tsiânn-tsē kiat-kó。
- POJ: 【 siaⁿ-tiāu 1、4 ài phah-chhut lâi】 kàu-io̍k-pō͘ su-ji̍p-hoat bián phah tē 1 siaⁿ kah tē 4 siaⁿ， tâi-gí khí-pôaⁿ sui-jiân mā ē-sái bián phah， tān-sī nā-ū phah siaⁿ-tiāu， kiat-kó hōe khah chún。 í「phang phang」 chit ê jī chòe lē， phah phang1 hōe chèng-khak chhōe tē-it siaⁿ ê jī， jio̍k-kan ta phah phang， hōe chhut-hiān chiâⁿ-chē kiat-kó。
- English: [Type out tones 1 and 4] The Ministry of Education input method lets you skip the 1st and 4th tones. Taigi Keyboard also lets you skip them, but typing the tone gives more accurate results. Take “phang 芳” for example: typing phang1 correctly finds the 1st-tone character, whereas typing just phang returns many results.
- 日本語: 【第1声・第4声も入力する】教育部の入力法では第1声と第4声を省略できます。台語キーボードでも省略できますが、声調を入力すると結果がより正確になります。「phang 芳」を例にすると、phang1 と打てば第1声の字が正しく見つかりますが、phang だけだと多くの結果が出ます。
- flags: 愛=>ài:1char; 來=>lâi:1char; 免=>bián:1char; 拍=>phah:1char; 第=>tē:1char; 聲=>siann:1char; 佮=>kah:1char; 第=>tē:1char; 聲=>siann:1char; 台語=>tâi-gí:AMB; 雖然=>sui-jiân:AMB; 嘛=>mā:1char; 會使=>ē-sái:AMB; 免=>bián:1char; 拍=>phah:1char; 拍=>phah:1char; 會=>huē:1char; 較=>khah:1char; 準=>tsún:1char; 以=>í:1char; 芳=>phang:1char; 字=>jī:1char; 做=>tsuè:1char; 例=>lē:1char; 拍=>phah:1char; 會=>huē:1char; 揣=>tshuē:1char; 第一=>tē-it:AMB; 聲=>siann:1char; 字=>jī:1char; 焦=>ta:1char; 拍=>phah:1char; 會=>huē:1char

### `features[accuracyTips].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 第 4 聲嘛相仝，第 4 聲韻尾一定是 -p、-t、-k、-h，若聲調數字拍出來會比無拍較準。
- Tâi-lô: tē 4 siann mā sio-kâng， tē 4 siann-ūn bué it-tīng sī -p、-t、-k、-h， nā siann-tiāu sòo-jī phah-tshut lâi huē pí bô phah khah tsún。
- POJ: tē 4 siaⁿ mā sio-kâng， tē 4 siaⁿ-ūn bóe it-tēng sī -p、-t、-k、-h， nā siaⁿ-tiāu sò͘-jī phah-chhut lâi hōe pí bô phah khah chún。
- English: The same goes for the 4th tone: its coda is always -p, -t, -k, or -h, and typing the tone number is more accurate than not.
- 日本語: 第4声も同様で、末子音は必ず -p、-t、-k、-h です。声調数字を入力するほうが、入力しないより正確になります。
- flags: 第=>tē:1char; 聲=>siann:1char; 嘛=>mā:1char; 相仝=>sio-kâng:AMB; 第=>tē:1char; 尾=>bué:1char; 是=>sī:1char; 若=>nā:1char; 數字=>sòo-jī:AMB; 來=>lâi:1char; 會=>huē:1char; 比=>pí:1char; 無=>bô:1char; 拍=>phah:1char; 較=>khah:1char; 準=>tsún:1char

### `features[accuracyTips].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 【關掉無需要 ê 辭典】若感覺候選詞傷濟、想欲揣 ê 字攏佇後壁，會當去「詞庫管理」頁面共無需要 ê 辭典關起來。
- Tâi-lô: 【 kuainn--tiāu bô-su-iàu ê sû-tián】 nā kám-kak hāu-suán sû siunn-tse、 siūnn-beh tshuē ê jī láng tī āu-piah， ē-tàng khì「sû khòo kuán-lí」 ia̍h-bīn kā bô-su-iàu ê sû-tián kuainn--khí-lâi。
- POJ: 【 koaiⁿ--tiāu bô-su-iàu ê sû-tián】 nā kám-kak hāu-soán sû siuⁿ-che、 siūⁿ-beh chhōe ê jī láng tī āu-piah， ē-tàng khì「sû khò͘ koán-lí」 ia̍h-bīn kā bô-su-iàu ê sû-tián koaiⁿ--khí-lâi。
- English: [Turn off dictionaries you don't need] If there are too many candidates and the character you want is always at the back, you can turn off unneeded dictionaries on the Dictionary Management page.
- 日本語: 【不要な辞書をオフにする】候補が多すぎて、探している字がいつも後ろにある場合は、「辞書管理」ページで不要な辞書をオフにできます。
- flags: 若=>nā:1char; 詞=>sû:1char; 傷濟=>siunn-tse:AMB; 想欲=>siūnn-beh:AMB; 揣=>tshuē:1char; 字=>jī:1char; 攏=>láng:1char; 佇=>tī:1char; 會當=>ē-tàng:AMB; 去=>khì:1char; 詞=>sû:1char; 庫=>khòo:1char; 共=>kā:1char

### `features[accuracyTips].paragraphs[4].text` — Tâi-lô conf: LOW

- 漢字: 【「腔口差」開關關起來】「腔口差」收錄誠濟無仝腔口，會增加候選詞 ê 數量。若你平常時拍字干焦用通用腔，會使考慮佇「詞庫管理」頁面共「腔口差」關起來。
- Tâi-lô: 【「khiunn-kháu tsha」 khai-kuan kuainn--khí-lâi】「khiunn-kháu tsha」 siu-lio̍k tsiânn-tsē bô kâng khiunn-kháu， huē tsing-ka hāu-suán sû ê sòo-liōng。 nā lí pîng-siông-sî phah-jī kan-na iōng thong-iōng khiunn， ē-sái khó-lū tī「sû khòo kuán-lí」 ia̍h-bīn kā「khiunn-kháu tsha」 kuainn--khí-lâi。
- POJ: 【「khiuⁿ-kháu chha」 khai-koan koaiⁿ--khí-lâi】「khiuⁿ-kháu chha」 siu-lio̍k chiâⁿ-chē bô kâng khiuⁿ-kháu， hōe cheng-ka hāu-soán sû ê sò͘-liōng。 nā lí pêng-siông-sî phah-jī kan-na iōng thong-iōng khiuⁿ， ē-sái khó-lū tī「sû khò͘ koán-lí」 ia̍h-bīn kā「khiuⁿ-kháu chha」 koaiⁿ--khí-lâi。
- English: [Turn off Accent Variations] Accent Variations includes many different accents, which increases the number of candidates. If you usually type only in the common accent, consider turning off Accent Variations on the Dictionary Management page.
- 日本語: 【「方言差」をオフにする】「方言差」は多くの異なる訛りを収録しており、候補語の数が増えます。普段は共通の発音だけで入力するなら、「辞書管理」ページで「方言差」をオフにすることを検討してください。
- flags: 差=>tsha:1char; 差=>tsha:1char; 收錄=>siu-lio̍k:AMB; 無仝=>bô kâng:AMB; 會=>huē:1char; 詞=>sû:1char; 若=>nā:1char; 你=>lí:1char; 拍字=>phah-jī:AMB; 干焦=>kan-na:AMB; 用=>iōng:1char; 腔=>khiunn:1char; 會使=>ē-sái:AMB; 考慮=>khó-lū:AMB; 佇=>tī:1char; 詞=>sû:1char; 庫=>khòo:1char; 共=>kā:1char; 差=>tsha:1char


## `hanloDesign`

### `features[hanloDesign].title` — Tâi-lô conf: HIGH

- 漢字: 針對漢羅合用設計
- Tâi-lô: tsiam-tuì hàn-lô ha̍h-īng siat-kè
- POJ: chiam-tùi hàn-lô ha̍h-ēng siat-kè
- English: Designed for mixed Hanji-romanization
- 日本語: 漢羅混用のための設計

### `features[hanloDesign].summary` — Tâi-lô conf: LOW

- 漢字: 「括號標註」予漢字佮羅馬字做伙輸出。「自動空白」佇漢羅合用 ê 時陣自動處理空格。
- Tâi-lô: 「kuat-hō phiau-tsù」 hōo hàn-jī kah lô-má-jī tsò-hué su-tshut。「tsū-tōng khàng-pe̍h」 tī hàn-lô ha̍h-īng ê sî-sūn tsū-tōng tshú-lí khang-keh。
- POJ: 「koat-hō phiau-chù」 hō͘ hàn-jī kah lô-má-jī chò-hóe su-chhut。「chū-tōng khàng-pe̍h」 tī hàn-lô ha̍h-ēng ê sî-sūn chū-tōng chhú-lí khang-keh。
- English: “Bracket annotation” outputs the Hanji and romanization together. “Auto-space” handles spacing automatically when mixing Hanji and romanization.
- 日本語: 「括弧注記」は漢字とローマ字を一緒に出力します。「自動スペース」は漢羅混用のときに空白を自動で処理します。
- flags: 予=>hōo:1char; 漢字=>hàn-jī:AMB; 佮=>kah:1char; 羅馬字=>lô-má-jī:AMB; 做伙=>tsò-hué:AMB; 佇=>tī:1char; 時陣=>sî-sūn:AMB

### `features[hanloDesign].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 台語書寫有人慣勢用全漢字，有人慣勢用全羅馬字，嘛有人漢字佮羅馬字攙做伙寫，這个齒盤有特別為著無仝拍字習慣設計。
- Tâi-lô: tâi-gí su-siá ū-lâng kuàn-sì iōng tsuân-hàn jī， ū-lâng kuàn-sì iōng tsuân-lô bé jī， mā-ū jîn hàn-jī kah lô-má-jī tsham tsò-hué siá， tsit ê khí-puânn ū ti̍k-pia̍t uī-tio̍h bô kâng phah-jī si̍p-kuàn siat-kè。
- POJ: tâi-gí su-siá ū-lâng koàn-sì iōng choân-hàn jī， ū-lâng koàn-sì iōng choân-lô bé jī， mā-ū jîn hàn-jī kah lô-má-jī chham chò-hóe siá， chit ê khí-pôaⁿ ū te̍k-pia̍t ūi-tio̍h bô kâng phah-jī si̍p-koàn siat-kè。
- English: Some people write Taiwanese in all Hanji, some in all romanization, and some mix Hanji and romanization. This keyboard is specially designed for these different writing habits.
- 日本語: 台湾語の表記には、すべて漢字で書く人、すべてローマ字で書く人、漢字とローマ字を混ぜて書く人がいます。このキーボードはそうした異なる表記習慣のために特別に設計されています。
- flags: 台語=>tâi-gí:AMB; 用=>iōng:1char; 字=>jī:1char; 用=>iōng:1char; 馬=>bé:1char; 字=>jī:1char; 人=>jîn:1char; 漢字=>hàn-jī:AMB; 佮=>kah:1char; 羅馬字=>lô-má-jī:AMB; 攙=>tsham:1char; 做伙=>tsò-hué:AMB; 寫=>siá:1char; 有=>ū:1char; 無仝=>bô kâng:AMB; 拍字=>phah-jī:AMB

### `features[hanloDesign].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 齒盤上尾排有 1 个「漢 / 羅」轉換揤鈕，揤著會切換漢字佮羅馬字模式。預設是羅馬字模式，揤候選詞會輸出羅馬字，漢字顯示佇下底做參考；切做「漢字優先」模式了後，揤候選詞會直接輸出漢字，羅馬字改做參考，標點嘛會自動換做全形。
- Tâi-lô: khí-puânn siōng-bué pâi ū 1 ê「hàn / lô」 tsuán-uānn tshi̍h-liú， tshi̍h tio̍h huē tshiat-uānn hàn-jī kah lô-má-jī bôo-sik。 ī-siat sī lô-má-jī bôo-sik， tshi̍h hāu-suán sû huē su-tshut lô-má-jī， hàn-jī hián-sī tī ē-tué tsuè tsham-khó； tshiat tsuè「hàn-jī iu-sian」 bôo-sik liáu-āu， tshi̍h hāu-suán sû huē ti̍t-tsiap su-tshut hàn-jī， lô-má-jī kué-tsuè tsham-khó， phiau-tiám mā huē tsū-tōng uānn-tsò tsuân hîng。
- POJ: khí-pôaⁿ siōng-bóe pâi ū 1 ê「hàn / lô」 choán-ōaⁿ chhi̍h-liú， chhi̍h tio̍h hōe chhiat-ōaⁿ hàn-jī kah lô-má-jī bô͘-sek。 ī-siat sī lô-má-jī bô͘-sek， chhi̍h hāu-soán sû hōe su-chhut lô-má-jī， hàn-jī hián-sī tī ē-tóe chòe chham-khó； chhiat chòe「hàn-jī iu-sian」 bô͘-sek liáu-āu， chhi̍h hāu-soán sû hōe ti̍t-chiap su-chhut hàn-jī， lô-má-jī kóe-chòe chham-khó， phiau-tiám mā hōe chū-tōng ōaⁿ-chò choân hêng。
- English: The bottom row of the keyboard has a “漢 / 羅” toggle button that switches between Hanji and romanization modes. The default is romanization mode: tapping a candidate outputs the romanization, with the Hanji shown below for reference. After switching to “Hanji-first” mode, tapping a candidate outputs the Hanji directly, the romanization becomes the reference, and punctuation is also switched to full-width automatically.
- 日本語: キーボードの最下段に「漢 / 羅」切り替えボタンがあり、漢字モードとローマ字モードを切り替えます。既定はローマ字モードで、候補をタップするとローマ字が出力され、漢字は下に参考として表示されます。「漢字優先」モードに切り替えると、候補をタップすると漢字が直接出力され、ローマ字が参考に変わり、句読点も自動的に全角になります。
- flags: 上尾=>siōng-bué:AMB; 排=>pâi:1char; 有=>ū:1char; 个=>ê:1char; 漢=>hàn:1char; 羅=>lô:1char; 揤=>tshi̍h:1char; 著=>tio̍h:1char; 會=>huē:1char; 漢字=>hàn-jī:AMB; 佮=>kah:1char; 羅馬字=>lô-má-jī:AMB; 預設=>ī-siat:AMB; 是=>sī:1char; 羅馬字=>lô-má-jī:AMB; 揤=>tshi̍h:1char; 詞=>sû:1char; 會=>huē:1char; 羅馬字=>lô-má-jī:AMB; 漢字=>hàn-jī:AMB; 佇=>tī:1char; 下底=>ē-tué:AMB; 做=>tsuè:1char; 切=>tshiat:1char; 做=>tsuè:1char; 漢字=>hàn-jī:AMB; 揤=>tshi̍h:1char; 詞=>sû:1char; 會=>huē:1char; 漢字=>hàn-jī:AMB; 羅馬字=>lô-má-jī:AMB; 標點=>phiau-tiám:AMB; 嘛=>mā:1char; 會=>huē:1char; 換做=>uānn-tsò:AMB; 全=>tsuân:1char; 形=>hîng:1char

### `features[hanloDesign].paragraphs[2].text` — Tâi-lô conf: MED

- 漢字: 「括號標註」開關切開了後，輸出 ê 時陣漢字佮羅馬字會做伙出來，免家己加註，適合予當咧學台語 ê 朋友看。
- Tâi-lô: 「kuat-hō phiau-tsù」 khai-kuan tshiat-khui liáu-āu， su-tshut ê sî-sūn hàn-jī kah lô-má-jī huē tsò-hué tshut-lâi， bián ka-kī ke-tsù， sik-ha̍p hōo tng-teh ha̍k tâi-gí ê pîng-iú khuànn。
- POJ: 「koat-hō phiau-chù」 khai-koan chhiat-khui liáu-āu， su-chhut ê sî-sūn hàn-jī kah lô-má-jī hōe chò-hóe chhut-lâi， bián ka-kī ke-chù， sek-ha̍p hō͘ tng-teh ha̍k tâi-gí ê pêng-iú khòaⁿ。
- English: After the “Bracket annotation” toggle is on, the Hanji and romanization come out together when you type, with no need to annotate them yourself—handy for friends who are learning Taiwanese.
- 日本語: 「括弧注記」スイッチをオンにすると、出力時に漢字とローマ字が一緒に出てくるので、自分で注記する必要がありません。台湾語を学んでいる人に見せるのに便利です。
- flags: 時陣=>sî-sūn:AMB; 漢字=>hàn-jī:AMB; 佮=>kah:1char; 羅馬字=>lô-má-jī:AMB; 會=>huē:1char; 做伙=>tsò-hué:AMB; 免=>bián:1char; 家己=>ka-kī:AMB; 予=>hōo:1char; 當咧=>tng-teh:AMB; 學=>ha̍k:1char; 台語=>tâi-gí:AMB; 看=>khuànn:1char

### `features[hanloDesign].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 寫漢羅合用文 ê 時陣，使用「漢 / 羅」揤鈕隨時切換，配合「自動空白」開關，羅馬字 ê 空格縫嘛會自動處理，免家己加空格。
- Tâi-lô: siá hàn-lô ha̍h-īng bûn ê sî-sūn， sú-iōng「hàn / lô」 tshi̍h-liú suî-sî tshiat-uānn， phuè-ha̍p「tsū-tōng khàng-pe̍h」 khai-kuan， lô-má-jī ê khang-keh phāng mā huē tsū-tōng tshú-lí， bián ka-kī ka khang-keh。
- POJ: siá hàn-lô ha̍h-ēng bûn ê sî-sūn， sú-iōng「hàn / lô」 chhi̍h-liú sûi-sî chhiat-ōaⁿ， phòe-ha̍p「chū-tōng khàng-pe̍h」 khai-koan， lô-má-jī ê khang-keh phāng mā hōe chū-tōng chhú-lí， bián ka-kī ka khang-keh。
- English: When writing mixed Hanji-romanization text, use the “漢 / 羅” button to switch anytime, and with the “Auto-space” toggle the spacing around romanization is handled automatically, so you don't have to add spaces yourself.
- 日本語: 漢羅混用の文章を書くときは、「漢 / 羅」ボタンでいつでも切り替えられます。「自動スペース」スイッチと併せると、ローマ字の空白も自動で処理され、自分で空白を入れる必要がありません。
- flags: 寫=>siá:1char; 文=>bûn:1char; 時陣=>sî-sūn:AMB; 漢=>hàn:1char; 羅=>lô:1char; 配合=>phuè-ha̍p:AMB; 羅馬字=>lô-má-jī:AMB; 縫=>phāng:1char; 嘛=>mā:1char; 會=>huē:1char; 免=>bián:1char; 家己=>ka-kī:AMB; 加=>ka:1char


## `caseSwitch`

### `features[caseSwitch].title` — Tâi-lô conf: LOW

- 漢字: 3 段式大本字切換
- Tâi-lô: 3 tuānn sik tuā-pún-jī tshiat-uānn
- POJ: 3 tōaⁿ sek tōa-pún-jī chhiat-ōaⁿ
- English: Three-stage uppercase switching
- 日本語: 3段階の大文字切り替え
- flags: 段=>tuānn:1char; 式=>sik:1char; 大本字=>tuā-pún-jī:AMB

### `features[caseSwitch].summary` — Tâi-lô conf: MED

- 漢字: 「自動大本字」控制頭字母敢會自動變大本字。關起來著是細本字模式，連紲揤 Shift 2 改切做 Caps Lock。
- Tâi-lô: 「tsū-tōng tuā-pún-jī」 khòng-tsè thâu-jī bú kám-uē tsū-tōng piàn tuā-pún-jī。 kuainn--khí-lâi tio̍h-sī suè-pún-jī bôo-sik， liân-suà tshi̍h Shift 2 kái tshiat tsuè Caps Lock。
- POJ: 「chū-tōng tōa-pún-jī」 khòng-chè thâu-jī bú kám-ōe chū-tōng piàn tōa-pún-jī。 koaiⁿ--khí-lâi tio̍h-sī sòe-pún-jī bô͘-sek， liân-sòa chhi̍h Shift 2 kái chhiat chòe Caps Lock。
- English: “Auto-capitalize” controls whether the first letter is capitalized automatically. Turning it off gives lowercase mode; pressing Shift twice in a row switches to Caps Lock.
- 日本語: 「自動大文字」は先頭の字母を自動で大文字にするかどうかを制御します。オフにすると小文字モードになり、Shift を続けて2回押すと Caps Lock に切り替わります。
- flags: 大本字=>tuā-pún-jī:AMB; 母=>bú:1char; 敢會=>kám-uē:AMB; 變=>piàn:1char; 大本字=>tuā-pún-jī:AMB; 細本字=>suè-pún-jī:AMB; 揤=>tshi̍h:1char; 改=>kái:1char; 切=>tshiat:1char; 做=>tsuè:1char

### `features[caseSwitch].paragraphs[0].text` — Tâi-lô conf: LOW

- 漢字: 一般 ê 情形，頭 1 个字母會自動變大本字，Shift 揤鈕會反烏。若「自動大本字」有關起來，著愛家己揤 Shift，頭字才會變大本字。
- Tâi-lô: it-puann ê tsîng-hîng， thâu 1 ê jī-bú huē tsū-tōng piàn tuā-pún-jī，Shift tshi̍h-liú huē huán-oo。 nā「tsū-tōng tuā-pún-jī」 iú-kuan khí-lâi， tio̍h-ài ka-kī tshi̍h Shift， thâu-jī tsiah ē piàn tuā-pún-jī。
- POJ: it-poaⁿ ê chêng-hêng， thâu 1 ê jī-bú hōe chū-tōng piàn tōa-pún-jī，Shift chhi̍h-liú hōe hoán-o͘。 nā「chū-tōng tōa-pún-jī」 iú-koan khí-lâi， tio̍h-ài ka-kī chhi̍h Shift， thâu-jī chiah ē piàn tōa-pún-jī。
- English: Normally the first letter is capitalized automatically and the Shift key is highlighted. If “Auto-capitalize” is turned off, you have to press Shift yourself for the first letter to be capitalized.
- 日本語: 通常は先頭の字母が自動で大文字になり、Shift キーが反転します。「自動大文字」をオフにしている場合は、自分で Shift を押さないと先頭の字が大文字になりません。
- flags: 頭=>thâu:1char; 个=>ê:1char; 字母=>jī-bú:AMB; 會=>huē:1char; 變=>piàn:1char; 大本字=>tuā-pún-jī:AMB; 會=>huē:1char; 若=>nā:1char; 大本字=>tuā-pún-jī:AMB; 家己=>ka-kī:AMB; 揤=>tshi̍h:1char; 變=>piàn:1char; 大本字=>tuā-pún-jī:AMB

### `features[caseSwitch].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 「自動大本字」開關若關起來，就是細本字模式，拍出來 ê 字攏是細本字。
- Tâi-lô: 「tsū-tōng tuā-pún-jī」 khai-kuan nā kuainn--khí-lâi， tiō sī suè-pún-jī bôo-sik， phah-tshut lâi ê jī lóng sī suè-pún-jī。
- POJ: 「chū-tōng tōa-pún-jī」 khai-koan nā koaiⁿ--khí-lâi， tiō sī sòe-pún-jī bô͘-sek， phah-chhut lâi ê jī lóng sī sòe-pún-jī。
- English: If the “Auto-capitalize” toggle is off, you are in lowercase mode and everything you type comes out lowercase.
- 日本語: 「自動大文字」スイッチをオフにすると小文字モードになり、入力した字はすべて小文字になります。
- flags: 大本字=>tuā-pún-jī:AMB; 若=>nā:1char; 就是=>tiō sī:AMB; 細本字=>suè-pún-jī:AMB; 來=>lâi:1char; 字=>jī:1char; 細本字=>suè-pún-jī:AMB

### `features[caseSwitch].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 連紲揤 Shift 2 改會切做 Caps Lock 模式，揤鈕變烏色，圖示嘛無仝款，這時陣拍出來 ê 字攏是大本字，閣揤 1 改才會轉去細本字。
- Tâi-lô: liân-suà tshi̍h Shift 2 kái huē tshiat tsuè Caps Lock bôo-sik， tshi̍h-liú piàn-oo sik， tôo-sī mā bô-kāng-khuán， tsit-sî-tsūn phah-tshut lâi ê jī lóng sī tuā-pún-jī， koh tshi̍h 1 kái tsiah ē tńg--khì suè-pún-jī。
- POJ: liân-sòa chhi̍h Shift 2 kái hōe chhiat chòe Caps Lock bô͘-sek， chhi̍h-liú piàn-o͘ sek， tô͘-sī mā bô-kāng-khoán， chit-sî-chūn phah-chhut lâi ê jī lóng sī tōa-pún-jī， koh chhi̍h 1 kái chiah ē tńg--khì sòe-pún-jī。
- English: Pressing Shift twice in a row switches to Caps Lock mode; the key turns dark and its icon changes. In this state everything you type is uppercase, and pressing it once more returns to lowercase.
- 日本語: Shift を続けて2回押すと Caps Lock モードに切り替わり、キーが黒くなりアイコンも変わります。この状態では入力した字がすべて大文字になり、もう1回押すと小文字に戻ります。
- flags: 揤=>tshi̍h:1char; 改=>kái:1char; 會=>huē:1char; 切=>tshiat:1char; 做=>tsuè:1char; 色=>sik:1char; 嘛=>mā:1char; 來=>lâi:1char; 字=>jī:1char; 大本字=>tuā-pún-jī:AMB; 閣=>koh:1char; 揤=>tshi̍h:1char; 改=>kái:1char; 轉去=>tńg--khì:AMB; 細本字=>suè-pún-jī:AMB

### `features[caseSwitch].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 若連紲切換符號齒盤，大細本字有時陣會走精去，若拄著這个情形，切轉來羅馬字齒盤著會正常。
- Tâi-lô: nā liân-suà tshiat-uānn hû-hō khí-puânn， tuā-suè-pún-jī ū-sî-tsūn huē tsáu-tsing khì， nā tú-tio̍h tsit ê tsîng-hîng， tshiat tńg--lâi lô-má-jī khí-puânn tio̍h huē tsìng-siông。
- POJ: nā liân-sòa chhiat-ōaⁿ hû-hō khí-pôaⁿ， tōa-sòe-pún-jī ū-sî-chūn hōe cháu-cheng khì， nā tú-tio̍h chit ê chêng-hêng， chhiat tńg--lâi lô-má-jī khí-pôaⁿ tio̍h hōe chèng-siông。
- English: If you switch to the symbol keyboard repeatedly, the upper/lowercase state can sometimes drift. If this happens, switching back to the romanization keyboard restores it.
- 日本語: 記号キーボードへの切り替えを繰り返すと、大文字・小文字の状態がずれることがあります。その場合は、ローマ字キーボードに戻すと正常になります。
- flags: 若=>nā:1char; 大細本字=>tuā-suè-pún-jī:AMB; 會=>huē:1char; 去=>khì:1char; 若=>nā:1char; 切=>tshiat:1char; 羅馬字=>lô-má-jī:AMB; 著=>tio̍h:1char; 會=>huē:1char


## `tpsAutoCorrect`

### `features[tpsAutoCorrect].title` — Tâi-lô conf: HIGH

- 漢字: 方音符號自動校正
- Tâi-lô: hong-im-hû-hō tsū-tōng kàu-tsìng
- POJ: hong-im-hû-hō chū-tōng kàu-chèng
- English: Phonetic-symbol auto-correction
- 日本語: 方音符号の自動校正

### `features[tpsAutoCorrect].paragraphs[0].text` — Tâi-lô conf: LOW

- 漢字: 方音符號模式有自動校正功能，會自動修正符號，予你拍字較緊閣較正確。
- Tâi-lô: hong-im-hû-hō bôo-sik ū tsū-tōng kàu-tsìng kong-lîng， huē tsū-tōng siu-tsìng hû-hō， hōo-lí phah-jī khah-kín koh-khah tsìng-khak。
- POJ: hong-im-hû-hō bô͘-sek ū chū-tōng kàu-chèng kong-lêng， hōe chū-tōng siu-chèng hû-hō， hō͘-lí phah-jī khah-kín koh-khah chèng-khak。
- English: Phonetic-symbol mode has auto-correction that fixes symbols automatically, so you can type faster and more accurately.
- 日本語: 方音符号モードには自動校正機能があり、符号を自動で修正するので、より速く正確に入力できます。
- flags: 有=>ū:1char; 會=>huē:1char; 予你=>hōo-lí:AMB; 拍字=>phah-jī:AMB

### `features[tpsAutoCorrect].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 【齒音→顎化音】拍 ㄧ 或 ㆪ ê 時陣，頭前 ê 齒音會自動改做顎化音：ㄗ→ㄐ、ㄘ→ㄑ、ㄙ→ㄒ、ㆡ→ㆢ。譬論欲拍「錢 tsînn」，拍 ㄗ＋ㆪ 會自動變做 ㄐㆪ，免家己揤 ㄐ。
- Tâi-lô: 【 khí-im→ kok huà im】 phah ㄧ hi̍k ㆪ ê sî-sūn， thâu-tsîng ê khí-im huē tsū-tōng kué-tsuè kok huà im：ㄗ→ㄐ、ㄘ→ㄑ、ㄙ→ㄒ、ㆡ→ㆢ。 phì-lūn beh phah「tsînn tsînn」， phah ㄗ＋ㆪ huē tsū-tōng piàn-tsò ㄐㆪ， bián ka-kī tshi̍h ㄐ。
- POJ: 【 khí-im→ kok hòa im】 phah ㄧ he̍k ㆪ ê sî-sūn， thâu-chêng ê khí-im hōe chū-tōng kóe-chòe kok hòa im：ㄗ→ㄐ、ㄘ→ㄑ、ㄙ→ㄒ、ㆡ→ㆢ。 phì-lūn beh phah「chîⁿ chîⁿ」， phah ㄗ＋ㆪ hōe chū-tōng piàn-chò ㄐㆪ， bián ka-kī chhi̍h ㄐ。
- English: [Dental → palatal] When you type ㄧ or ㆪ, the preceding dental consonant is automatically changed to a palatal one: ㄗ→ㄐ, ㄘ→ㄑ, ㄙ→ㄒ, ㆡ→ㆢ. For example, to type “錢 tsînn,” typing ㄗ + ㆪ automatically becomes ㄐㆪ, so you don't have to press ㄐ yourself.
- 日本語: 【歯音→口蓋音】ㄧ または ㆪ を打つと、その前の歯音が自動的に口蓋音に変わります：ㄗ→ㄐ、ㄘ→ㄑ、ㄙ→ㄒ、ㆡ→ㆢ。例えば「錢 tsînn」を打つ場合、ㄗ＋ㆪ が自動的に ㄐㆪ になるので、自分で ㄐ を押す必要はありません。
- flags: 顎=>kok:1char; 化=>huà:1char; 音=>im:1char; 拍=>phah:1char; 或=>hi̍k:1char; 時陣=>sî-sūn:AMB; 頭前=>thâu-tsîng:AMB; 會=>huē:1char; 顎=>kok:1char; 化=>huà:1char; 音=>im:1char; 欲=>beh:1char; 拍=>phah:1char; 錢=>tsînn:1char; 拍=>phah:1char; 會=>huē:1char; 變做=>piàn-tsò:AMB; 免=>bián:1char; 家己=>ka-kī:AMB; 揤=>tshi̍h:1char

### `features[tpsAutoCorrect].paragraphs[2].text` — Tâi-lô conf: MED

- 漢字: 【聲母／韻尾自動切換】仝 1 粒揤鈕，佇音節頭是聲母，佇音節中間會自動變做韻尾：ㄇ→ㆬ、ㄋ→ㄣ、ㄫ→ㆭ；入聲韻尾嘛仝款：ㄅ→ㆴ、ㄉ→ㆵ、ㄍ→ㆻ、ㄏ→ㆷ。譬論拍「心 sim」，拍 ㄙ＋ㄧ＋ㄇ，尾字 ㄇ 會自動變做 ㆬ。
- Tâi-lô: 【 siann-bó／ ūn-bué tsū-tōng tshiat-uānn】 kāng 1 lia̍p tshi̍h-liú， tī im-tsiat thâu sī siann-bó， tī im-tsiat tiong-kan huē tsū-tōng piàn-tsò ūn-bué：ㄇ→ㆬ、ㄋ→ㄣ、ㄫ→ㆭ； ji̍p-siann ūn-bué mā kāng-khuán：ㄅ→ㆴ、ㄉ→ㆵ、ㄍ→ㆻ、ㄏ→ㆷ。 phì-lūn phah「sim sim」， phah ㄙ＋ㄧ＋ㄇ， bué jī ㄇ huē tsū-tōng piàn-tsò ㆬ。
- POJ: 【 siaⁿ-bó／ ūn-bóe chū-tōng chhiat-ōaⁿ】 kāng 1 lia̍p chhi̍h-liú， tī im-chiat thâu sī siaⁿ-bó， tī im-chiat tiong-kan hōe chū-tōng piàn-chò ūn-bóe：ㄇ→ㆬ、ㄋ→ㄣ、ㄫ→ㆭ； ji̍p-siaⁿ ūn-bóe mā kāng-khoán：ㄅ→ㆴ、ㄉ→ㆵ、ㄍ→ㆻ、ㄏ→ㆷ。 phì-lūn phah「sim sim」， phah ㄙ＋ㄧ＋ㄇ， bóe jī ㄇ hōe chū-tōng piàn-chò ㆬ。
- English: [Initial / coda auto-switching] The same key acts as an initial at the start of a syllable and automatically becomes a coda in the middle: ㄇ→ㆬ, ㄋ→ㄣ, ㄫ→ㆭ; the entering-tone codas likewise: ㄅ→ㆴ, ㄉ→ㆵ, ㄍ→ㆻ, ㄏ→ㆷ. For example, typing “心 sim” as ㄙ＋ㄧ＋ㄇ automatically turns the final ㄇ into ㆬ.
- 日本語: 【声母／末子音の自動切り替え】同じキーが、音節の頭では声母、音節の途中では自動的に末子音になります：ㄇ→ㆬ、ㄋ→ㄣ、ㄫ→ㆭ。入声の末子音も同様です：ㄅ→ㆴ、ㄉ→ㆵ、ㄍ→ㆻ、ㄏ→ㆷ。例えば「心 sim」を ㄙ＋ㄧ＋ㄇ と打つと、末尾の ㄇ が自動的に ㆬ になります。
- flags: 聲母=>siann-bó:AMB; 韻尾=>ūn-bué:AMB; 仝=>kāng:1char; 粒=>lia̍p:1char; 佇=>tī:1char; 音節=>im-tsiat:AMB; 頭=>thâu:1char; 是=>sī:1char; 聲母=>siann-bó:AMB; 佇=>tī:1char; 音節=>im-tsiat:AMB; 會=>huē:1char; 變做=>piàn-tsò:AMB; 韻尾=>ūn-bué:AMB; 入聲=>ji̍p-siann:AMB; 韻尾=>ūn-bué:AMB; 嘛=>mā:1char; 拍=>phah:1char; 心=>sim:1char; 拍=>phah:1char; 尾=>bué:1char; 字=>jī:1char; 會=>huē:1char; 變做=>piàn-tsò:AMB

### `features[tpsAutoCorrect].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 【鼻化韻自動修正】拍 ㄧ 了後揤 ㆮ（ainn），會自動改做 ㆯ（aunn），因為台語無「iainn」這个韻母，干焦「iaunn」才是正確 ê。
- Tâi-lô: 【 phīnn huà ūn tsū-tōng siu-tsìng】 phah ㄧ liáu-āu tshi̍h ㆮ（ainn）， huē tsū-tōng kué-tsuè ㆯ（aunn）， in-uī tâi-gí bô「iainn」 tsit ê ūn-bó， kan-na「iaunn」 tsiah-sī tsìng-khak ê。
- POJ: 【 phīⁿ hòa ūn chū-tōng siu-chèng】 phah ㄧ liáu-āu chhi̍h ㆮ（aiⁿ）， hōe chū-tōng kóe-chòe ㆯ（auⁿ）， in-ūi tâi-gí bô「iainn」 chit ê ūn-bó， kan-na「iauⁿ」 chiah-sī chèng-khak ê。
- English: [Nasal-rime auto-correction] After typing ㄧ, pressing ㆮ (ainn) is automatically changed to ㆯ (aunn), because Taiwanese has no rime “iainn”—only “iaunn” is correct.
- 日本語: 【鼻音韻の自動修正】ㄧ を打った後に ㆮ（ainn）を押すと、自動的に ㆯ（aunn）に変わります。台湾語には「iainn」という韻母がなく、「iaunn」だけが正しいからです。
- flags: 鼻=>phīnn:1char; 化=>huà:1char; 韻=>ūn:1char; 拍=>phah:1char; 揤=>tshi̍h:1char; 會=>huē:1char; 台語=>tâi-gí:AMB; 無=>bô:1char; 韻母=>ūn-bó:AMB; 干焦=>kan-na:AMB


## `nextWord`

### `features[nextWord].title` — Tâi-lô conf: LOW

- 漢字: 連紲建議詞
- Tâi-lô: liân-suà kiàn-gī sû
- POJ: liân-sòa kiàn-gī sû
- English: Next-word suggestions
- 日本語: 次候補の予測
- flags: 詞=>sû:1char

### `features[nextWord].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 拍字會連紲建議，譬論講拍「𠢕」這个字，齒盤會出現連紲適合 ê 詞：「𠢕」⭢「早」⭢「gâu-tsá」，毋免逐个音節攏家己拍。
- Tâi-lô: phah-jī huē liân-suà kiàn-gī， phì-lūn-kóng phah「gâu」 tsit ê jī， khí-puânn huē tshut-hiān liân-suà sik-ha̍p ê sû：「gâu」⭢「tsá」⭢「gâu-tsá」， m̄-bián ta̍k ê im-tsiat láng ka-kī phah。
- POJ: phah-jī hōe liân-sòa kiàn-gī， phì-lūn-kóng phah「gâu」 chit ê jī， khí-pôaⁿ hōe chhut-hiān liân-sòa sek-ha̍p ê sû：「gâu」⭢「chá」⭢「gâu-chá」， m̄-bián ta̍k ê im-chiat láng ka-kī phah。
- English: As you type, the keyboard keeps suggesting next words. For example, after typing “𠢕,” the keyboard offers fitting follow-up words: “𠢕” ⭢ “早” ⭢ “gâu-tsá,” so you don't have to type every syllable yourself.
- 日本語: 入力すると、キーボードが続けて候補を提案します。例えば「𠢕」を打つと、続く適切な語が表示されます：「𠢕」⭢「早」⭢「gâu-tsá」。各音節を自分で打つ必要はありません。
- flags: 拍字=>phah-jī:AMB; 會=>huē:1char; 拍=>phah:1char; 𠢕=>gâu:1char; 字=>jī:1char; 會=>huē:1char; 詞=>sû:1char; 𠢕=>gâu:1char; 早=>tsá:1char; 音節=>im-tsiat:AMB; 攏=>láng:1char; 家己=>ka-kī:AMB; 拍=>phah:1char

### `features[nextWord].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 詞庫無 ê 字，若拍過 1 改，後擺著會自動出現佇「連紲建議詞」，譬論拍「我想欲食飯」，以後著會記起來。
- Tâi-lô: sû khòo bô ê jī， nā phah kuè 1 kái， āu-pái tio̍h huē tsū-tōng tshut-hiān tī「liân-suà kiàn-gī sû」， phì-lūn phah「guá siūnn-beh tsia̍h-pn̄g」， í-āu tio̍h huē kì--khí-lâi。
- POJ: sû khò͘ bô ê jī， nā phah kòe 1 kái， āu-pái tio̍h hōe chū-tōng chhut-hiān tī「liân-sòa kiàn-gī sû」， phì-lūn phah「góa siūⁿ-beh chia̍h-pn̄g」， í-āu tio̍h hōe kì--khí-lâi。
- English: Words not in the dictionary, once typed, will automatically appear in “Next-word suggestions” afterward. For example, after typing “我想欲食飯” once, it will be remembered.
- 日本語: 辞書にない語でも、一度入力すると次回から自動的に「次候補」に表示されます。例えば「我想欲食飯」と入力すると、以後は記憶されます。
- flags: 詞=>sû:1char; 庫=>khòo:1char; 無=>bô:1char; 字=>jī:1char; 若=>nā:1char; 拍=>phah:1char; 過=>kuè:1char; 改=>kái:1char; 著=>tio̍h:1char; 會=>huē:1char; 佇=>tī:1char; 詞=>sû:1char; 拍=>phah:1char; 我=>guá:1char; 想欲=>siūnn-beh:AMB; 著=>tio̍h:1char; 會=>huē:1char

### `features[nextWord].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 拍羅馬字時，空格縫佮連劃愛家己揤。若「自動空白」開關有切開，空格縫會自動加，免家己加。
- Tâi-lô: phah lô-má-jī sî， khang-keh phāng kah liân-ue̍h ài ka-kī tshi̍h。 nā「tsū-tōng khàng-pe̍h」 khai-kuan ū tshiat-khui， khang-keh phāng huē tsū-tōng ka， bián ka-kī ka。
- POJ: phah lô-má-jī sî， khang-keh phāng kah liân-oe̍h ài ka-kī chhi̍h。 nā「chū-tōng khàng-pe̍h」 khai-koan ū chhiat-khui， khang-keh phāng hōe chū-tōng ka， bián ka-kī ka。
- English: When typing romanization, you have to enter spaces and hyphens yourself. If the “Auto-space” toggle is on, spaces are added automatically, so you don't have to.
- 日本語: ローマ字を入力するときは、空白とハイフンを自分で打つ必要があります。「自動スペース」スイッチをオンにすると、空白が自動で追加されるので、自分で入れる必要はありません。
- flags: 拍=>phah:1char; 羅馬字=>lô-má-jī:AMB; 時=>sî:1char; 縫=>phāng:1char; 佮=>kah:1char; 連劃=>liân-ue̍h:AMB; 愛=>ài:1char; 家己=>ka-kī:AMB; 揤=>tshi̍h:1char; 若=>nā:1char; 有=>ū:1char; 縫=>phāng:1char; 會=>huē:1char; 加=>ka:1char; 免=>bián:1char; 家己=>ka-kī:AMB; 加=>ka:1char


## `customFont`

### `features[customFont].title` — Tâi-lô conf: LOW

- 漢字: 詞庫管理
- Tâi-lô: sû khòo kuán-lí
- POJ: sû khò͘ koán-lí
- English: Dictionary management
- 日本語: 辞書管理
- flags: 詞=>sû:1char; 庫=>khòo:1char

### `features[customFont].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 齒盤預設使用「教育部用字」詞庫，包括「教育部臺灣台語常用詞辭典」、「公視台語台台語新詞辭庫」、「教育部學科術語臺灣台語對譯」、「工藝中心臺灣台語工藝詞庫」，閣有其他辭典好選，無仝詞庫收錄 ê 字各有特色。
- Tâi-lô: khí-puânn ī-siat sú-iōng「kàu-io̍k-pōo iōng-jī」 sû khòo， pau-kuat「kàu-io̍k-pōo tâi-uân-tâi-gí siâng-iōng-sû sû-tián」、「kong-sī tâi-gí-tâi tâi-gí sin sû sî khòo」、「kàu-io̍k-pōo ha̍k-kho su̍t-gí tâi-uân-tâi-gí tuì-i̍k」、「kang-gē tiong-sim tâi-uân-tâi-gí kang-gē sû khòo」， koh-ū kî-thann sû-tián hó suán， bô kâng sû khòo siu-lio̍k ê jī kok ū ti̍k-sik。
- POJ: khí-pôaⁿ ī-siat sú-iōng「kàu-io̍k-pō͘ iōng-jī」 sû khò͘， pau-koat「kàu-io̍k-pō͘ tâi-oân-tâi-gí siâng-iōng-sû sû-tián」、「kong-sī tâi-gí-tâi tâi-gí sin sû sî khò͘」、「kàu-io̍k-pō͘ ha̍k-kho su̍t-gí tâi-oân-tâi-gí tùi-e̍k」、「kang-gē tiong-sim tâi-oân-tâi-gí kang-gē sû khò͘」， koh-ū kî-thaⁿ sû-tián hó soán， bô kâng sû khò͘ siu-lio̍k ê jī kok ū te̍k-sek。
- English: By default the keyboard uses the “MOE wordings” dictionary, including the “MOE Dictionary of Frequently-Used Taiwan Taiwanese,” the “PTS Taigi Channel New-Word Lexicon,” the “MOE Academic-Term Taiwan Taiwanese Translation,” and the “National Taiwan Craft Research Center Taiwan Taiwanese Craft Lexicon,” with other dictionaries also available. Each dictionary has its own characteristic entries.
- 日本語: キーボードは既定で「教育部用字」辞書を使用します。これには「教育部臺灣台語常用詞辭典」「公視台語台台語新詞辭庫」「教育部學科術語臺灣台語對譯」「工藝中心臺灣台語工藝詞庫」が含まれ、他の辞書も選べます。辞書ごとに収録する字に特色があります。
- flags: 預設=>ī-siat:AMB; 詞=>sû:1char; 庫=>khòo:1char; 常用詞=>siâng-iōng-sû:AMB; 台語台=>tâi-gí-tâi:AMB; 台語=>tâi-gí:AMB; 新=>sin:1char; 詞=>sû:1char; 辭=>sî:1char; 庫=>khòo:1char; 術語=>su̍t-gí:AMB; 詞=>sû:1char; 庫=>khòo:1char; 好=>hó:1char; 選=>suán:1char; 無仝=>bô kâng:AMB; 詞=>sû:1char; 庫=>khòo:1char; 收錄=>siu-lio̍k:AMB; 字=>jī:1char; 各=>kok:1char; 有=>ū:1char

### `features[customFont].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 若感覺候選詞傷濟，會使共無用著 ê 辭典關起來，按呢候選詞著會較少，想欲揣 ê 字會較佇頭前。
- Tâi-lô: nā kám-kak hāu-suán sû siunn-tse， ē-sái kā bô-iōng tio̍h ê sû-tián kuainn--khí-lâi， án-ne hāu-suán sû tio̍h huē khah siàu， siūnn-beh tshuē ê jī huē khah tī thâu-tsîng。
- POJ: nā kám-kak hāu-soán sû siuⁿ-che， ē-sái kā bô-iōng tio̍h ê sû-tián koaiⁿ--khí-lâi， án-ne hāu-soán sû tio̍h hōe khah siàu， siūⁿ-beh chhōe ê jī hōe khah tī thâu-chêng。
- English: If there are too many candidates, you can turn off dictionaries you don't use; then there will be fewer candidates, and the character you want will be nearer the front.
- 日本語: 候補が多すぎると感じたら、使わない辞書をオフにできます。そうすると候補が減り、探している字が前のほうに来ます。
- flags: 若=>nā:1char; 詞=>sû:1char; 傷濟=>siunn-tse:AMB; 會使=>ē-sái:AMB; 共=>kā:1char; 著=>tio̍h:1char; 按呢=>án-ne:AMB; 詞=>sû:1char; 著=>tio̍h:1char; 會=>huē:1char; 較=>khah:1char; 少=>siàu:1char; 想欲=>siūnn-beh:AMB; 揣=>tshuē:1char; 字=>jī:1char; 會=>huē:1char; 較=>khah:1char; 佇=>tī:1char; 頭前=>thâu-tsîng:AMB

### `features[customFont].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 「詞庫」頁面嘛有「拍字揣詞」ê 功能，拍漢字抑是羅馬字攏會使揣，方便確認齒盤內底有收錄啥物詞，嘛會使連去線頂辭典查。
- Tâi-lô: 「sû khòo」 ia̍h-bīn mā-ū「phah-jī tshuē sû」ê kong-lîng， phah hàn-jī ah-sī lô-má-jī lóng-ē-sài tshuē， hong-piān khak-jīn khí-puânn lāi-té ū siu-lio̍k siánn-mih sû， mā ē-sái nî khì suànn-tíng sû-tián tsa。
- POJ: 「sû khò͘」 ia̍h-bīn mā-ū「phah-jī chhōe sû」ê kong-lêng， phah hàn-jī ah-sī lô-má-jī lóng-ē-sài chhōe， hong-piān khak-jīn khí-pôaⁿ lāi-té ū siu-lio̍k siáⁿ-mih sû， mā ē-sái nî khì sòaⁿ-téng sû-tián cha。
- English: The “Dictionary” page also has a “Search words by typing” feature; you can search by Hanji or romanization, making it easy to check which words the keyboard includes, and you can also link out to an online dictionary.
- 日本語: 「辞書」ページには「入力して語を検索」機能もあり、漢字でもローマ字でも検索できます。キーボードにどの語が収録されているか確認するのに便利で、オンライン辞書へのリンクもできます。
- flags: 詞=>sû:1char; 庫=>khòo:1char; 拍字=>phah-jī:AMB; 揣=>tshuē:1char; 詞=>sû:1char; 拍=>phah:1char; 漢字=>hàn-jī:AMB; 抑是=>ah-sī:AMB; 羅馬字=>lô-má-jī:AMB; 攏會使=>lóng-ē-sài:AMB; 揣=>tshuē:1char; 確認=>khak-jīn:AMB; 內底=>lāi-té:AMB; 有=>ū:1char; 收錄=>siu-lio̍k:AMB; 啥物=>siánn-mih:AMB; 詞=>sû:1char; 嘛=>mā:1char; 會使=>ē-sái:AMB; 連=>nî:1char; 去=>khì:1char; 查=>tsa:1char

### `features[customFont].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 辭典 ê 詞是半自動、半人工校對，若有拄著問題請回報予我知。
- Tâi-lô: sû-tián ê sû sī puànn tsū-tōng、 puànn-lâng kang kàu-tuì， nā-ū tú-tio̍h būn-tê tshiánn huê-pò hōo--guá tsai。
- POJ: sû-tián ê sû sī pòaⁿ chū-tōng、 pòaⁿ-lâng kang kàu-tùi， nā-ū tú-tio̍h būn-tê chhiáⁿ hôe-pò hō͘--góa chai。
- English: The dictionary entries are proofread half-automatically and half-manually; if you run into a problem, please report it to me.
- 日本語: 辞書の語は半自動・半手動で校正しています。問題があれば私まで報告してください。
- flags: 詞=>sû:1char; 是=>sī:1char; 半=>puànn:1char; 工=>kang:1char; 問題=>būn-tê:AMB; 請=>tshiánn:1char; 回報=>huê-pò:AMB; 知=>tsai:1char


## `userDict`

### `features[userDict].title` — Tâi-lô conf: MED

- 漢字: 拍字記持詞庫
- Tâi-lô: phah-jī kì-tî sû khòo
- POJ: phah-jī kì-tî sû khò͘
- English: Typing-memory dictionary
- 日本語: 入力記憶辞書
- flags: 拍字=>phah-jī:AMB; 詞=>sû:1char; 庫=>khòo:1char

### `features[userDict].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 台語齒盤有 1 个「拍字記持詞庫」，拍過 ê 字攏會記起來，較捷拍 ê 詞會出現佇頭前，愈久無拍 ê 詞會沓沓仔排佇後壁。白話字佮台羅 ê 詞攏會記。
- Tâi-lô: tâi-gí khí-puânn ū 1 ê「phah-jī kì-tî sû khòo」， phah kuè ê jī láng huē kì--khí-lâi， khah tsia̍p phah ê sû huē tshut-hiān tī thâu-tsîng， jú kú bô phah ê sû huē ta̍uh-ta̍uh-á pâi tī āu-piah。 pe̍h-uē-jī kah tâi-lô ê sû láng huē kì。
- POJ: tâi-gí khí-pôaⁿ ū 1 ê「phah-jī kì-tî sû khò͘」， phah kòe ê jī láng hōe kì--khí-lâi， khah chia̍p phah ê sû hōe chhut-hiān tī thâu-chêng， jú kú bô phah ê sû hōe tau̍h-tau̍h-á pâi tī āu-piah。 pe̍h-ōe-jī kah tâi-lô ê sû láng hōe kì。
- English: Taigi Keyboard has a “Typing-memory dictionary” that remembers what you've typed. Words you type more often appear nearer the front, while words you haven't typed in a while gradually move to the back. Both Pe̍h-ōe-jī and Tâi-lô words are remembered.
- 日本語: 台語キーボードには「入力記憶辞書」があり、入力した字を記憶します。よく打つ語は前に表示され、長く打っていない語は次第に後ろへ移動します。白話字と台羅のどちらの語も記憶されます。
- flags: 台語=>tâi-gí:AMB; 有=>ū:1char; 个=>ê:1char; 拍字=>phah-jī:AMB; 詞=>sû:1char; 庫=>khòo:1char; 拍=>phah:1char; 過=>kuè:1char; 字=>jī:1char; 攏=>láng:1char; 會=>huē:1char; 較=>khah:1char; 捷=>tsia̍p:1char; 拍=>phah:1char; 詞=>sû:1char; 會=>huē:1char; 佇=>tī:1char; 頭前=>thâu-tsîng:AMB; 愈=>jú:1char; 久=>kú:1char; 無=>bô:1char; 拍=>phah:1char; 詞=>sû:1char; 會=>huē:1char; 排=>pâi:1char; 佇=>tī:1char; 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 詞=>sû:1char; 攏=>láng:1char; 會=>huē:1char; 記=>kì:1char

### `features[userDict].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 這个詞庫是予「連紲建議詞」使用 ê，會根據你當時 ê 輸入模式顯示對應 ê 詞，譬論佇白話字模式著顯示白話字 ê 詞，台羅模式著顯示台羅 ê 詞。
- Tâi-lô: tsit ê sû khòo sī hōo「liân-suà kiàn-gī sû」 sú-iōng ê， huē kin-kì lí tang-sî ê su-ji̍p bôo-sik hián-sī tuì-ìng ê sû， phì-lūn tī pe̍h-uē-jī bôo-sik tio̍h hián-sī pe̍h-uē-jī ê sû， tâi-lô bôo-sik tio̍h hián-sī tâi-lô ê sû。
- POJ: chit ê sû khò͘ sī hō͘「liân-sòa kiàn-gī sû」 sú-iōng ê， hōe kin-kì lí tang-sî ê su-ji̍p bô͘-sek hián-sī tùi-èng ê sû， phì-lūn tī pe̍h-ōe-jī bô͘-sek tio̍h hián-sī pe̍h-ōe-jī ê sû， tâi-lô bô͘-sek tio̍h hián-sī tâi-lô ê sû。
- English: This dictionary is used by “Next-word suggestions” and shows words matching your current input mode—e.g. it shows Pe̍h-ōe-jī words in Pe̍h-ōe-jī mode and Tâi-lô words in Tâi-lô mode.
- 日本語: この辞書は「次候補」のために使われ、その時点の入力モードに応じた語を表示します。例えば白話字モードでは白話字の語、台羅モードでは台羅の語を表示します。
- flags: 詞=>sû:1char; 庫=>khòo:1char; 是=>sī:1char; 予=>hōo:1char; 詞=>sû:1char; 會=>huē:1char; 根據=>kin-kì:AMB; 你=>lí:1char; 當時=>tang-sî:AMB; 輸入=>su-ji̍p:AMB; 詞=>sû:1char; 佇=>tī:1char; 白話字=>pe̍h-uē-jī:AMB; 著=>tio̍h:1char; 白話字=>pe̍h-uē-jī:AMB; 詞=>sû:1char; 著=>tio̍h:1char; 詞=>sû:1char

### `features[userDict].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 詞出現 ê 順序是照「拍過幾改」佮「偌久無拍」來排 ê，若感覺想欲用 ê 字攏揣無，請回報予我知。
- Tâi-lô: sû tshut-hiān ê sūn-sū sī tsiàu「phah kuè kuí kái」 kah「guā-kú bô phah」 lâi pâi ê， nā kám-kak siūnn-beh iōng ê jī láng tshuē-bô， tshiánn huê-pò hōo--guá tsai。
- POJ: sû chhut-hiān ê sūn-sū sī chiàu「phah kòe kúi kái」 kah「gōa-kú bô phah」 lâi pâi ê， nā kám-kak siūⁿ-beh iōng ê jī láng chhōe-bô， chhiáⁿ hôe-pò hō͘--góa chai。
- English: The order in which words appear is based on how many times you've typed them and how long it's been since you last typed them. If you find that the character you want never shows up, please report it to me.
- 日本語: 語が表示される順序は、「何回打ったか」と「どのくらい打っていないか」で決まります。使いたい字がどうしても出てこない場合は、私まで報告してください。
- flags: 詞=>sû:1char; 順序=>sūn-sū:AMB; 是=>sī:1char; 照=>tsiàu:1char; 拍=>phah:1char; 過=>kuè:1char; 幾=>kuí:1char; 改=>kái:1char; 佮=>kah:1char; 偌久=>guā-kú:AMB; 無=>bô:1char; 拍=>phah:1char; 來=>lâi:1char; 排=>pâi:1char; 若=>nā:1char; 想欲=>siūnn-beh:AMB; 用=>iōng:1char; 字=>jī:1char; 攏=>láng:1char; 請=>tshiánn:1char; 回報=>huê-pò:AMB; 知=>tsai:1char


## `variant`

### `features[variant].title` — Tâi-lô conf: LOW

- 漢字: 異用字開關
- Tâi-lô: ī iōng-jī khai-kuan
- POJ: ī iōng-jī khai-koan
- English: Variant-character toggle
- 日本語: 異用字スイッチ
- flags: 異=>ī:1char

### `features[variant].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 依據教典 ê 資料標示台語異用字，譬論：「人」⭢「儂」、「生」⭢「青」。這个開關預設是關起來，有需要才開。
- Tâi-lô: i-kì kàu-tián ê tsu-liāu phiau-sī tâi-gí ī iōng-jī， phì-lūn：「jîn」⭢「lâng」、「sing」⭢「tshenn」。 tsit ê khai-kuan ī-siat sī kuainn--khí-lâi， ū su-iàu tsâi khui。
- POJ: i-kì kàu-tián ê chu-liāu phiau-sī tâi-gí ī iōng-jī， phì-lūn：「jîn」⭢「lâng」、「seng」⭢「chheⁿ」。 chit ê khai-koan ī-siat sī koaiⁿ--khí-lâi， ū su-iàu châi khui。
- English: Marks Taiwanese variant characters based on data from the MOE Dictionary, e.g. “人” ⭢ “儂,” “生” ⭢ “青.” This toggle is off by default; turn it on only if needed.
- 日本語: 教典（教育部辞典）のデータに基づいて台湾語の異用字を示します。例：「人」⭢「儂」、「生」⭢「青」。このスイッチは既定でオフで、必要なときだけオンにします。
- flags: 依據=>i-kì:AMB; 標示=>phiau-sī:AMB; 台語=>tâi-gí:AMB; 異=>ī:1char; 人=>jîn:1char; 儂=>lâng:1char; 生=>sing:1char; 青=>tshenn:1char; 預設=>ī-siat:AMB; 是=>sī:1char; 有=>ū:1char; 才=>tsâi:1char; 開=>khui:1char

### `features[variant].paragraphs[0].attachment.text` — Tâi-lô conf: LOW

- 漢字: 揤遮到教典網站掠辭典資料
- Tâi-lô: tshi̍h jia kàu kàu-tián bāng-tsām lia̍h sû-tián tsu-liāu
- POJ: chhi̍h jia kàu kàu-tián bāng-chām lia̍h sû-tián chu-liāu
- English: Tap here to get dictionary data from the MOE Dictionary website
- 日本語: ここをタップして教典サイトから辞書データを取得
- flags: 揤=>tshi̍h:1char; 遮=>jia:1char; 到=>kàu:1char; 掠=>lia̍h:1char

### `features[variant].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 因為教典異用字 ê 資料猶未齊全，有 ê 字可能無標示著，若有發現請回報予我知。
- Tâi-lô: in-uī kàu-tián ī iōng-jī ê tsu-liāu ah-buē tsiâu-tsn̂g， ū ê jī khó-lîng bô phiau-sī tio̍h， nā-ū huat-hiān tshiánn huê-pò hōo--guá tsai。
- POJ: in-ūi kàu-tián ī iōng-jī ê chu-liāu ah-bōe chiâu-chn̂g， ū ê jī khó-lêng bô phiau-sī tio̍h， nā-ū hoat-hiān chhiáⁿ hôe-pò hō͘--góa chai。
- English: Because the MOE Dictionary's variant-character data is not yet complete, some characters may not be marked. If you notice any, please report them to me.
- 日本語: 教典の異用字データはまだ完全ではないため、一部の字が示されないことがあります。お気づきの際は私まで報告してください。
- flags: 異=>ī:1char; 猶未=>ah-buē:AMB; 齊全=>tsiâu-tsn̂g:AMB; 有=>ū:1char; 字=>jī:1char; 可能=>khó-lîng:AMB; 無=>bô:1char; 標示=>phiau-sī:AMB; 著=>tio̍h:1char; 請=>tshiánn:1char; 回報=>huê-pò:AMB; 知=>tsai:1char


## `appearance`

### `features[appearance].title` — Tâi-lô conf: HIGH

- 漢字: 外觀設定
- Tâi-lô: guā-kuan siat-tīng
- POJ: gōa-koan siat-tēng
- English: Appearance settings
- 日本語: 外観設定

### `features[appearance].paragraphs[0].text` — Tâi-lô conf: LOW

- 漢字: 調齒盤 ê 外觀，包括字骨、色水、揤鈕大細佮形體，調整了會當佇下底 ê 齒盤預覽看著效果。這是我改 ê 「間諜扮公伙仔」配色。
- Tâi-lô: tiâu khí-puânn ê guā-kuan， pau-kuat jī-kut、 sik-tsuí、 tshi̍h-liú tuā-sè kah hîng-thé， tiâu-tsíng liáu ē-tàng tī ē-tué ê khí-puânn ī-lám khuànn-tio̍h hāu-kó。 tse-sī guá kái ê 「kàn-tia̍p pān-kong-hué-á」 phuè-sik。
- POJ: tiâu khí-pôaⁿ ê gōa-koan， pau-koat jī-kut、 sek-chúi、 chhi̍h-liú tōa-sè kah hêng-thé， tiâu-chéng liáu ē-tàng tī ē-tóe ê khí-pôaⁿ ī-lám khòaⁿ-tio̍h hāu-kó。 che-sī góa kái ê 「kàn-tia̍p pān-kong-hóe-á」 phòe-sek。
- English: Adjust the keyboard's appearance, including font, colors, and key size and shape. After adjusting, you can see the effect in the keyboard preview below. This is my take on the “Spy × Family” color scheme.
- 日本語: フォント、色、キーの大きさや形など、キーボードの外観を調整できます。調整後は、下のキーボードプレビューで効果を確認できます。これは私が手を加えた「SPY×FAMILY」風の配色です。
- flags: 調=>tiâu:1char; 字骨=>jī-kut:AMB; 大細=>tuā-sè:AMB; 佮=>kah:1char; 了=>liáu:1char; 會當=>ē-tàng:AMB; 佇=>tī:1char; 下底=>ē-tué:AMB; 預覽=>ī-lám:AMB; 看著=>khuànn-tio̍h:AMB; 我=>guá:1char; 改=>kái:1char; 扮公伙仔=>pān-kong-hué-á:AMB; 配色=>phuè-sik:AMB

### `features[appearance].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 字型有 5 種通好揀：系統預設、粉圓、芫荽、源樣明體、源樣烏體。後 4 種是專門支援台語特殊字符 ê 開源字型，若揀系統預設，有 ê 漢字無法度顯示。
- Tâi-lô: jī-hîng ū 5 tsíng thang-hó kíng： hē-thóng ī-siat、 hún-înn、 ian-sui、 guân iūnn bîng-thé、 guân iūnn oo thé。 āu 4 tsíng sī tsuan-bûn tsi-uān tâi-gí ti̍k-sû jī hû ê khai-guân jī-hîng， nā kíng hē-thóng ī-siat， ū ê hàn-jī bô-huat-tōo hián-sī。
- POJ: jī-hêng ū 5 chéng thang-hó kéng： hē-thóng ī-siat、 hún-îⁿ、 ian-sui、 goân iūⁿ bêng-thé、 goân iūⁿ o͘ thé。 āu 4 chéng sī choan-bûn chi-oān tâi-gí te̍k-sû jī hû ê khai-goân jī-hêng， nā kéng hē-thóng ī-siat， ū ê hàn-jī bô-hoat-tō͘ hián-sī。
- English: There are 5 fonts to choose from: System default, Huninn, Iansui, GenYoMin, and GenYoGothic. The latter 4 are open-source fonts that specifically support Taiwanese special characters; if you choose the system default, some Hanji may not display.
- 日本語: フォントは5種類から選べます：システム既定、粉圓（Huninn）、芫荽（Iansui）、源樣明體（GenYoMin）、源樣黑體（GenYoGothic）。後ろの4つは台湾語の特殊文字に対応したオープンソースフォントです。システム既定を選ぶと、一部の漢字が表示されないことがあります。
- flags: 字型=>jī-hîng:AMB; 有=>ū:1char; 種=>tsíng:1char; 揀=>kíng:1char; 預設=>ī-siat:AMB; 芫荽=>ian-sui:AMB; 源=>guân:1char; 樣=>iūnn:1char; 源=>guân:1char; 樣=>iūnn:1char; 烏=>oo:1char; 體=>thé:1char; 後=>āu:1char; 種=>tsíng:1char; 是=>sī:1char; 台語=>tâi-gí:AMB; 字=>jī:1char; 符=>hû:1char; 字型=>jī-hîng:AMB; 若=>nā:1char; 揀=>kíng:1char; 預設=>ī-siat:AMB; 有=>ū:1char; 漢字=>hàn-jī:AMB

### `features[appearance].paragraphs[2].text` — Tâi-lô conf: MED

- 漢字: 色水會使改齒盤背景、揤鈕色、字色佮候選欄 ê 色水，逐个攏有色盤通好選，若改了無合意，揤重設揤鈕著會轉去預設值。
- Tâi-lô: sik-tsuí ē-sái kái khí-puânn puē-kíng、 tshi̍h-liú sik、 jī sik kah hāu-suán nuâ ê sik-tsuí， ta̍k ê lóng-ū sik puânn thang-hó suán， nā kái liáu bô-ha̍h ì， tshi̍h tāng siat tshi̍h-liú tio̍h huē tńg--khì ī-siat ta̍t。
- POJ: sek-chúi ē-sái kái khí-pôaⁿ pōe-kéng、 chhi̍h-liú sek、 jī sek kah hāu-soán nôa ê sek-chúi， ta̍k ê lóng-ū sek pôaⁿ thang-hó soán， nā kái liáu bô-ha̍h ì， chhi̍h tāng siat chhi̍h-liú tio̍h hōe tńg--khì ī-siat ta̍t。
- English: You can change the colors of the keyboard background, keys, text, and candidate bar; each has a color palette to choose from. If you don't like your changes, the reset button returns everything to the default values.
- 日本語: キーボードの背景、キー、文字、候補欄の色を変更できます。それぞれにカラーパレットがあります。変更が気に入らない場合は、リセットボタンで既定値に戻ります。
- flags: 會使=>ē-sái:AMB; 改=>kái:1char; 色=>sik:1char; 字=>jī:1char; 色=>sik:1char; 佮=>kah:1char; 欄=>nuâ:1char; 色=>sik:1char; 盤=>puânn:1char; 選=>suán:1char; 若=>nā:1char; 改=>kái:1char; 了=>liáu:1char; 意=>ì:1char; 揤=>tshi̍h:1char; 重=>tāng:1char; 設=>siat:1char; 著=>tio̍h:1char; 會=>huē:1char; 轉去=>tńg--khì:AMB; 預設=>ī-siat:AMB; 值=>ta̍t:1char

### `features[appearance].paragraphs[3].text` — Tâi-lô conf: MED

- 漢字: 揤鈕懸度、字型大細、圓角攏會使用 at-á 調整，一个一个試看覓，揣著家己上佮意 ê 設定。
- Tâi-lô: tshi̍h-liú kuân-tōo、 jī-hîng tuā-sè、 înn kak lóng-ē-sài iōng at-á tiâu-tsíng， tsi̍t-ê--tsi̍t-ê tshì khuànn-māi， tshuē-tio̍h ka-kī siōng kah-ì ê siat-tīng。
- POJ: chhi̍h-liú koân-tō͘、 jī-hêng tōa-sè、 îⁿ kak lóng-ē-sài iōng at-á tiâu-chéng， chi̍t-ê--chi̍t-ê chhì khòaⁿ-māi， chhōe-tio̍h ka-kī siōng kah-ì ê siat-tēng。
- English: Key height, font size, and corner rounding can all be adjusted with sliders; try them one by one to find the settings you like best.
- 日本語: キーの高さ、フォントの大きさ、角の丸みは、すべてスライダーで調整できます。一つずつ試して、自分の一番好みの設定を見つけてください。
- flags: 字型=>jī-hîng:AMB; 大細=>tuā-sè:AMB; 圓=>înn:1char; 角=>kak:1char; 攏會使=>lóng-ē-sài:AMB; 用=>iōng:1char; 揣著=>tshuē-tio̍h:AMB; 家己=>ka-kī:AMB; 上=>siōng:1char


## `dataManagement`

### `features[dataManagement].title` — Tâi-lô conf: HIGH

- 漢字: 資料管理
- Tâi-lô: tsu-liāu kuán-lí
- POJ: chu-liāu koán-lí
- English: Data management
- 日本語: データ管理

### `features[dataManagement].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 台語齒盤袂收集你 ê 資料傳去別 ê 所在，所有暫時儉 ê 資料攏囥佇你家己 ê 手機仔內底，App 刣掉了後資料著攏會無去。
- Tâi-lô: tâi-gí khí-puânn buē siu-tsi̍p lí ê tsu-liāu thuân khì pia̍t ê sóo-tsāi， sóo-iú tsiām-sî khiām ê tsu-liāu láng khǹg tī lí ka-kī ê tshiú-ki-á lāi-té，App thâi-tiāu liáu-āu tsu-liāu tio̍h láng ē--bô khì。
- POJ: tâi-gí khí-pôaⁿ bōe siu-chi̍p lí ê chu-liāu thoân khì pia̍t ê só͘-chāi， só͘-iú chiām-sî khiām ê chu-liāu láng khǹg tī lí ka-kī ê chhiú-ki-á lāi-té，App thâi-tiāu liáu-āu chu-liāu tio̍h láng ē--bô khì。
- English: Taigi Keyboard does not collect your data or send it anywhere; all temporarily stored data stays on your own phone, and once you delete the app the data is all gone.
- 日本語: 台語キーボードはあなたのデータを収集して外部に送ることはありません。一時的に保存されるデータはすべてあなた自身の端末内にあり、アプリを削除すればデータもすべて消えます。
- flags: 台語=>tâi-gí:AMB; 袂=>buē:1char; 你=>lí:1char; 傳=>thuân:1char; 去=>khì:1char; 別=>pia̍t:1char; 所有=>sóo-iú:AMB; 儉=>khiām:1char; 攏=>láng:1char; 囥=>khǹg:1char; 佇=>tī:1char; 你=>lí:1char; 家己=>ka-kī:AMB; 內底=>lāi-té:AMB; 著=>tio̍h:1char; 攏=>láng:1char; 去=>khì:1char

### `features[dataManagement].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 台語齒盤紀錄 ê 詞頻資料是誠有價值 ê 資料，因為台語誠欠這部份 ê 語料統計。若欲予研究使用，愛會記得先刣掉敏感 ê 詞，嘛毋通凊彩傳予生份人，手機仔嘛莫借予別人使用。
- Tâi-lô: tâi-gí khí-puânn kì-lio̍k ê sû pîn tsu-liāu sī sîng ū-kè ta̍t ê tsu-liāu， in-uī tâi-gí sîng khiàm tse pōo-hūn ê gí liāu thóng-kè。 nā-beh hōo gián-kiù sú-iōng， ài ē-kì--eh sian thâi-tiāu bín-kám ê sû， mā m̄-thang tshìn-tshái thuân hōo senn-hūn-lâng， tshiú-ki-á mā bo̍k tsioh hōo pa̍t-lâng sú-iōng。
- POJ: tâi-gí khí-pôaⁿ kì-lio̍k ê sû pîn chu-liāu sī sêng ū-kè ta̍t ê chu-liāu， in-ūi tâi-gí sêng khiàm che pō͘-hūn ê gí liāu thóng-kè。 nā-beh hō͘ gián-kiù sú-iōng， ài ē-kì--eh sian thâi-tiāu bín-kám ê sû， mā m̄-thang chhìn-chhái thoân hō͘ seⁿ-hūn-lâng， chhiú-ki-á mā bo̍k chioh hō͘ pa̍t-lâng sú-iōng。
- English: The word-frequency data Taigi Keyboard records is very valuable, because Taiwanese sorely lacks this kind of corpus statistics. If you want to use it for research, remember to delete sensitive words first, don't share it carelessly with strangers, and don't lend your phone to others.
- 日本語: 台語キーボードが記録する語頻度データは非常に価値があります。台湾語にはこの種のコーパス統計が大きく不足しているからです。研究に使う場合は、まず機微な語を削除し、知らない人に軽々しく渡さず、端末を他人に貸さないようにしてください。
- flags: 台語=>tâi-gí:AMB; 紀錄=>kì-lio̍k:AMB; 詞=>sû:1char; 頻=>pîn:1char; 是=>sī:1char; 誠=>sîng:1char; 值=>ta̍t:1char; 台語=>tâi-gí:AMB; 誠=>sîng:1char; 欠=>khiàm:1char; 這=>tse:1char; 語=>gí:1char; 料=>liāu:1char; 予=>hōo:1char; 愛=>ài:1char; 會記得=>ē-kì--eh:AMB; 先=>sian:1char; 詞=>sû:1char; 嘛=>mā:1char; 傳=>thuân:1char; 予=>hōo:1char; 生份人=>senn-hūn-lâng:AMB; 嘛=>mā:1char; 莫=>bo̍k:1char; 借=>tsioh:1char; 予=>hōo:1char

### `features[dataManagement].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 「自訂詞庫」嘛仝款，內底莫囥敏感 ê 詞抑是個人資訊，譬論電話、身分證號碼、人名，尤其是欲分享予別人 ê 時陣愛較注意。
- Tâi-lô: 「tsū tīng sû khòo」 mā kāng-khuán， lāi-té bo̍k khǹg bín-kám ê sû ah-sī kò-jîn tsu-sìn， phì-lūn tiān-uē、 sin-hūn-tsìng hō-bé、 lâng-miâ， iû-kî sī beh hun-hióng hōo pa̍t-lâng ê sî-sūn ài khah tsù-ì。
- POJ: 「chū tēng sû khò͘」 mā kāng-khoán， lāi-té bo̍k khǹg bín-kám ê sû ah-sī kò-jîn chu-sìn， phì-lūn tiān-ōe、 sin-hūn-chèng hō-bé、 lâng-miâ， iû-kî sī beh hun-hióng hō͘ pa̍t-lâng ê sî-sūn ài khah chù-ì。
- English: The same goes for the “Custom dictionary”: don't put sensitive words or personal information in it, such as phone numbers, ID numbers, or names—especially be careful when you intend to share it with others.
- 日本語: 「カスタム辞書」も同様です。電話番号、身分証番号、氏名など、機微な語や個人情報を入れないでください。特に他人と共有するときは注意が必要です。
- flags: 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 庫=>khòo:1char; 嘛=>mā:1char; 內底=>lāi-té:AMB; 莫=>bo̍k:1char; 囥=>khǹg:1char; 詞=>sû:1char; 抑是=>ah-sī:AMB; 個人=>kò-jîn:AMB; 是=>sī:1char; 欲=>beh:1char; 分享=>hun-hióng:AMB; 予=>hōo:1char; 時陣=>sî-sūn:AMB; 愛=>ài:1char; 較=>khah:1char

### `features[dataManagement].paragraphs[3].text` — Tâi-lô conf: MED

- 漢字: 台語齒盤有提供資料備份 ê 功能，若驚資料無去，愛記得定期備份較安心。
- Tâi-lô: tâi-gí khí-puânn ū thê-kiong tsu-liāu pī-hūn ê kong-lîng， nā kiann tsu-liāu bô khì， ài kì-tit tīng-kî pī-hūn khah an-sim。
- POJ: tâi-gí khí-pôaⁿ ū thê-kiong chu-liāu pī-hūn ê kong-lêng， nā kiaⁿ chu-liāu bô khì， ài kì-tit tēng-kî pī-hūn khah an-sim。
- English: Taigi Keyboard offers a data backup feature; if you're worried about losing data, remember to back up regularly for peace of mind.
- 日本語: 台語キーボードにはデータのバックアップ機能があります。データが消えるのが心配なら、定期的にバックアップしておくと安心です。
- flags: 台語=>tâi-gí:AMB; 有=>ū:1char; 若=>nā:1char; 驚=>kiann:1char; 無去=>bô khì:AMB; 愛=>ài:1char; 記得=>kì-tit:AMB; 較=>khah:1char


## `taigiConverter`

### `features[taigiConverter].title` — Tâi-lô conf: MED

- 漢字: 台語通用轉換器
- Tâi-lô: tâi-gí thong-iōng tsuán-uānn khì
- POJ: tâi-gí thong-iōng choán-ōaⁿ khì
- English: Taigi Universal Converter
- 日本語: 台語汎用コンバーター
- flags: 台語=>tâi-gí:AMB; 器=>khì:1char

### `features[taigiConverter].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 「台語通用轉換器」是我做 ê 線頂家私，會使共白話字、台羅佮方音符號互相轉換，嘛會使一改共聲調數字轉做聲調符號。
- Tâi-lô: 「tâi-gí thong-iōng tsuán-uānn khì」 sī guá tsuè ê suànn-tíng ke-si， ē-sái kā pe̍h-uē-jī、 tâi-lô kah hong-im-hû-hō hōo-siong tsuán-uānn， mā ē-sái tsi̍t kái kā siann-tiāu sòo-jī tńg tsuè-siann tiāu-hō hō。
- POJ: 「tâi-gí thong-iōng choán-ōaⁿ khì」 sī góa chòe ê sòaⁿ-téng ke-si， ē-sái kā pe̍h-ōe-jī、 tâi-lô kah hong-im-hû-hō hō͘-siong choán-ōaⁿ， mā ē-sái chi̍t kái kā siaⁿ-tiāu sò͘-jī tńg chòe-siaⁿ tiāu-hō hō。
- English: The “Taigi Universal Converter” is an online tool I made; it can convert between Pe̍h-ōe-jī, Tâi-lô, and Taiwanese Phonetic Symbols, and can also convert tone numbers to tone marks in one go.
- 日本語: 「台語汎用コンバーター」は私が作ったオンラインツールで、白話字・台羅・方音符号を相互に変換でき、声調数字を声調記号へまとめて変換することもできます。
- flags: 台語=>tâi-gí:AMB; 器=>khì:1char; 是=>sī:1char; 我=>guá:1char; 做=>tsuè:1char; 會使=>ē-sái:AMB; 共=>kā:1char; 白話字=>pe̍h-uē-jī:AMB; 佮=>kah:1char; 嘛=>mā:1char; 會使=>ē-sái:AMB; 一=>tsi̍t:1char; 改=>kái:1char; 共=>kā:1char; 數字=>sòo-jī:AMB; 轉=>tńg:1char; 做聲=>tsuè-siann:AMB; 號=>hō:1char

### `features[taigiConverter].paragraphs[0].attachment.text` — Tâi-lô conf: LOW

- 漢字: 揤遮去台語通用轉換器
- Tâi-lô: tshi̍h jia khì tâi-gí thong-iōng tsuán-uānn khì
- POJ: chhi̍h jia khì tâi-gí thong-iōng choán-ōaⁿ khì
- English: Tap here to go to the Taigi Universal Converter
- 日本語: ここをタップして台語汎用コンバーターへ
- flags: 揤=>tshi̍h:1char; 遮=>jia:1char; 去=>khì:1char; 台語=>tâi-gí:AMB; 器=>khì:1char

### `features[taigiConverter].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 這个轉換器佮台語齒盤用相仝 ê 核心，所以轉換 ê 結果佮齒盤拍出來 ê 攏仝款。若拍字 ê 時陣對聲調抑是拼法有問題，嘛會使去網站比較看覓。
- Tâi-lô: tsit ê tsuán-uānn khì kah tâi-gí khí-puânn iōng sio-kâng ê hi̍k-sim， sóo-í tsuán-uānn ê kiat-kó kah khí-puânn phah-tshut lâi ê láng kāng-khuán。 nā phah-jī ê sî-sūn tuì siann-tiāu ah-sī ping-huat ū-būn-tê， mā ē-sái khì bāng-tsām pí-kàu khuànn-māi。
- POJ: chit ê choán-ōaⁿ khì kah tâi-gí khí-pôaⁿ iōng sio-kâng ê he̍k-sim， só͘-í choán-ōaⁿ ê kiat-kó kah khí-pôaⁿ phah-chhut lâi ê láng kāng-khoán。 nā phah-jī ê sî-sūn tùi siaⁿ-tiāu ah-sī peng-hoat ū-būn-tê， mā ē-sái khì bāng-chām pí-kàu khòaⁿ-māi。
- English: This converter uses the same core as Taigi Keyboard, so the conversion results match what the keyboard produces. If you have a question about a tone or spelling while typing, you can compare on the website.
- 日本語: このコンバーターは台語キーボードと同じコアを使っているため、変換結果はキーボードで打ったものと同じです。入力中に声調や綴りに疑問があれば、サイトで照らし合わせることもできます。
- flags: 器=>khì:1char; 佮=>kah:1char; 台語=>tâi-gí:AMB; 用=>iōng:1char; 相仝=>sio-kâng:AMB; 核心=>hi̍k-sim:AMB; 佮=>kah:1char; 來=>lâi:1char; 攏=>láng:1char; 若=>nā:1char; 拍字=>phah-jī:AMB; 時陣=>sî-sūn:AMB; 對=>tuì:1char; 抑是=>ah-sī:AMB; 嘛=>mā:1char; 會使=>ē-sái:AMB; 去=>khì:1char

### `features[taigiConverter].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 若發現轉換結果有毋著 ê 所在，請共我講，我會趕緊修理。
- Tâi-lô: nā huat-hiān tsuán-uānn kiat-kó ū m̄-tio̍h ê sóo-tsāi， tshiánn kā-guá kóng， guá huē kuánn-kín siu-lí。
- POJ: nā hoat-hiān choán-ōaⁿ kiat-kó ū m̄-tio̍h ê só͘-chāi， chhiáⁿ kā-góa kóng， góa hōe kóaⁿ-kín siu-lí。
- English: If you find anything wrong with the conversion results, please tell me and I'll fix it right away.
- 日本語: 変換結果に誤りを見つけたら、私に教えてください。すぐに修正します。
- flags: 若=>nā:1char; 有=>ū:1char; 請=>tshiánn:1char; 講=>kóng:1char; 我=>guá:1char; 會=>huē:1char


## `installIssue`

### `faqs[installIssue].title` — Tâi-lô conf: LOW

- 漢字: 齒盤裝好了後無出現
- Tâi-lô: khí-puânn tsong hâu-lio̍h āu bô tshut-hiān
- POJ: khí-pôaⁿ chong hâu-lio̍h āu bô chhut-hiān
- English: The keyboard doesn't appear after installing
- 日本語: インストール後にキーボードが表示されない
- flags: 裝=>tsong:1char; 後=>āu:1char; 無=>bô:1char

### `faqs[installIssue].paragraphs[0].text` — Tâi-lô conf: LOW

- 漢字: 1. 先檢查齒盤敢有照「啟用方法」設定好。
- Tâi-lô: 1. sian kiám-tsa khí-puânn kám ū tsiàu「khé-iōng hong-huat」 siat-tīng hó。
- POJ: 1. sian kiám-cha khí-pôaⁿ kám ū chiàu「khé-iōng hong-hoat」 siat-tēng hó。
- English: 1. First check whether the keyboard has been set up according to the “How to enable” guide.
- 日本語: 1. まず、キーボードが「有効化の方法」に従って設定されているか確認してください。
- flags: 先=>sian:1char; 照=>tsiàu:1char; 好=>hó:1char

### `faqs[installIssue].paragraphs[0].attachment.text` — Tâi-lô conf: LOW

- 漢字: 揤遮去看「啟用方法」
- Tâi-lô: tshi̍h jia khì khuànn「khé-iōng hong-huat」
- POJ: chhi̍h jia khì khòaⁿ「khé-iōng hong-hoat」
- English: Tap here to see “How to enable”
- 日本語: ここをタップして「有効化の方法」を見る
- flags: 揤=>tshi̍h:1char; 遮=>jia:1char; 去=>khì:1char; 看=>khuànn:1char

### `faqs[installIssue].paragraphs[1].text` — Tâi-lô conf: LOW

- 漢字: 2. 重開你當咧使用 ê App，予 App 重新掠著齒盤。
- Tâi-lô: 2. tîng-khui lí tng-teh sú-iōng ê App， hōo App tiông-sin lia̍h--tio̍h khí-puânn。
- POJ: 2. têng-khui lí tng-teh sú-iōng ê App， hō͘ App tiông-sin lia̍h--tio̍h khí-pôaⁿ。
- English: 2. Restart the app you are using so that it re-detects the keyboard.
- 日本語: 2. 使用中のアプリを再起動して、アプリにキーボードを再認識させてください。
- flags: 你=>lí:1char; 當咧=>tng-teh:AMB; 予=>hōo:1char

### `faqs[installIssue].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 3. 佇會當拍字 ê 所在，揤牢地球圖示就會使切去台語齒盤。
- Tâi-lô: 3. tī ē-tàng phah-jī ê sóo-tsāi， tshi̍h tiâu tuē-kiû-tôo sī tsiu-ē-sái tshiat khì tâi-gí khí-puânn。
- POJ: 3. tī ē-tàng phah-jī ê só͘-chāi， chhi̍h tiâu tōe-kiû-tô͘ sī chiu-ē-sái chhiat khì tâi-gí khí-pôaⁿ。
- English: 3. Where you can type, press and hold the globe icon to switch to Taigi Keyboard.
- 日本語: 3. 入力できる場所で、地球儀アイコンを長押しすると台語キーボードに切り替えられます。
- flags: 佇=>tī:1char; 會當=>ē-tàng:AMB; 拍字=>phah-jī:AMB; 揤=>tshi̍h:1char; 牢=>tiâu:1char; 示=>sī:1char; 切=>tshiat:1char; 去=>khì:1char; 台語=>tâi-gí:AMB


## `feedback`

### `faqs[feedback].title` — Tâi-lô conf: MED

- 漢字: 回報問題無消息
- Tâi-lô: huê-pò būn-tê bô-siau sit
- POJ: hôe-pò būn-tê bô-siau sit
- English: No response after reporting an issue
- 日本語: 問題を報告しても返事がない
- flags: 回報=>huê-pò:AMB; 問題=>būn-tê:AMB; 息=>sit:1char

### `faqs[feedback].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 可能無小心落勾去，麻煩 koh 回報 1 遍，抑是直接聯絡我。
- Tâi-lô: khó-lîng bô-sió-sim làu-kau khì， mâ-huân koh huê-pò 1 piàn， ah-sī ti̍t-tsiap liân-lo̍k guá。
- POJ: khó-lêng bô-sió-sim làu-kau khì， mâ-hoân koh hôe-pò 1 piàn， ah-sī ti̍t-chiap liân-lo̍k góa。
- English: It may have slipped through by accident; please report it once more, or contact me directly.
- 日本語: うっかり見落とされた可能性があります。お手数ですがもう一度報告するか、直接私にご連絡ください。
- flags: 可能=>khó-lîng:AMB; 去=>khì:1char; 回報=>huê-pò:AMB; 遍=>piàn:1char; 抑是=>ah-sī:AMB; 我=>guá:1char

### `faqs[feedback].paragraphs[0].attachment.text` — Tâi-lô conf: LOW

- 漢字: 揤遮看「關於」
- Tâi-lô: tshi̍h jia khuànn「kuan-î」
- POJ: chhi̍h jia khòaⁿ「koan-î」
- English: Tap here to see “About”
- 日本語: ここをタップして「概要」を見る
- flags: 揤=>tshi̍h:1char; 遮=>jia:1char; 看=>khuànn:1char


## `samsungSwitch`

### `faqs[samsungSwitch].title` — Tâi-lô conf: MED

- 漢字: 三星手機仔齒盤切換
- Tâi-lô: sann-tshenn tshiú-ki-á khí-puânn tshiat-uānn
- POJ: saⁿ-chheⁿ chhiú-ki-á khí-pôaⁿ chhiat-ōaⁿ
- English: Switching keyboards on Samsung phones
- 日本語: Samsung端末でのキーボード切り替え
- flags: 三星=>sann-tshenn:AMB

### `faqs[samsungSwitch].paragraphs[0].text` — Tâi-lô conf: MED

- 漢字: 因為三星手機無「預設齒盤」、「副齒盤」ê 功能，下跤是切換齒盤較利便 ê 方法。
- Tâi-lô: in-uī sann-tshenn tshiú-ki bô「ī-siat khí-puânn」、「hù khí-puânn」ê kong-lîng， ē-kha sī tshiat-uānn khí-puânn khah lī-piān ê hong-huat。
- POJ: in-ūi saⁿ-chheⁿ chhiú-ki bô「ī-siat khí-pôaⁿ」、「hù khí-pôaⁿ」ê kong-lêng， ē-kha sī chhiat-ōaⁿ khí-pôaⁿ khah lī-piān ê hong-hoat。
- English: Because Samsung phones don't have “default keyboard” / “secondary keyboard” features, here is a more convenient way to switch keyboards.
- 日本語: Samsung端末には「既定のキーボード」「サブキーボード」機能がないため、以下はキーボードを切り替えるより便利な方法です。
- flags: 三星=>sann-tshenn:AMB; 無=>bô:1char; 預設=>ī-siat:AMB; 副=>hù:1char; 是=>sī:1char; 較=>khah:1char

### `faqs[samsungSwitch].paragraphs[1].text` — Tâi-lô conf: MED

- 漢字: 1. 揣著三星齒盤倒爿下跤 ê mài-kù 揤鈕，伊預設是語音輸入。
- Tâi-lô: 1. tshuē-tio̍h sann-tshenn khí-puânn tò-pîn ē-kha ê mài-kù tshi̍h-liú， i ī-siat sī gí-im su-ji̍p。
- POJ: 1. chhōe-tio̍h saⁿ-chheⁿ khí-pôaⁿ tò-pîn ē-kha ê mài-kù chhi̍h-liú， i ī-siat sī gí-im su-ji̍p。
- English: 1. Find the mic button at the bottom-left of the Samsung keyboard; by default it is voice input.
- 日本語: 1. Samsungキーボードの左下にあるマイクボタンを探します。既定では音声入力になっています。
- flags: 揣著=>tshuē-tio̍h:AMB; 三星=>sann-tshenn:AMB; 倒爿=>tò-pîn:AMB; 伊=>i:1char; 預設=>ī-siat:AMB; 是=>sī:1char; 語音=>gí-im:AMB; 輸入=>su-ji̍p:AMB

### `faqs[samsungSwitch].paragraphs[2].text` — Tâi-lô conf: LOW

- 漢字: 2. 揤牢 mài-kù 揤鈕，共伊改做「輸入法」。
- Tâi-lô: 2. tshi̍h tiâu mài-kù tshi̍h-liú， kā-i kué-tsuè「su-ji̍p-hoat」。
- POJ: 2. chhi̍h tiâu mài-kù chhi̍h-liú， kā-i kóe-chòe「su-ji̍p-hoat」。
- English: 2. Press and hold the mic button and change it to “input method.”
- 日本語: 2. マイクボタンを長押しして、「入力方式」に変更します。
- flags: 揤=>tshi̍h:1char; 牢=>tiâu:1char

### `faqs[samsungSwitch].paragraphs[3].text` — Tâi-lô conf: LOW

- 漢字: 3. 揤鈕會變做齒盤圖示，閣揤一下著會當切換齒盤矣。
- Tâi-lô: 3. tshi̍h-liú huē piàn-tsò khí-puânn tôo-sī， koh tshi̍h tsi̍t-ē tio̍h ē-tàng tshiat-uānn khí-puânn ah。
- POJ: 3. chhi̍h-liú hōe piàn-chò khí-pôaⁿ tô͘-sī， koh chhi̍h chi̍t-ē tio̍h ē-tàng chhiat-ōaⁿ khí-pôaⁿ ah。
- English: 3. The button becomes a keyboard icon; press it again and you can switch keyboards.
- 日本語: 3. ボタンがキーボードのアイコンに変わります。もう一度押すとキーボードを切り替えられます。
- flags: 會=>huē:1char; 變做=>piàn-tsò:AMB; 閣=>koh:1char; 揤=>tshi̍h:1char; 著=>tio̍h:1char; 會當=>ē-tàng:AMB; 矣=>ah:1char

