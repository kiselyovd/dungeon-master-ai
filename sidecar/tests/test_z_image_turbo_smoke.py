"""GPU smoke for ZImageTurboBackend - currently SKIPPED (deferred).

See backends/z_image_turbo.py docstring. ZImagePipeline + Qwen3Model are not
in the pinned diffusers/transformers versions; backend raises NotImplementedError
on load(). Revisit when diffusers 0.36 ships stable."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.z_image_turbo import ZImageTurboBackend  # noqa: E402


@pytest.mark.gpu
def test_z_image_turbo_smokes_on_cuda(gpu_weights_dir):
    backend = ZImageTurboBackend(weights_dir=gpu_weights_dir)
    backend.load()
    out = backend.generate(PromptParams(
        text="a wise wizard in a tower library, candlelight",
        steps=8,
        seed=42,
        resolution=(1024, 1024),
    ))
    backend.unload()
    assert out[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {out[:16]!r}"
    assert len(out) > 100_000

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "quality-oss-wizard.png").write_bytes(out)
