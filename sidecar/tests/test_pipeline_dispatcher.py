"""Dispatcher hot-swap unit tests. Uses stub backends so no torch / diffusers
required for these tests to pass - real backend smoke is M7.5-DM scope."""
from __future__ import annotations

import sys
from pathlib import Path
from typing import ClassVar

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from pipeline import PipelineDispatcher  # noqa: E402


class StubBackend:
    vram_estimate_bytes: ClassVar[int] = 1

    def __init__(self, name: str, modality: str = "image") -> None:
        self.name = name
        self.modality = modality
        self.load_calls = 0
        self.unload_calls = 0

    def load(self) -> None:
        self.load_calls += 1

    def unload(self) -> None:
        self.unload_calls += 1

    def generate(self, params: PromptParams) -> bytes:
        return f"{self.name}|{params.text}".encode()


def test_dispatcher_starts_with_no_loaded_backend():
    d = PipelineDispatcher.test_instance()
    assert d.loaded is None


def test_dispatcher_loads_backend_on_first_generate():
    d = PipelineDispatcher.test_instance()
    a = StubBackend("fast")
    d.backends["fast"] = a
    out = d.generate("fast", PromptParams(text="hi"))
    assert out == b"fast|hi"
    assert d.loaded == "fast"
    assert a.load_calls == 1
    assert a.unload_calls == 0


def test_dispatcher_unloads_previous_on_swap():
    d = PipelineDispatcher.test_instance()
    a = StubBackend("fast")
    b = StubBackend("balanced")
    d.backends["fast"] = a
    d.backends["balanced"] = b
    d.generate("fast", PromptParams(text="x"))
    d.generate("balanced", PromptParams(text="x"))
    assert d.loaded == "balanced"
    assert a.unload_calls == 1
    assert b.load_calls == 1


def test_dispatcher_no_load_on_same_backend_repeat():
    d = PipelineDispatcher.test_instance()
    a = StubBackend("fast")
    d.backends["fast"] = a
    d.generate("fast", PromptParams(text="x"))
    d.generate("fast", PromptParams(text="y"))
    assert a.load_calls == 1  # not reloaded


def test_dispatcher_unknown_backend_raises():
    d = PipelineDispatcher.test_instance()
    with pytest.raises(KeyError, match="unknown backend"):
        d.generate("not-a-backend", PromptParams(text="x"))


def test_production_dispatcher_registers_4_image_plus_ltx_video():
    d = PipelineDispatcher.production()
    assert set(d.backends.keys()) == {
        "fast",
        "balanced",
        "quality",
        "quality-oss",
        "ltx-video",
    }
    image_backends = [b for b in d.backends.values() if b.modality == "image"]
    video_backends = [b for b in d.backends.values() if b.modality == "video"]
    assert len(image_backends) == 4
    assert len(video_backends) == 1


