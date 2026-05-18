"""FastAPI app + uvicorn bootstrap for the local image/video sidecar.

Convention (matches Rust local_runtime expectation):
    first stdout line MUST be `LISTENING_ON_PORT=<n>`.

/generate accepts backend selection and returns base64-encoded bytes inline
(image_b64 for image backends, video_b64 for video backends).
"""
from __future__ import annotations

import argparse
import base64
import socket
from pathlib import Path
from typing import Optional

import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from backends.protocol import PromptParams


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class GenerateRequest(BaseModel):
    prompt: str
    seed: int = 0
    steps: Optional[int] = None
    backend: str = "fast"
    negative: Optional[str] = None
    guidance: Optional[float] = None
    width: Optional[int] = None
    height: Optional[int] = None
    style_lora: Optional[str] = None
    frame_count: Optional[int] = None


def _to_params(req: GenerateRequest) -> PromptParams:
    if req.width and req.height:
        resolution = (req.width, req.height)
    else:
        resolution = (1024, 1024)
    return PromptParams(
        text=req.prompt,
        negative=req.negative,
        seed=req.seed,
        steps=req.steps,
        guidance=req.guidance,
        resolution=resolution,
        style_lora=req.style_lora,
        frame_count=req.frame_count,
    )


def create_app(weights_dir: Path):
    """Build the FastAPI app with a fresh PipelineDispatcher attached to app.state.
    Tests can swap individual dispatcher.backends[id] to stubs."""
    from pipeline import PipelineDispatcher

    app = FastAPI()
    app.state.dispatcher = PipelineDispatcher.production(weights_dir)
    app.state.weights_dir = weights_dir

    @app.get("/healthz")
    def healthz():
        return {"status": "ok"}

    @app.get("/backends")
    def backends():
        out = []
        for backend_id, backend in app.state.dispatcher.backends.items():
            sub = app.state.weights_dir / backend_id
            out.append({
                "id": backend_id,
                "modality": backend.modality,
                "installed": sub.exists(),
            })
        return {"backends": out}

    @app.post("/generate")
    def generate(req: GenerateRequest):
        if req.backend not in app.state.dispatcher.backends:
            raise HTTPException(404, f"unknown backend: {req.backend}")
        backend = app.state.dispatcher.backends[req.backend]
        try:
            payload = app.state.dispatcher.generate(req.backend, _to_params(req))
        except RuntimeError as exc:
            raise HTTPException(503, str(exc)) from exc
        b64 = base64.b64encode(payload).decode("ascii")
        if backend.modality == "video":
            return {"video_b64": b64, "mime": "video/mp4"}
        return {"image_b64": b64, "mime": "image/png"}

    @app.post("/unload")
    def unload():
        loaded = app.state.dispatcher.loaded
        if loaded is not None:
            app.state.dispatcher.backends[loaded].unload()
            app.state.dispatcher.loaded = None
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
