# dmai-image-sidecar

[Documentation](../docs/README.md) > Python sidecar | [Configuration](../docs/CONFIGURATION.md) | [Contributing](../CONTRIBUTING.md)

Optional Python 3.12 image/video generation sidecar for Dungeon Master AI. It dispatches the configured SDXL, Nunchaku/Z-Image and LTX backends. It is separate from the standard cloud-only desktop bundle and requires the chosen backend's runtime and model weights.

## Run from source

Run dependency setup and offline work inside an approved Docker/dev container. Keep dependencies in its filesystem or named volumes and mount existing model weights read-only at the configured model root. Do not create a host `.venv` as the routine setup path. Native GPU/packaging work needs a separately approved target-OS environment; see [Contributing](../CONTRIBUTING.md).

After installing the project requirements and the selected CPU/GPU torch build in that environment, run from `sidecar/`:

```bash
python app.py --weights-dir /models
```

The first stdout line is `LISTENING_ON_PORT=<n>`, followed by a health endpoint at `http://127.0.0.1:<n>/healthz`. In the desktop application, Tauri owns this child process and the backend accesses it through the runtime-control boundary. A standalone source run is a separate development process.

## Tests

The offline suite does not require model downloads or a GPU. In a Python 3.12 container with the minimal test dependencies listed in `.github/workflows/test.yml`, run from the repository root:

```bash
python -m ruff check sidecar
RUN_GPU_TESTS=0 python -m pytest sidecar/tests -q
```

The tests inject stub backends. Real GPU/model checks require `RUN_GPU_TESTS=1` and are a separate acceptance stage.

## Build the standalone binary

Use `sidecar/scripts/build.ps1` or `build.sh` in the approved target-platform build environment; the Python prebuild workflow shows its dependency setup. PyInstaller's spec produces the executable `dist/dmai-image-sidecar` (`.exe` on Windows). The scripts stage it as `src-tauri/binaries/dmai-image-sidecar-<target-triple>[.exe]`, matching Tauri's `externalBin` contract. This artifact is not included in the normal cloud release matrix.

## Nunchaku install (Quality + Quality-OSS backends, optional)

`nunchaku` is intentionally NOT in `requirements.txt` - PyPI has a different
unrelated package under the same name (a Bayesian sampler library). The real
SVDQuant Nunchaku from nunchaku-tech ships prebuilt wheels via GitHub Releases
(the mit-han-lab HF mirror is frozen at 0.3.1; v1.x and Z-Image support live
on GitHub).

The following optional dependency commands belong in the approved matching Windows/Python 3.12/torch 2.8 GPU environment, not an unrelated host environment:

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
for the Quality-OSS backend's `ZImagePipeline` and `ZImageTransformer2DModel` APIs.
Follow the current manifest rather than treating an old version note as proof of
release availability. Replacing this source dependency is a separate dependency
change that needs compatibility checks.

The manifest bounds transformers to `>=4.51,<5.0` for the existing Qwen3 text
encoder/backend compatibility. Do not upgrade that range as part of documentation
or optional runtime setup.

## See Also

- [Configuration](../docs/CONFIGURATION.md)
- [Release packaging](../docs/RELEASE.md)
- [Contributing](../CONTRIBUTING.md)
