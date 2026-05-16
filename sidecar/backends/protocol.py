"""Backend protocol shared by all image + video generators registered in
PipelineDispatcher. Each backend lazy-loads its model on `load()`, frees VRAM
on `unload()`, and produces PNG / MP4 bytes from a PromptParams."""
from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, Literal, Optional, Protocol, runtime_checkable


@dataclass
class PromptParams:
    text: str
    negative: Optional[str] = None
    seed: Optional[int] = None
    steps: Optional[int] = None
    guidance: Optional[float] = None
    resolution: tuple[int, int] = (1024, 1024)
    style_lora: Optional[str] = None
    # video-only
    frame_count: Optional[int] = None
    teacache_threshold: Optional[float] = None


@runtime_checkable
class GenerationBackend(Protocol):
    name: ClassVar[str]
    modality: ClassVar[Literal["image", "video"]]
    vram_estimate_bytes: ClassVar[int]

    def load(self) -> None: ...
    def unload(self) -> None: ...
    def generate(self, params: PromptParams) -> bytes: ...
