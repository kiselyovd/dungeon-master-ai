"""GPU smoke + path-resolution unit for SdxlTurboBackend.

The path unit test runs everywhere (the constructor does not import torch); the
generate smoke is GPU-gated like the other backend smokes.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.sdxl_turbo import SdxlTurboBackend  # noqa: E402


def test_weights_dir_resolves_to_hf_repo_subdir():
    """The DownloadManager writes diffusers folders to <base>/<hf_repo>, and the
    sidecar is launched with that <base> as --weights-dir. The fast backend must
    therefore load from <base>/stabilityai/sdxl-turbo, not the shared <base>
    root (which also holds chat GGUFs) - otherwise from_pretrained 500s."""
    backend = SdxlTurboBackend(weights_dir=Path("/w"))
    assert backend._weights_dir == Path("/w") / "stabilityai" / "sdxl-turbo"


@pytest.mark.gpu
def test_sdxl_turbo_smokes_on_cuda(gpu_weights_dir):
    backend = SdxlTurboBackend(weights_dir=gpu_weights_dir)
    backend.load()
    out = backend.generate(PromptParams(
        text="a cozy medieval tavern interior, fantasy art, warm firelight",
        steps=4,
        seed=42,
        resolution=(1024, 1024),
    ))
    backend.unload()

    assert out[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {out[:16]!r}"
    assert len(out) > 100_000, f"PNG too small: {len(out)} bytes"

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "fast-tavern.png").write_bytes(out)
