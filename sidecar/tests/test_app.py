"""HTTP-surface tests for the local image/video sidecar.

Tests inject stub backends into app.state.dispatcher.backends so torch/diffusers
are never touched and no GPU is required.
"""
from __future__ import annotations

import base64
import json
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


class _StubVideoWithProgress:
    """Stub video backend that fires a progress callback before returning."""
    vram_estimate_bytes: ClassVar[int] = 1

    def __init__(self) -> None:
        self.name = "ltx-video"
        self.modality = "video"
        self._progress_callback = None

    def set_progress_callback(self, cb) -> None:
        self._progress_callback = cb

    def load(self) -> None: ...

    def unload(self) -> None: ...

    def generate(self, params):
        if self._progress_callback is not None:
            self._progress_callback(0.5)
        return b"\x00\x00\x00\x18ftypmp42FAKE_MP4"


def test_video_generate_sse_streams_events(app_with_weights):
    """POST /video/generate returns SSE with started, progress, and done events."""
    app, _ = app_with_weights
    app.state.dispatcher.backends["ltx-video"] = _StubVideoWithProgress()

    r = TestClient(app).post(
        "/video/generate",
        json={"prompt": "fog rolls across the battlefield", "frame_count": 97},
    )
    assert r.status_code == 200
    assert "text/event-stream" in r.headers["content-type"]

    raw = r.text
    # Must contain a started event.
    assert "event: started" in raw
    # Must contain a done event with video_b64.
    assert "event: done" in raw

    # Parse the done payload.
    done_line = next(
        line for line in raw.splitlines() if line.startswith("data:") and '"type": "done"' in line
    )
    payload = json.loads(done_line[len("data: "):])
    assert payload["type"] == "done"
    assert "mp4_bytes_b64" in payload
    # Decode and check we got the fake MP4 bytes back.
    decoded = base64.b64decode(payload["mp4_bytes_b64"])
    assert decoded == b"\x00\x00\x00\x18ftypmp42FAKE_MP4"

    # Must contain a progress event (because _StubVideoWithProgress fires one).
    assert "event: progress" in raw
    progress_line = next(
        line for line in raw.splitlines()
        if line.startswith("data:") and '"type": "progress"' in line
    )
    prog = json.loads(progress_line[len("data: "):])
    assert prog["percent"] == 0.5


def test_video_generate_no_backend_404(app_with_weights):
    """If ltx-video backend is missing, the endpoint returns 404."""
    app, _ = app_with_weights
    # Remove ltx-video so it is not in the dispatch table.
    app.state.dispatcher.backends.pop("ltx-video", None)

    r = TestClient(app).post("/video/generate", json={"prompt": "test"})
    assert r.status_code == 404


def test_backends_endpoint_reports_installed_state(tmp_path):
    """Each backend reports installed=True iff weights_dir/<id>/ exists."""
    from app import create_app

    (tmp_path / "fast").mkdir()

    app = create_app(tmp_path)
    r = TestClient(app).get("/backends")
    assert r.status_code == 200
    rows = {b["id"]: b for b in r.json()["backends"]}
    assert set(rows.keys()) == {"fast", "balanced", "quality", "quality-oss", "ltx-video"}
    assert rows["fast"]["installed"] is True
    assert rows["balanced"]["installed"] is False
    assert rows["ltx-video"]["modality"] == "video"
    assert rows["balanced"]["modality"] == "image"
