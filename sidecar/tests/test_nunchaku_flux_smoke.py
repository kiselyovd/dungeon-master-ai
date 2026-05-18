"""GPU smoke for NunchakuFluxBackend. Requires RUN_GPU_TESTS=1, ~16 GB weights, nunchaku installed."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.nunchaku_flux import NunchakuFluxBackend  # noqa: E402


@pytest.mark.gpu
def test_nunchaku_flux_smokes_on_cuda(gpu_weights_dir):
    pytest.importorskip("nunchaku", reason="install nunchaku per sidecar/README.md")
    backend = NunchakuFluxBackend(weights_dir=gpu_weights_dir)
    backend.load()
    out = backend.generate(PromptParams(
        text="ornate elven longbow, mithril inlay, fantasy concept art",
        steps=8,
        seed=42,
        resolution=(1024, 1024),
    ))
    backend.unload()

    assert out[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {out[:16]!r}"
    assert len(out) > 100_000

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "quality-elven-bow.png").write_bytes(out)
