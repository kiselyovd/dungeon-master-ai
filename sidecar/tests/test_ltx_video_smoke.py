"""GPU smoke for LtxVideoBackend. Requires RUN_GPU_TESTS=1 + ~9 GB weights."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.ltx_video import LtxVideoBackend  # noqa: E402


@pytest.mark.gpu
def test_ltx_video_smokes_on_cuda(gpu_weights_dir):
    backend = LtxVideoBackend(weights_dir=gpu_weights_dir)

    progress_calls = []
    backend.set_progress_callback(lambda p: progress_calls.append(p))

    backend.load()
    out = backend.generate(PromptParams(
        text="fog rolls through a dim dungeon corridor, torchlight flickers on stone walls",
        steps=8,
        seed=42,
        frame_count=97,
        resolution=(704, 480),
    ))
    backend.unload()

    # MP4 magic - ftyp box at offset 4.
    assert out[4:8] == b"ftyp", f"not an MP4: {out[:16]!r}"
    assert len(out) > 100_000, f"MP4 too small: {len(out)} bytes"
    assert len(progress_calls) == 8, f"expected 8 progress callbacks, got {len(progress_calls)}"
    assert progress_calls[-1] == pytest.approx(1.0)

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "ltx-fog-corridor.mp4").write_bytes(out)
