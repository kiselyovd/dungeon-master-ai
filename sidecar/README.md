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

## Nunchaku install (Quality + Quality-OSS backends, optional)

`nunchaku` is intentionally NOT in `requirements.txt` - PyPI has a different
unrelated package under the same name (a Bayesian sampler library). The real
SVDQuant Nunchaku from nunchaku-tech ships prebuilt wheels via GitHub Releases
(the mit-han-lab HF mirror is frozen at 0.3.1; v1.x and Z-Image support live
on GitHub).

Install on Windows + Python 3.12 + torch 2.8:

    gh release download v1.2.1 --repo nunchaku-tech/nunchaku \
      --pattern "nunchaku-1.2.1+cu12.8torch2.8-cp312-cp312-win_amd64.whl"
    uv pip install nunchaku-1.2.1+cu12.8torch2.8-cp312-cp312-win_amd64.whl

Adjust the asset pattern per your Python + torch + CUDA version - browse
https://github.com/nunchaku-tech/nunchaku/releases.

Without nunchaku the `quality` (Nunchaku FLUX-dev INT4) and `quality-oss`
(Z-Image-Turbo SVDQ INT4 r128) backends are unavailable. The `balanced`
(SDXL-Lightning Apache 2.0) backend stays fully functional and is the
recommended default anyway.

## Torch CUDA install

torch is NOT in requirements.txt because PyPI ships CPU-only wheels by default
on Windows. Install with CUDA 12.8 wheels:

    uv pip install --index-url https://download.pytorch.org/whl/cu128 torch==2.8.0 torchvision==0.23.0

torch 2.8 is the minimum compatible version for the nunchaku 1.2.x cp312
win_amd64 wheel. CUDA 12.8 toolkit on the host is NOT required - the wheel
bundles its own cudart.

## diffusers from git main

requirements.txt pins `diffusers @ git+https://github.com/huggingface/diffusers.git@main`
because the Quality-OSS backend (Z-Image-Turbo) needs `ZImagePipeline` and
`ZImageTransformer2DModel`, which only landed in unreleased 0.36+ dev. Switch back
to a pinned release once 0.36 (or whichever version ships ZImagePipeline) is on PyPI.

transformers is pinned `>=4.51,<5.0` because Z-Image-Turbo's text encoder is
Qwen3 (added in 4.51) but transformers 5.x removed symbols diffusers still imports.
