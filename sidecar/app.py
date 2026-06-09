"""FastAPI app + uvicorn bootstrap for the local image/video sidecar.

Convention (matches Rust local_runtime expectation):
    first stdout line MUST be `LISTENING_ON_PORT=<n>`.

/generate accepts backend selection and returns base64-encoded bytes inline
(image_b64 for image backends, video_b64 for video backends).

/video/generate accepts a video generation request and streams SSE progress
events (started, progress, done) to the caller. The Rust LocalVideoSidecarProvider
reads this endpoint.
"""
from __future__ import annotations

import argparse
import base64
import json
import queue
import socket
import threading
from pathlib import Path
from typing import Optional

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.responses import StreamingResponse
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


class VideoGenerateRequest(BaseModel):
    """Request body for POST /video/generate (SSE endpoint)."""
    prompt: str
    init_image_b64: Optional[str] = None
    resolution: Optional[tuple[int, int]] = None
    frame_count: Optional[int] = None
    seed: Optional[int] = None


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

    @app.post("/video/generate")
    def video_generate(req: VideoGenerateRequest):
        """Stream video generation progress as SSE events.

        SSE event shapes (exactly what LocalVideoSidecarProvider parses):
          event: started   data: {"type": "started", "estimated_seconds": N}
          event: progress  data: {"type": "progress", "percent": 0.0..1.0, "eta_seconds": N}
          event: done      data: {"type": "done", "mp4_bytes_b64": "<base64>", "duration_seconds": F}
          event: error     data: {"type": "error", "message": "..."}
        """
        backend_id = "ltx-video"
        if backend_id not in app.state.dispatcher.backends:
            raise HTTPException(404, f"backend not found: {backend_id}")

        backend = app.state.dispatcher.backends[backend_id]

        # Build PromptParams from the request.
        resolution = req.resolution or (704, 480)
        params = PromptParams(
            text=req.prompt,
            negative=None,
            seed=req.seed or 0,
            steps=None,
            guidance=None,
            resolution=resolution,
            style_lora=None,
            frame_count=req.frame_count,
        )

        # Progress events are pushed via a thread-safe queue from the callback.
        prog_queue: queue.Queue = queue.Queue()

        def _on_progress(frac: float) -> None:
            prog_queue.put(frac)

        # Wire the progress callback if the backend supports it.
        if hasattr(backend, "set_progress_callback"):
            backend.set_progress_callback(_on_progress)

        result_holder: list = []
        error_holder: list = []

        def _run_generation() -> None:
            try:
                backend.load()
                mp4_bytes = backend.generate(params)
                result_holder.append(mp4_bytes)
            except Exception as exc:  # noqa: BLE001
                error_holder.append(str(exc))
            finally:
                prog_queue.put(None)  # sentinel: generation finished

        gen_thread = threading.Thread(target=_run_generation, daemon=True)

        def _event_stream():
            # started
            yield f"event: started\ndata: {json.dumps({'type': 'started', 'estimated_seconds': 24})}\n\n"

            gen_thread.start()

            steps_done = 0
            while True:
                item = prog_queue.get()
                if item is None:
                    # sentinel - generation done (or errored)
                    break
                steps_done += 1
                frac = float(item)
                eta = max(0, int((1.0 - frac) * 24))
                yield (
                    f"event: progress\n"
                    f"data: {json.dumps({'type': 'progress', 'percent': round(frac, 3), 'eta_seconds': eta})}\n\n"
                )

            if error_holder:
                yield f"event: error\ndata: {json.dumps({'type': 'error', 'message': error_holder[0]})}\n\n"
                return

            mp4_bytes = result_holder[0] if result_holder else b""
            b64 = base64.b64encode(mp4_bytes).decode("ascii")
            payload = json.dumps({
                "type": "done",
                "mp4_bytes_b64": b64,
                "duration_seconds": round((req.frame_count or 97) / 24.0, 2),
            })
            yield f"event: done\ndata: {payload}\n\n"

        return StreamingResponse(_event_stream(), media_type="text/event-stream")

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
