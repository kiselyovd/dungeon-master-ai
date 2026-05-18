"""Shared pytest configuration: gpu marker skip + weights dir fixture."""
from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


def pytest_collection_modifyitems(config, items):
    if os.environ.get("RUN_GPU_TESTS") == "1":
        return
    skip_gpu = pytest.mark.skip(reason="set RUN_GPU_TESTS=1 to run GPU smoke tests")
    for item in items:
        if "gpu" in item.keywords:
            item.add_marker(skip_gpu)


@pytest.fixture(scope="session")
def gpu_weights_dir() -> Path:
    """Per-backend subfolder root. Default ~/.cache/dm-ai-gpu-weights; override
    via DM_AI_GPU_WEIGHTS_DIR. NOT cleaned up between sessions so smoke runs
    reuse weights. snapshot_download caches independently in HF_HOME."""
    default = Path.home() / ".cache" / "dm-ai-gpu-weights"
    root = Path(os.environ.get("DM_AI_GPU_WEIGHTS_DIR", default))
    root.mkdir(parents=True, exist_ok=True)
    return root
