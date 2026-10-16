"""Backend protocol shared by all image + video generators registered in
PipelineDispatcher. Each backend lazy-loads its model on `load()`, frees VRAM
on `unload()`, and produces PNG / MP4 bytes from a PromptParams."""
from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, Literal, Protocol, runtime_checkable


@dataclass
class PromptParams:
    text: str
    negative: str | None = None
    seed: int | None = None
    steps: int | None = None
    guidance: float | None = None
    resolution: tuple[int, int] = (1024, 1024)
    style_lora: str | None = None
    # video-only
    frame_count: int | None = None


@runtime_checkable
class GenerationBackend(Protocol):
    name: ClassVar[str]
    modality: ClassVar[Literal["image", "video"]]
    vram_estimate_bytes: ClassVar[int]

    def load(self) -> None: ...
    def unload(self) -> None: ...
    def generate(self, params: PromptParams) -> bytes: ...
