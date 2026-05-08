"""Smoke tests for the SDXL sidecar HTTP surface.

Tests stub `pipeline.SdxlPipeline.__init__` so torch / diffusers do not need to
load real weights or even be installed for the test to run.
"""
from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, str(Path(__file__).parent.parent))


@pytest.fixture(name="client")
def _client(tmp_path):
    with patch("pipeline.SdxlPipeline.__init__", return_value=None):
        from app import create_app

        app = create_app(tmp_path)
        yield TestClient(app)


def test_healthz_returns_ok(client):
    r = client.get("/healthz")
    assert r.status_code == 200
    assert r.json() == {"status": "ok"}


def test_unload_returns_ok(client):
    with patch("pipeline.SdxlPipeline.unload", return_value=None):
        r = client.post("/unload")
    assert r.status_code == 200
    assert r.json() == {"status": "unloaded"}
