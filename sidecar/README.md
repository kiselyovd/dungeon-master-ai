# dmai-image-sidecar

Local SDXL-Turbo image generation sidecar for dungeon-master-ai.

## Run from source

```pwsh
cd sidecar
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -r requirements.txt
python app.py --weights-dir ../models/stabilityai/sdxl-turbo
```

The first stdout line is `LISTENING_ON_PORT=<n>`; the Rust `LocalRuntime`
parses this and probes `http://127.0.0.1:<n>/healthz`.

## Tests

```pwsh
pip install pytest httpx
pytest tests/
```

Tests stub the diffusers pipeline so torch is the only heavy import that
survives.

## Build the standalone binary

```pwsh
pip install pyinstaller==6.10
pyinstaller --noconfirm --clean build_spec.spec
```

The resulting `dist/dmai-image-sidecar/` directory ships into
`src-tauri/binaries/python_sidecar_<target-triple>/` for Tauri's `externalBin`.

## Nunchaku install (Quality backend, optional)

`nunchaku` is intentionally NOT in `requirements.txt` - PyPI has a different
unrelated package under the same name (a Bayesian sampler library). The real
SVDQuant Nunchaku from mit-han-lab ships prebuilt wheels via their HF mirror.

Install on Windows + Python 3.12 + torch 2.5:

    pip install https://huggingface.co/mit-han-lab/nunchaku/resolve/main/nunchaku-0.3.1+torch2.5-cp312-cp312-win_amd64.whl

Adjust the URL per your Python + torch version - browse the wheel list at
https://huggingface.co/mit-han-lab/nunchaku/tree/main.

Without it the `quality` backend (Nunchaku FLUX-dev INT4) raises
NotImplementedError. The `balanced` (SDXL-Lightning Apache 2.0) backend stays
fully functional and is the recommended default anyway.

## Torch CUDA install

torch is NOT in requirements.txt because PyPI ships CPU-only wheels by default
on Windows. Install with CUDA 12.1 wheels:

    uv pip install --index-url https://download.pytorch.org/whl/cu121 torch==2.5.1 torchvision==0.20.1

torch 2.5.1 is the minimum compatible version for nunchaku 0.3.1 (cp312).

## diffusers from git main

requirements.txt pins `diffusers @ git+https://github.com/huggingface/diffusers.git@main`
because the Quality-OSS backend (Z-Image-Turbo) needs `ZImagePipeline` and
`ZImageTransformer2DModel`, which only landed in unreleased 0.36+ dev. Switch back
to a pinned release once 0.36 (or whichever version ships ZImagePipeline) is on PyPI.

transformers is pinned `>=4.51,<5.0` because Z-Image-Turbo's text encoder is
Qwen3 (added in 4.51) but transformers 5.x removed symbols diffusers still imports.
