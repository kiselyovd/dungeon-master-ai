"""Fast preset (current M4 pipeline, ported into the dispatcher shape)."""
from __future__ import annotations

import io
from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends.protocol import GenerationBackend, PromptParams


class SdxlTurboBackend:
    name: ClassVar[str] = "fast"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = 5 * 1024**3

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        # The DownloadManager stores diffusers folders at <base>/<hf_repo> and
        # the sidecar is launched with that <base> as --weights-dir. Resolve the
        # model's own subdir so from_pretrained targets the real weights instead
        # of the shared base root (which also holds chat GGUFs).
        if weights_dir is not None:
            self._weights_dir = weights_dir / "stabilityai" / "sdxl-turbo"
        else:
            self._weights_dir = Path.cwd() / "models" / "sdxl-turbo"
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        # Real model loader; gated behind an import so non-CUDA test environments
        # can construct the backend without pulling torch+diffusers in.
        import torch  # noqa: PLC0415
        from diffusers import AutoPipelineForText2Image  # noqa: PLC0415

        self._pipe = AutoPipelineForText2Image.from_pretrained(
            str(self._weights_dir),
            torch_dtype=torch.float16,
            variant="fp16",
        ).to("cuda" if torch.cuda.is_available() else "cpu")

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
        image = self._pipe(
            prompt=params.text,
            num_inference_steps=params.steps or 4,
            guidance_scale=0.0,
            height=params.resolution[1],
            width=params.resolution[0],
        ).images[0]
        buf = io.BytesIO()
        image.save(buf, format="PNG")
        return buf.getvalue()
