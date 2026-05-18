"""Quality-OSS preset (Z-Image-Turbo 6B, Apache 2.0).

License: Apache 2.0 - the commercial-safe quality choice.
Target on 3080 10GB: 3-5 s/image at 1024x1024 8 steps.

STATUS (2026-05-18): DEFERRED. Tongyi-MAI/Z-Image-Turbo's model_index.json
declares `_class_name: ZImagePipeline` and `_diffusers_version: 0.36.0.dev0`.
ZImagePipeline + ZImageTransformer2DModel only exist in unreleased diffusers
0.36 dev. The text encoder is Qwen3Model which needs transformers 4.51+.
We are pinned at diffusers 0.33.1 + transformers 4.46.0.

NO-GO criterion from spec section 6 was met: upstream pipeline incompatible
with the M7.5-DM dep pins. Per anti-decision list, NOT falling back to FP8
or Comfy-Org mirrors. Revisit when diffusers 0.36 lands stable (~mid-2026 ETA
based on dev branch cadence) and transformers >= 4.51 is in the pin window."""
from __future__ import annotations

from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends.protocol import GenerationBackend, PromptParams


class ZImageTurboBackend:
    name: ClassVar[str] = "quality-oss"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = int(5.5 * 1024**3)

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        self._weights_dir = weights_dir or Path.home() / ".cache" / "dm-ai-gpu-weights"
        self._pipe = None

    def load(self) -> None:
        raise NotImplementedError(
            "Quality-OSS (Z-Image-Turbo) deferred: Tongyi-MAI/Z-Image-Turbo "
            "needs diffusers 0.36+ (ZImagePipeline) and transformers 4.51+ "
            "(Qwen3Model). DM-AI is pinned at diffusers 0.33.1 / transformers "
            "4.46.0 for the rest of the M7.5-DM image stack. Revisit when "
            "diffusers 0.36 ships stable. Use 'balanced' (SDXL-Lightning) or "
            "'quality' (Nunchaku FLUX-dev INT4) preset until then."
        )

    def unload(self) -> None:
        self._pipe = None

    def generate(self, params: PromptParams) -> bytes:  # pragma: no cover
        raise NotImplementedError("see load()")
