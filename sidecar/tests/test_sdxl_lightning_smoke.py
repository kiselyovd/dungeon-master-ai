"""GPU smoke for SdxlLightningBackend. Requires RUN_GPU_TESTS=1 + ~7 GB weights."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.sdxl_lightning import SdxlLightningBackend  # noqa: E402


@pytest.mark.gpu
def test_sdxl_lightning_smokes_on_cuda(gpu_weights_dir):
    """Real load + generate. Saves output to samples/ for visual check."""
    backend = SdxlLightningBackend(weights_dir=gpu_weights_dir)
    backend.load()
    out = backend.generate(PromptParams(
        text="a noble paladin in plate armor, fantasy art, highly detailed",
        steps=4,
        seed=42,
        resolution=(1024, 1024),
    ))
    backend.unload()

    assert out[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {out[:16]!r}"
    assert len(out) > 100_000, f"PNG too small: {len(out)} bytes"

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "balanced-paladin.png").write_bytes(out)
