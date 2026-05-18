"""VRAM preflight helper. Each backend's load() calls assert_vram_free(bytes)
before pulling weights to GPU. On insufficient VRAM, raises a structured error
that the FastAPI route maps to HTTP 503."""
from __future__ import annotations


class InsufficientVramError(RuntimeError):
    def __init__(self, required_bytes: int, available_bytes: int) -> None:
        self.required_bytes = required_bytes
        self.available_bytes = available_bytes
        super().__init__(
            f"insufficient VRAM: need {required_bytes / 1024**3:.1f} GB free, "
            f"have {available_bytes / 1024**3:.1f} GB"
        )


def _mem_get_info() -> tuple[int, int]:
    """Wrapper kept separate so tests can monkey-patch without importing torch."""
    import torch  # noqa: PLC0415

    return torch.cuda.mem_get_info()


def assert_vram_free(required_bytes: int) -> None:
    """Raise InsufficientVramError if device 0 has less than required_bytes free.
    Propagates the underlying RuntimeError if CUDA is unavailable."""
    available, _total = _mem_get_info()
    if available < required_bytes:
        raise InsufficientVramError(required_bytes, available)
