"""FastAPI app + uvicorn bootstrap for the local SDXL sidecar.

Convention (matches Rust `local_runtime` expectation):
    first stdout line MUST be `LISTENING_ON_PORT=<n>`.

`/generate` returns base64-encoded PNG bytes inline (no filesystem hop).
Rust LocalSdxlSidecarProvider decodes on receipt.
"""
from __future__ import annotations

import argparse
import base64
import io
import socket
from pathlib import Path

import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class GenerateRequest(BaseModel):
    prompt: str
    seed: int = 0
    steps: int = 4


def create_app(weights_dir: Path):
    """Late-import the heavy pipeline so unit tests can stub it via monkeypatch."""
    from pipeline import SdxlPipeline

    app = FastAPI()
    pipeline = SdxlPipeline(weights_dir)

    @app.get("/healthz")
    def healthz():
        return {"status": "ok"}

    @app.post("/generate")
    def generate(req: GenerateRequest):
        try:
            image = pipeline.generate(req.prompt, req.seed, req.steps)
        except RuntimeError as exc:
            raise HTTPException(503, str(exc)) from exc
        buf = io.BytesIO()
        image.save(buf, format="PNG")
        b64 = base64.b64encode(buf.getvalue()).decode("ascii")
        return {"image_b64": b64, "mime": "image/png"}

    @app.post("/unload")
    def unload():
        pipeline.unload()
        return {"status": "unloaded"}

    return app


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--weights-dir", type=Path, required=True)
    args = parser.parse_args()

    port = args.port if args.port > 0 else _free_port()
    print(f"LISTENING_ON_PORT={port}", flush=True)

    app = create_app(args.weights_dir)
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")


if __name__ == "__main__":
    main()
