# CoreWar WASM visualization

This directory contains the browser entry point for `corewar-viz`.

## Requirements

- Rust with the `wasm32-unknown-unknown` target installed
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/)

Install the target if needed:

```bash
rustup target add wasm32-unknown-unknown
```

## Build

From the repository root:

```bash
./wasm/build.sh
```

On Windows PowerShell:

```powershell
.\wasm\build.ps1
```

The generated package is written to `wasm/pkg/`.

## Serve locally

Serve the `wasm/` directory so `index.html` can load `pkg/` assets:

```bash
cd wasm
python -m http.server 8000
```

Then open:

```text
http://localhost:8000/
```

## Connecting to a server

By default the page connects to `ws://localhost:9000` and subscribes to `arena-1`.
You can override both with query parameters:

```text
http://localhost:8000/?server=ws://localhost:9000&instance=arena-2
```

`server` (or `ws`) sets the WebSocket endpoint and `instance` (or `instance_id`) selects the battle instance.
