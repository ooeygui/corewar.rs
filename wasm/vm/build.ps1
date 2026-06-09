$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')

Push-Location $RepoRoot
try {
    wasm-pack build crates/corewar-vm --target web --out-dir ../../wasm/vm/pkg -- --features wasm
}
finally {
    Pop-Location
}
