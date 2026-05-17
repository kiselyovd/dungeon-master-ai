"""Dispatcher pattern: one PipelineDispatcher holds N image/video backends and
hot-swaps between them as the HTTP layer requests different presets.

Only one backend is loaded at a time (10 GB VRAM cap on RTX 3080); hot-swap
between presets carries a 5-15s load/unload penalty which is acceptable since
users don't switch presets per request.

The legacy `SdxlPipeline` shim is kept as a thin alias so the existing
app.create_app() keeps working until the route layer migrates to dispatch
(Phase E.7 Rust side).
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
            "balanced": SdxlLightningBackend(),
            "quality": NunchakuFluxBackend(),
            "quality-oss": ZImageTurboBackend(),
            "ltx-video": LtxVideoBackend(),
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


# Legacy shim — kept so existing app.create_app() keeps working until the
# Phase E.7 route migration. New code should use PipelineDispatcher directly.
class SdxlPipeline:
    def __init__(self, weights_dir: Path) -> None:
        self._backend = SdxlTurboBackend(weights_dir)

    def generate(self, prompt: str, seed: int = 0, steps: int = 4):
        # Returns PIL.Image (legacy contract — app.py base64-encodes it itself).
        self._backend.load()
        assert self._backend._pipe is not None
        import torch  # noqa: PLC0415

        generator = torch.Generator(device=self._backend._pipe.device).manual_seed(seed)
        image = self._backend._pipe(
            prompt=prompt,
            num_inference_steps=steps,
            guidance_scale=0.0,
            generator=generator,
        ).images[0]
        return image

    def unload(self) -> None:
        self._backend.unload()
