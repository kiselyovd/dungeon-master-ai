"""Balanced preset (SDXL 1.0 base + SDXL-Lightning 4-step LoRA).

License: Apache 2.0. Default for image generation per spec D1.
On 3080 10GB: ~2.5 s/image at 1024x1024 4 steps."""
from __future__ import annotations

import io
from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends._capabilities import assert_vram_free
from backends.protocol import GenerationBackend, PromptParams


class SdxlLightningBackend:
    name: ClassVar[str] = "balanced"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = 8 * 1024**3

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        self._weights_dir = weights_dir or Path.home() / ".cache" / "dm-ai-gpu-weights"
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        assert_vram_free(7 * 1024**3)

        import torch  # noqa: PLC0415
        from diffusers import EulerDiscreteScheduler, StableDiffusionXLPipeline  # noqa: PLC0415

        base_dir = self._weights_dir / "balanced" / "sdxl-base"
        lora_path = self._weights_dir / "balanced" / "lightning-lora.safetensors"

        pipe = StableDiffusionXLPipeline.from_pretrained(
            str(base_dir),
            torch_dtype=torch.float16,
            variant="fp16",
            use_safetensors=True,
        ).to("cuda" if torch.cuda.is_available() else "cpu")

        pipe.load_lora_weights(str(lora_path))
        pipe.fuse_lora()

        pipe.scheduler = EulerDiscreteScheduler.from_config(
            pipe.scheduler.config, timestep_spacing="trailing"
        )
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
        if params.style_lora:
            self._pipe.load_lora_weights(params.style_lora, adapter_name="style")
            self._pipe.set_adapters(["style"], adapter_weights=[1.0])

        image = self._pipe(
            prompt=params.text,
            negative_prompt=params.negative,
            num_inference_steps=params.steps or 4,
            guidance_scale=0.0,
            height=params.resolution[1],
            width=params.resolution[0],
            generator=generator,
        ).images[0]

        buf = io.BytesIO()
        image.save(buf, format="PNG")
        return buf.getvalue()
