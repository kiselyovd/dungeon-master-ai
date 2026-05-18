"""LTX-Video backend (text-to-video, opt-in via Settings).

License: LTX open. On 3080 10GB at 704x480, 97 frames, 8 steps: ~22 s/clip.
Uses diffusers LTXVideoTransformer3DModel.from_single_file with the distilled
2B 0.9.6 checkpoint, plugged into LTXPipeline against the same Lightricks/LTX-Video
diffusers folder (which provides VAE, T5 encoder, scheduler, tokenizer).
ComfyUI GGUF Q8 path deferred to M9-DM+."""
from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Callable, ClassVar, Literal, Optional

from backends._capabilities import assert_vram_free
from backends.protocol import GenerationBackend, PromptParams


class LtxVideoBackend:
    name: ClassVar[str] = "ltx-video"
    modality: ClassVar[Literal["video"]] = "video"
    vram_estimate_bytes: ClassVar[int] = 7 * 1024**3

    def __init__(self, weights_dir: Optional[Path] = None) -> None:
        self._weights_dir = weights_dir or Path.home() / ".cache" / "dm-ai-gpu-weights"
        self._pipe = None
        self._progress_callback: Optional[Callable[[float], None]] = None

    def set_progress_callback(self, cb: Callable[[float], None]) -> None:
        self._progress_callback = cb

    def load(self) -> None:
        if self._pipe is not None:
            return
        # 7 GB preflight: T5 offloads after text encode; diffusion peak is
        # distilled 2B transformer (~3-4 GB BF16) + VAE slicing/tiling (~1-2 GB).
        assert_vram_free(7 * 1024**3)

        import torch  # noqa: PLC0415
        from diffusers import LTXPipeline, LTXVideoTransformer3DModel  # noqa: PLC0415

        repo_dir = self._weights_dir / "ltx-video" / "ltx-video"
        distilled_path = self._weights_dir / "ltx-video" / "ltxv-2b-0.9.6-distilled-04-25.safetensors"

        transformer = LTXVideoTransformer3DModel.from_single_file(
            str(distilled_path), torch_dtype=torch.bfloat16
        )
        pipe = LTXPipeline.from_pretrained(
            str(repo_dir),
            transformer=transformer,
            torch_dtype=torch.bfloat16,
        )
        if torch.cuda.is_available():
            # model_cpu_offload swaps T5 out before diffusion to keep peak VRAM
            # comfortably below 10 GB on RTX 3080 (matches Z-Image backend strategy).
            pipe.enable_model_cpu_offload()
        else:
            pipe = pipe.to("cpu")
        pipe.vae.enable_slicing()
        pipe.vae.enable_tiling()
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
        from diffusers.utils import export_to_video  # noqa: PLC0415

        steps = params.steps or 8
        frames = params.frame_count or 97
        cb = self._progress_callback

        def _on_step(_pipe, i, _t, kw):
            if cb is not None:
                cb((i + 1) / steps)
            return kw

        height = params.resolution[1] if params.resolution[1] >= 256 else 480
        width = params.resolution[0] if params.resolution[0] >= 256 else 704

        generator = torch.Generator(device=self._pipe.device).manual_seed(params.seed or 0)
        result = self._pipe(
            prompt=params.text,
            negative_prompt=params.negative or "worst quality, blurry, low detail",
            num_inference_steps=steps,
            num_frames=frames,
            height=height,
            width=width,
            generator=generator,
            callback_on_step_end=_on_step,
        )

        with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as tmp:
            tmp_path = Path(tmp.name)
        try:
            export_to_video(result.frames[0], str(tmp_path), fps=24)
            return tmp_path.read_bytes()
        finally:
            tmp_path.unlink(missing_ok=True)
