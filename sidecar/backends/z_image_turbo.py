"""Quality-OSS preset (Z-Image-Turbo 6B, Apache 2.0).

License: Apache 2.0 - the commercial-safe quality choice.
On 3080 10GB: target 3-5 s/image at 1024x1024 8 steps.

Requires diffusers 0.39+ (ZImagePipeline) and transformers 4.51+ (Qwen3Model).
Unblocked 2026-05-18 by bumping diffusers to git main + transformers to 4.57."""
from __future__ import annotations

import io
from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends._capabilities import assert_vram_free
from backends.protocol import GenerationBackend, PromptParams


class ZImageTurboBackend:
    name: ClassVar[str] = "quality-oss"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = int(6 * 1024**3)

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        self._weights_dir = weights_dir or Path.home() / ".cache" / "dm-ai-gpu-weights"
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        assert_vram_free(6 * 1024**3)

        import torch  # noqa: PLC0415
        from diffusers import ZImagePipeline  # noqa: PLC0415

        repo_dir = self._weights_dir / "quality-oss" / "z-image-turbo"
        # BF16 not FP16: 6B params @ FP16 produced NaN on RTX 3080 (verified
        # 2026-05-18). BF16 has the same memory footprint with stable dynamic
        # range. Until mit-han-lab ships an SVDQ-INT4 release this is the only
        # working dtype on 10 GB Ampere.
        pipe = ZImagePipeline.from_pretrained(
            str(repo_dir),
            torch_dtype=torch.bfloat16,
        ).to("cuda" if torch.cuda.is_available() else "cpu")
        self._pipe = pipe

    def unload(self) -> None:
        if self._pipe is not None:
            self._pipe = None
            try:
                import torch  # noqa: PLC0415
                if torch.cuda.is_available():
                    torch.cuda.empty_cache()
            except ImportError:
                pass

    def generate(self, params: PromptParams) -> bytes:
        assert self._pipe is not None, "call load() first"

        import torch  # noqa: PLC0415

        generator = torch.Generator(device=self._pipe.device).manual_seed(params.seed or 0)
        image = self._pipe(
            prompt=params.text,
            negative_prompt=params.negative,
            num_inference_steps=params.steps or 8,
            guidance_scale=params.guidance if params.guidance is not None else 0.0,
            height=params.resolution[1],
            width=params.resolution[0],
            generator=generator,
        ).images[0]

        buf = io.BytesIO()
        image.save(buf, format="PNG")
        return buf.getvalue()
