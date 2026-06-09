# corewar.rs

A modern, modular CoreWar system written in Rust. Compiles to native binaries and WebAssembly for browser and Docker deployments.

## Features

- **ICWS'94 + pMARS extensions** — Full instruction set with all addressing modes and modifiers
- **Multi-warrior battles** — Support for 100+ warriors in a single core
- **Tournament orchestrator** — Round-robin, Swiss, and elimination tournaments
- **Glicko-2 leaderboard** — In-memory rating system with optional file persistence
- **WebGPU visualization** — GPU-accelerated rendering of arbitrarily large cores (8K to 800K+ cells)
- **256+ warrior colors** — Perceptually distinct color palette for massive battles
- **WebSocket protocol** — Real-time streaming of battle events
- **WASM support** — Run the VM and visualizer directly in the browser

## Architecture

```
crates/
├── corewar-core         # Shared types: instructions, memory, events
├── corewar-parser       # Redcode assembler (ICWS'94 + pMARS)
├── corewar-vm           # MARS virtual machine
├── corewar-orchestrator # Tournament scheduling
├── corewar-leaderboard  # Rating system & persistence
├── corewar-protocol     # WebSocket message definitions
├── corewar-server       # Tokio WebSocket server
└── corewar-viz          # WebGPU visualization client
```

## Quick Start

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Start the server
cargo run -p corewar-server

# Build for WASM (requires wasm32 target)
rustup target add wasm32-unknown-unknown
cargo build -p corewar-vm --target wasm32-unknown-unknown --no-default-features --features wasm
```

## Docker

```bash
docker compose -f docker/docker-compose.yml up
```

## Warriors

Example warriors are in the `warriors/` directory. See [ICWS'94 standard](http://corewar.co.uk/icws94.txt) for the Redcode language reference.

## License

MIT