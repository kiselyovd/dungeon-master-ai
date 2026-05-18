"""Quality-OSS preset (Z-Image-Turbo 6B SVDQuant INT4, Apache 2.0).

License: Apache 2.0 - the commercial-safe quality choice.
On 3080 10GB: target 3-5 s/image at 1024x1024 8 steps via SVDQuant INT4 r128.

Requires diffusers 0.39+ (ZImagePipeline), transformers 4.51+ (Qwen3Model), and
nunchaku 1.1.0+ (NunchakuZImageTransformer2DModel).
Unblocked 2026-05-18 evening via nunchaku-tech/nunchaku-z-image-turbo SVDQ release
and nunchaku 1.2.1 wheel (torch 2.8 + cu128)."""
from __future__ import annotations

import io
from pathlib import Path
from typing import ClassVar, Literal, Optional

from backends._capabilities import assert_vram_free
from backends.protocol import GenerationBackend, PromptParams


def _patch_nunchaku_zimage_forward(transformer_cls) -> None:
    """Workaround for nunchaku 1.2.1 vs diffusers main signature drift.

    nunchaku's NunchakuZImageTransformer2DModel.forward calls
    super().forward(x, t, cap_feats, patch_size, f_patch_size, return_dict)
    positionally, but diffusers main inserted controlnet_block_samples /
    siglip_feats / image_noise_mask BETWEEN return_dict and patch_size, so
    patch_size=2 lands in the controlnet_block_samples slot and trips a
    'argument of type int is not iterable' deep in the unified-sequence loop.
    Fix until nunchaku ships a release that uses keyword args."""
    if getattr(transformer_cls, "_dm_ai_forward_patched", False):
        return
    from nunchaku.models.transformers.transformer_zimage import (  # noqa: PLC0415
        NunchakuZImageRopeHook,
    )
    base_forward = transformer_cls.__mro__[1].forward  # diffusers ZImageTransformer2DModel.forward

    def _patched(self, x, t, cap_feats, patch_size=2, f_patch_size=1, return_dict=True):
        rope_hook = NunchakuZImageRopeHook()
        self.register_rope_hook(rope_hook)
        try:
            return base_forward(
                self,
                x,
                t,
                cap_feats,
                return_dict=return_dict,
                patch_size=patch_size,
                f_patch_size=f_patch_size,
            )
        finally:
            self.unregister_rope_hook()
            del rope_hook

    transformer_cls.forward = _patched
    transformer_cls._dm_ai_forward_patched = True


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
        from nunchaku import NunchakuZImageTransformer2DModel  # noqa: PLC0415

        _patch_nunchaku_zimage_forward(NunchakuZImageTransformer2DModel)

        base_dir = self._weights_dir / "quality-oss" / "z-image-turbo"
        svdq_path = self._weights_dir / "quality-oss" / "svdq-int4_r128-z-image-turbo.safetensors"

        # INT4 rank 128: balanced quality/speed default for SVDQuant on Ampere.
        # FP4 variants forbidden on RTX 3080 (no native FP4 hardware).
        transformer = NunchakuZImageTransformer2DModel.from_pretrained(
            str(svdq_path), torch_dtype=torch.bfloat16
        )
        pipe = ZImagePipeline.from_pretrained(
            str(base_dir),
            transformer=transformer,
            torch_dtype=torch.bfloat16,
            low_cpu_mem_usage=False,
        )
        if torch.cuda.is_available():
            # Z-Image-Turbo full stack (SVDQ transformer 4 GB + Qwen3-4B encoder 4 GB +
            # VAE 0.5 GB) lands at ~11.5 GB resident which is over 10 GB on 3080,
            # causing unified-memory paging and ~50 s/step. model_cpu_offload keeps
            # only the active component on GPU (peak ~6-7 GB), restoring fast path.
            pipe.enable_model_cpu_offload()
        else:
            pipe = pipe.to("cpu")
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
        # guidance_scale=0.0 mandatory for Turbo models (distilled, no CFG path).
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
