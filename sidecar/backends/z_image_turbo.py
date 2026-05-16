"""Quality-OSS preset (Z-Image-Turbo 6B SVDQ-INT4 + Qwen3-4B text encoder).

License: Apache 2.0. Shares Qwen3-4B text encoder with local chat (via
ModelManifest.requires graph).

NOTE: Real wiring deferred to M7.5-DM (upstream package + GPU smoke).
"""
from __future__ import annotations

import io
from typing import ClassVar, Literal

from backends.protocol import GenerationBackend, PromptParams


class ZImageTurboBackend:
    name: ClassVar[str] = "quality-oss"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = int(5.5 * 1024**3)

    def __init__(self) -> None:
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        raise NotImplementedError(
            "ZImageTurboBackend.load() is wired in M7.5-DM; "
            "select Balanced or Fast preset for now."
        )

    def unload(self) -> None:
        self._pipe = None

    def generate(self, params: PromptParams) -> bytes:  # pragma: no cover
        assert self._pipe is not None, "call load() first"
        del params
        return io.BytesIO().getvalue()
