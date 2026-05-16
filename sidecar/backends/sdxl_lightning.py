"""Balanced preset (SDXL base + Lightning 4-step LoRA + optional style LoRA).

License: Apache 2.0. Default for M7-DM image generation.

NOTE: Real diffusers wiring deferred to M7.5-DM (requires HF download +
GPU smoke test). This module ships the protocol-shape stub so the dispatcher
can register the backend; load() raises until the real loader lands.
"""
from __future__ import annotations

import io
from typing import ClassVar, Literal

from backends.protocol import GenerationBackend, PromptParams


class SdxlLightningBackend:
    name: ClassVar[str] = "balanced"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = 5 * 1024**3

    def __init__(self) -> None:
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        # Real implementation will load StableDiffusionXLPipeline +
        # sdxl_lightning_4step_lora.safetensors + EulerDiscreteScheduler with
        # timestep_spacing="trailing", per spec §3 D1. Deferred to M7.5-DM.
        raise NotImplementedError(
            "SdxlLightningBackend.load() is wired in M7.5-DM; "
            "use Fast preset (sdxl-turbo) for now."
        )

    def unload(self) -> None:
        self._pipe = None

    def generate(self, params: PromptParams) -> bytes:  # pragma: no cover
        assert self._pipe is not None, "call load() first"
        del params
        return io.BytesIO().getvalue()
