"""LTX-Video 0.9.6 distilled backend (~20-28s/clip, 704x480) on RTX 3080.

Shares T5xxl text encoder with FLUX Quality preset via the manifest's
requires graph. NOTE: full diffusers LTXPipeline wiring deferred to
M7.5-DM (model download + GPU smoke); skeleton ships here so the
PipelineDispatcher can register it and the Rust SSE route has a target.
"""
from __future__ import annotations

import io
from typing import Callable, ClassVar, Literal, Optional

from backends.protocol import GenerationBackend, PromptParams


class LtxVideoBackend:
    name: ClassVar[str] = "ltx-video"
    modality: ClassVar[Literal["video"]] = "video"
    vram_estimate_bytes: ClassVar[int] = 8 * 1024**3

    def __init__(self) -> None:
        self._pipe = None
        self._progress_callback: Optional[Callable[[float], None]] = None

    def set_progress_callback(self, cb: Callable[[float], None]) -> None:
        """Wired by the HTTP layer when streaming SSE so each diffusion step
        emits a Progress event to the client."""
        self._progress_callback = cb

    def load(self) -> None:
        if self._pipe is not None:
            return
        # Real: `from diffusers import LTXPipeline` + distilled checkpoint
        # + T5xxl encoder (shared via manifest deps). Deferred M7.5-DM.
        raise NotImplementedError(
            "LtxVideoBackend.load() is wired in M7.5-DM; "
            "use the pre-recorded mp4 library fallback for now."
        )

    def unload(self) -> None:
        self._pipe = None

    def generate(self, params: PromptParams) -> bytes:  # pragma: no cover
        assert self._pipe is not None, "call load() first"
        del params
        return io.BytesIO().getvalue()
