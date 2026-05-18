"""HTTP-surface tests for the local image/video sidecar.

Tests inject stub backends into app.state.dispatcher.backends so torch/diffusers
are never touched and no GPU is required.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import ClassVar

import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).parent.parent))


@pytest.fixture(name="app_with_weights")
def _app_with_weights(tmp_path):
    """Returns (app, weights_dir). Caller swaps dispatcher.backends[id] to stubs."""
    from app import create_app

    app = create_app(tmp_path)
    return app, tmp_path


def test_healthz_returns_ok(app_with_weights):
    app, _ = app_with_weights
    r = TestClient(app).get("/healthz")
    assert r.status_code == 200
    assert r.json() == {"status": "ok"}


def test_unload_returns_ok(app_with_weights):
    app, _ = app_with_weights
    r = TestClient(app).post("/unload")
    assert r.status_code == 200
    assert r.json() == {"status": "unloaded"}


class _StubImage:
    vram_estimate_bytes: ClassVar[int] = 1

    def __init__(self, name: str = "balanced") -> None:
        self.name = name
        self.modality = "image"
        self.calls = []

    def load(self) -> None: ...

    def unload(self) -> None: ...

    def generate(self, params):
        self.calls.append(params)
        return b"\x89PNG\r\n\x1a\nFAKE"


class _StubVideo:
    vram_estimate_bytes: ClassVar[int] = 1

    def __init__(self) -> None:
        self.name = "ltx-video"
        self.modality = "video"

    def load(self) -> None: ...

    def unload(self) -> None: ...

    def generate(self, params):
        return b"\x00\x00\x00\x18ftypmp42FAKE"


def test_generate_routes_to_named_backend(app_with_weights):
    app, _ = app_with_weights
    stub = _StubImage(name="balanced")
    app.state.dispatcher.backends["balanced"] = stub

    r = TestClient(app).post("/generate", json={
        "prompt": "a wizard",
        "backend": "balanced",
        "seed": 7,
        "steps": 4,
        "width": 512,
        "height": 512,
    })
    assert r.status_code == 200
    body = r.json()
    assert body["mime"] == "image/png"
    assert "image_b64" in body
    assert len(stub.calls) == 1
    assert stub.calls[0].text == "a wizard"
    assert stub.calls[0].resolution == (512, 512)
    assert stub.calls[0].seed == 7
    assert stub.calls[0].steps == 4


def test_generate_video_backend_returns_mp4_key(app_with_weights):
    app, _ = app_with_weights
    app.state.dispatcher.backends["ltx-video"] = _StubVideo()

    r = TestClient(app).post("/generate", json={"prompt": "fog rolls in", "backend": "ltx-video"})
    assert r.status_code == 200
    body = r.json()
    assert body["mime"] == "video/mp4"
    assert "video_b64" in body
    assert "image_b64" not in body


def test_generate_unknown_backend_404(app_with_weights):
    app, _ = app_with_weights
    r = TestClient(app).post("/generate", json={"prompt": "x", "backend": "no-such-backend"})
    assert r.status_code == 404


def test_generate_defaults_to_fast_backend_when_field_omitted(app_with_weights):
    app, _ = app_with_weights
    stub = _StubImage(name="fast")
    app.state.dispatcher.backends["fast"] = stub

    r = TestClient(app).post("/generate", json={"prompt": "x"})
    assert r.status_code == 200
    assert len(stub.calls) == 1
