"""`TaigiConverter.prefetch` fills the reply cache `_call` reads.

A prefetched reply must be exactly the reply the single call returns — error
replies included — so the per-row graceful / strict helpers behave the same
whether or not their stage prefetched.
"""

from __future__ import annotations

import io

import pytest

from common.taigi_bridge import BridgeDeadError, TaigiConverter

# (op, text, kwargs): ordinary conversions, an empty text, a per-item JS
# error (unknown source system) and a validation miss.
_REQUESTS = [
    ("convert", "hó-sè", {"source": "tl", "target": "poj"}),
    ("convert", "", {"source": "tl", "target": "poj"}),
    ("convert", "tsiah-pn̄g", {"source": "tl", "target": "zhuyin"}),
    ("convert", "hó", {"source": "bogus", "target": "poj"}),
    ("toToneNumber", "hó-sè", {}),
    ("toToneNumberAscii", "hó͘", {}),
    ("validate", "hó-sè", {}),
    ("validate", "xyz", {}),
]


@pytest.fixture
def converter():
    c = TaigiConverter()
    yield c
    c.close()


def _count_roundtrips(c: TaigiConverter, monkeypatch) -> list[dict]:
    sent: list[dict] = []
    real = c._roundtrip

    def spy(request: dict) -> dict:
        sent.append(request)
        return real(request)

    monkeypatch.setattr(c, "_roundtrip", spy)
    return sent


def test_prefetched_reply_equals_single_call_reply(converter):
    single = TaigiConverter()
    try:
        for op, text, kwargs in _REQUESTS:
            converter.prefetch(op, [text], **kwargs)
            expected = single._call(op, text=text, **kwargs)
            assert converter._call(op, text=text, **kwargs) == expected
    finally:
        single.close()


def test_error_reply_is_cached_per_item(converter):
    converter.prefetch("convert", ["hó", "sè"], source="bogus", target="poj")
    reply = converter._call("convert", text="hó", source="bogus", target="poj")
    assert reply["result"] is None
    assert "bogus" in reply["error"]


def test_call_after_prefetch_makes_no_roundtrip(converter, monkeypatch):
    converter.prefetch("toToneNumber", ["hó", "sè", "hó"])
    sent = _count_roundtrips(converter, monkeypatch)
    converter._call("toToneNumber", text="hó")
    converter._call("toToneNumber", text="sè")
    assert sent == []


def test_prefetch_sends_only_uncached_distinct_texts(converter, monkeypatch):
    converter._call("validate", text="hó")
    sent = _count_roundtrips(converter, monkeypatch)
    converter.prefetch("validate", ["hó", "sè", "sè", "tsa"])
    assert [r["texts"] for r in sent] == [["sè", "tsa"]]
    converter.prefetch("validate", ["hó", "sè"])
    assert len(sent) == 1


def test_prefetch_chunks_long_lists(converter, monkeypatch):
    monkeypatch.setattr("common.taigi_bridge._PREFETCH_CHUNK", 2)
    sent = _count_roundtrips(converter, monkeypatch)
    converter.prefetch("toToneNumber", ["a", "á", "à", "â", "ā"])
    assert [len(r["texts"]) for r in sent] == [2, 2, 1]
    assert converter._call("toToneNumber", text="â") == {"result": "a5"}


def test_dead_bridge_raises(converter, monkeypatch):
    converter._call("validate", text="hó")
    assert converter._process is not None
    # Subprocess stdout at EOF: what `readline` sees once node has exited.
    monkeypatch.setattr(converter._process, "stdout", io.StringIO(""))
    with pytest.raises(BridgeDeadError):
        converter.prefetch("validate", ["sè"])


def test_list_reply_of_wrong_length_raises(converter, monkeypatch):
    monkeypatch.setattr(converter, "_roundtrip", lambda request: {"results": []})
    with pytest.raises(BridgeDeadError):
        converter.prefetch("validate", ["sè"])
    assert converter._replies == {}
