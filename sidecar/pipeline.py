"""Dispatcher pattern: one PipelineDispatcher holds N image/video backends and
hot-swaps between them as the HTTP layer requests different presets.

Only one backend is loaded at a time (10 GB VRAM cap on RTX 3080); hot-swap
between presets carries a 5-15s load/unload penalty which is acceptable since
users don't switch presets per request.
"""
from __future__ import annotations

from pathlib import Path
from typing import Optional

from backends import GenerationBackend, PromptParams
from backends.ltx_video import LtxVideoBackend
from backends.nunchaku_flux import NunchakuFluxBackend
from backends.sdxl_lightning import SdxlLightningBackend
from backends.sdxl_turbo import SdxlTurboBackend
from backends.z_image_turbo import ZImageTurboBackend


class PipelineDispatcher:
    def __init__(self, backends: dict[str, GenerationBackend]) -> None:
        self.backends = backends
        self.loaded: Optional[str] = None

    @classmethod
    def production(cls, weights_dir: Optional[Path] = None) -> "PipelineDispatcher":
        return cls({
            "fast": SdxlTurboBackend(weights_dir),
            "balanced": SdxlLightningBackend(weights_dir),
            "quality": NunchakuFluxBackend(weights_dir),
            "quality-oss": ZImageTurboBackend(weights_dir),
            "ltx-video": LtxVideoBackend(weights_dir),
        })

    @classmethod
    def test_instance(cls) -> "PipelineDispatcher":
        """Empty dispatcher; tests inject stub backends via `d.backends[id] = ...`."""
        return cls({})

    def generate(self, backend_id: str, params: PromptParams) -> bytes:
        if backend_id not in self.backends:
            raise KeyError(f"unknown backend: {backend_id}")
        if self.loaded != backend_id:
            if self.loaded is not None:
                self.backends[self.loaded].unload()
            self.backends[backend_id].load()
            self.loaded = backend_id
        return self.backends[backend_id].generate(params)
