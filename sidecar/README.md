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
