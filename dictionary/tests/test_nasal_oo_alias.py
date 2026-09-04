"""Nasal-`oo` alias keys emitted by `create_fst`.

The nasal final /ɔ̃/ is written `onn` (TL) / `oⁿ` (POJ) throughout the
dictionary; some writers spell it `o͘ⁿ`, which reaches the engine as ASCII
`oonn`. `create_fst` indexes that spelling beside the canonical one so those
users find the same rows.

The load-bearing part is that the respelling runs PER SYLLABLE, on the
digit-separated `*_num` value. Inside one syllable the letters `onn` can only
be the nasal final; across a seam they are an `oo`-final meeting the next
syllable's `nng` (滷卵 `loo|nng`) or an `o`-final meeting `onn` (可惡
`kho|onn`), and respelling there would invent a key nobody types.

Mirrors `phonetics::nasal_oo_alias_spelling`, which does the same job for
`syllables.fst` where the caller already holds one syllable per token.
"""

from __future__ import annotations

from build.create_fst import _nasal_oo_alias_keys, _nasal_oo_alias_num


def test_respells_the_nasal_final_of_each_syllable():
    # 好 / 呼 / 否 hònn hoⁿ hónn — one syllable, one respelling.
    assert _nasal_oo_alias_num("honn3") == "hoonn3"
    # 乎 / 歟 honnh — stop coda kept.
    assert _nasal_oo_alias_num("honnh4") == "hoonnh4"
    # 好玄 hònn-hiân — respell the first syllable, copy the second.
    assert _nasal_oo_alias_num("honn3hian5") == "hoonn3hian5"
    # 呼呼 hōnn-hōnn — BOTH syllables respell.
    assert _nasal_oo_alias_num("honn7honn7") == "hoonn7hoonn7"
    # 可惡 khó-ònn — the nasal is the SECOND syllable.
    assert _nasal_oo_alias_num("kho2onn3") == "kho2oonn3"
    # 苦惡鳥 khóo-ònn-tsiáu — an `oo` syllable sits right before the nasal one.
    assert _nasal_oo_alias_num("khoo2onn3tsiau2") == "khoo2oonn3tsiau2"


def test_leaves_cross_syllable_seams_alone():
    # Every dictionary value whose letters contain `onn` WITHOUT holding a
    # nasal final: the `oo` closes one syllable and `nng` opens the next.
    # 滷卵 lóo-nn̄g / 芋卵 ōo-nn̄g / 菜脯卵 tshài-póo-nn̄g / 飛烏卵 pue-oo-nn̄g.
    for num_value in ("loo2nng7", "oo7nng7", "tshai3poo2nng7", "pue1oo1nng7"):
        assert _nasal_oo_alias_num(num_value) == "", num_value
        assert _nasal_oo_alias_keys(num_value) == (), num_value


def test_emits_both_the_numeric_and_toneless_key_bodies():
    # The toneless body is derived from the ALIAS num (digits stripped), never
    # from the stored `*_notone` column — that column is already fused, so its
    # syllable boundaries are gone and 滷卵's `loonng` would respell across the
    # seam.
    assert _nasal_oo_alias_keys("honn3") == ("hoonn3", "hoonn")
    assert _nasal_oo_alias_keys("honn3hian5") == ("hoonn3hian5", "hoonnhian")
    assert _nasal_oo_alias_keys("kho2onn3") == ("kho2oonn3", "khooonn")


def test_no_nasal_final_means_no_alias_keys():
    for num_value in ("tai5gi2", "hoo7", "nng7", "", "tsiah8"):
        assert _nasal_oo_alias_keys(num_value) == (), num_value


def test_skips_a_value_that_does_not_partition_into_syllables():
    # Defensive: a dirty row (uppercase, punctuation, CJK) is skipped rather
    # than mangled — the regex partition check must fail closed.
    assert _nasal_oo_alias_num("HONN3") == ""
    assert _nasal_oo_alias_num("honn3-hian5") == ""
    assert _nasal_oo_alias_num("原住民族委員會1") == ""
