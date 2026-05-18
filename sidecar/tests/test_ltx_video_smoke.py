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
    # frame_count=49 (not spec's 97) because Lightricks/LTX-Video's diffusers
    # folder ships the larger v0.9.7+ transformer, not the 2B 0.9.6 distilled
    # single-file checkpoint. On 10 GB Ampere with the full transformer + T5xxl,
    # 97 frames at 704x480 takes 12+ min; 49 frames keeps wall time under 7 min.
    # M9-DM follow-up: switch to from_single_file with ltxv-2b-0.9.6-distilled
    # for the 22-28s target wall time per spec.
    out = backend.generate(PromptParams(
        text="fog rolls through a dim dungeon corridor, torchlight flickers on stone walls",
        steps=8,
        seed=42,
        frame_count=49,
        resolution=(704, 480),
    ))
    backend.unload()

    # MP4 magic - ftyp box at offset 4.
    assert out[4:8] == b"ftyp", f"not an MP4: {out[:16]!r}"
    # x264 with crf=25 compresses heavily; 49 frames at 704x480 lands ~20-80 KB.
    assert len(out) > 20_000, f"MP4 too small: {len(out)} bytes"
    assert len(progress_calls) == 8, f"expected 8 progress callbacks, got {len(progress_calls)}"
    assert progress_calls[-1] == pytest.approx(1.0)

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "ltx-fog-corridor.mp4").write_bytes(out)
