"""Lazy-loaded SDXL-Turbo wrapper.

The diffusers pipeline is materialised on the first generate() call so the
process startup stays cheap. unload() drops the in-GPU weights so the LLM
sidecar can claim VRAM during tool-call cycles.
"""
from __future__ import annotations

import threading
from pathlib import Path
from typing import Optional

import torch
from diffusers import AutoPipelineForText2Image


class SdxlPipeline:
    def __init__(self, weights_dir: Path):
        self._weights_dir = weights_dir
        self._pipe: Optional[AutoPipelineForText2Image] = None
        self._lock = threading.Lock()

    def _ensure_loaded(self) -> None:
        with self._lock:
            if self._pipe is None:
                self._pipe = AutoPipelineForText2Image.from_pretrained(
                    str(self._weights_dir),
                    torch_dtype=torch.float16,
                    variant="fp16",
                )
                self._pipe.to("cuda" if torch.cuda.is_available() else "cpu")

    def generate(self, prompt: str, seed: int = 0, steps: int = 4):
        self._ensure_loaded()
        generator = torch.Generator(device=self._pipe.device).manual_seed(seed)
        image = self._pipe(
            prompt=prompt,
            num_inference_steps=steps,
            guidance_scale=0.0,
            generator=generator,
        ).images[0]
        return image

    def unload(self) -> None:
        with self._lock:
            self._pipe = None
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
