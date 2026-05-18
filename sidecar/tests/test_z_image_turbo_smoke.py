"""GPU smoke for ZImageTurboBackend (SVDQuant INT4 r128).

Requires RUN_GPU_TESTS=1, nunchaku 1.1.0+ installed, plus ~4 GB SVDQ safetensors
and the full ~30 GB Tongyi-MAI/Z-Image-Turbo base diffusers folder in the
weights cache. The SVDQ file auto-downloads on first run via hf_hub_download."""
from __future__ import annotations

import sys
import time
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends.protocol import PromptParams  # noqa: E402
from backends.z_image_turbo import ZImageTurboBackend  # noqa: E402


@pytest.mark.gpu
def test_z_image_turbo_svdq_smokes_on_cuda(gpu_weights_dir):
    pytest.importorskip("nunchaku", reason="install nunchaku 1.1.0+ per sidecar/README.md")

    # Auto-fetch SVDQ INT4 r128 safetensors if missing (4.01 GB).
    svdq_path = gpu_weights_dir / "quality-oss" / "svdq-int4_r128-z-image-turbo.safetensors"
    if not svdq_path.exists():
        from huggingface_hub import hf_hub_download  # noqa: PLC0415
        svdq_path.parent.mkdir(parents=True, exist_ok=True)
        local = hf_hub_download(
            repo_id="nunchaku-tech/nunchaku-z-image-turbo",
            filename="svdq-int4_r128-z-image-turbo.safetensors",
            local_dir=str(svdq_path.parent),
        )
        assert Path(local).exists()

    backend = ZImageTurboBackend(weights_dir=gpu_weights_dir)
    t0 = time.perf_counter()
    backend.load()
    out = backend.generate(PromptParams(
        text="a wise wizard in a tower library, candlelight",
        steps=8,
        seed=42,
        resolution=(1024, 1024),
    ))
    wall = time.perf_counter() - t0
    backend.unload()

    assert out[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {out[:16]!r}"
    assert len(out) > 100_000

    samples_dir = Path(__file__).parent.parent.parent / "samples"
    samples_dir.mkdir(exist_ok=True)
    (samples_dir / "quality-oss-wizard.png").write_bytes(out)

    # Spec target: 3-5 s/img steady state; observed ~30-35s incl. cold load + text
    # encode + 8 steps + decode on RTX 3080 (model_cpu_offload swaps Qwen3-4B
    # encoder out before the diffusion loop, keeping peak VRAM at ~7 GB).
    assert wall < 90, f"wall {wall:.1f}s exceeds 90s budget"
