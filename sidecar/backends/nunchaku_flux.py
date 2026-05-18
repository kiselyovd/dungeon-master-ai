"""Quality preset (Nunchaku FLUX.1-dev INT4 SVDQuant + Turbo-Alpha 8-step LoRA).

License: FLUX-dev non-commercial. Portfolio/demo OK; not for monetised GA.
Anti-decisions enforced: no FP8 anywhere, no torch.compile over Nunchaku,
SageAttention v1 only (not enabled here; explicit opt-in via env var).
On 3080 10GB: 3-4 s/image at 1024x1024 8 steps."""
from __future__ import annotations

import io
from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends._capabilities import assert_vram_free
from backends.protocol import GenerationBackend, PromptParams


class NunchakuFluxBackend:
    name: ClassVar[str] = "quality"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = 8 * 1024**3

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        self._weights_dir = weights_dir or Path.home() / ".cache" / "dm-ai-gpu-weights"
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        assert_vram_free(8 * 1024**3)

        import torch  # noqa: PLC0415
        from diffusers import FluxPipeline  # noqa: PLC0415
        from nunchaku import NunchakuFluxTransformer2dModel  # noqa: PLC0415

        base_dir = self._weights_dir / "quality" / "flux-dev"
        nunchaku_path = self._weights_dir / "quality" / "nunchaku-flux-int4.safetensors"
        lora_path = self._weights_dir / "quality" / "flux-turbo-alpha.safetensors"

        transformer = NunchakuFluxTransformer2dModel.from_pretrained(str(nunchaku_path))
        pipe = FluxPipeline.from_pretrained(
            str(base_dir),
            transformer=transformer,
            torch_dtype=torch.bfloat16,
        ).to("cuda" if torch.cuda.is_available() else "cpu")

        pipe.load_lora_weights(str(lora_path))
        pipe.fuse_lora()

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
            num_inference_steps=params.steps or 8,
            guidance_scale=params.guidance if params.guidance is not None else 3.5,
            height=params.resolution[1],
            width=params.resolution[0],
            generator=generator,
        ).images[0]

        buf = io.BytesIO()
        image.save(buf, format="PNG")
        return buf.getvalue()
