"""Backend protocol + concrete backends for the M7-DM image/video dispatcher."""

from backends.protocol import GenerationBackend, PromptParams

__all__ = ["GenerationBackend", "PromptParams"]
