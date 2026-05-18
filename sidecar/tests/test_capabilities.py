"""Unit tests for VRAM preflight helper. Mocks torch.cuda so no GPU required."""
from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import patch

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from backends._capabilities import InsufficientVramError, assert_vram_free  # noqa: E402


def test_assert_vram_free_passes_when_enough():
    with patch("backends._capabilities._mem_get_info", return_value=(8 * 1024**3, 10 * 1024**3)):
        assert_vram_free(7 * 1024**3)


def test_assert_vram_free_raises_when_short():
    with patch("backends._capabilities._mem_get_info", return_value=(4 * 1024**3, 10 * 1024**3)):
        with pytest.raises(InsufficientVramError) as exc_info:
            assert_vram_free(7 * 1024**3)
        assert exc_info.value.available_bytes == 4 * 1024**3
        assert exc_info.value.required_bytes == 7 * 1024**3


def test_assert_vram_free_no_cuda_raises_runtime_error():
    with patch("backends._capabilities._mem_get_info", side_effect=RuntimeError("no cuda")):
        with pytest.raises(RuntimeError, match="no cuda"):
            assert_vram_free(1)
