# CoreWar VM WASM

This directory contains a minimal browser test harness for the `corewar-vm` WebAssembly build.

## Requirements

- `wasm-pack`
- Rust with the `wasm32-unknown-unknown` target installed

```powershell
rustup target add wasm32-unknown-unknown
```

## Build

From the repository root:

```powershell
.\wasm\vm\build.ps1
```

That runs:

```powershell
wasm-pack build crates/corewar-vm --target web --out-dir ../../wasm/vm/pkg -- --features wasm
```

## Test in a browser

Serve the `wasm/vm` directory with any static file server, for example:

```powershell
cd wasm\vm
python -m http.server 8000
```

Then open <http://localhost:8000/>. The page loads Imp and Dwarf, runs the VM to completion, and prints the JSON result.
