"""Quality preset (Nunchaku FLUX.1-dev INT4 SVDQuant + Turbo-Alpha 8-step LoRA).

License: FLUX-dev non-commercial. Forbidden on RTX 3080: FP8 anywhere,
torch.compile over Nunchaku, SageAttention v2/v3 (v1 only). See
crates/app-server hardware constraint comments.

NOTE: Real wiring deferred to M7.5-DM (requires nunchaku pip install +
GPU smoke test). Skeleton ships here so PipelineDispatcher can register it.
"""
from __future__ import annotations

import io
from typing import ClassVar, Literal

from backends.protocol import GenerationBackend, PromptParams


class NunchakuFluxBackend:
    name: ClassVar[str] = "quality"
    modality: ClassVar[Literal["image"]] = "image"
    vram_estimate_bytes: ClassVar[int] = int(6.5 * 1024**3)

    def __init__(self) -> None:
        self._pipe = None

    def load(self) -> None:
        if self._pipe is not None:
            return
        raise NotImplementedError(
            "NunchakuFluxBackend.load() is wired in M7.5-DM; "
            "select Balanced or Fast preset for now."
        )

    def unload(self) -> None:
        self._pipe = None

    def generate(self, params: PromptParams) -> bytes:  # pragma: no cover
        assert self._pipe is not None, "call load() first"
        del params
        return io.BytesIO().getvalue()
