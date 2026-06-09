$ErrorActionPreference = 'Stop'

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir

Push-Location $RepoRoot
try {
    wasm-pack build crates/corewar-viz --target web --out-dir ../../wasm/pkg -- --features wasm
    Copy-Item -Path (Join-Path $ScriptDir 'index.html') -Destination (Join-Path $ScriptDir 'pkg\index.html') -Force
    Write-Host "Built WASM package in $(Join-Path $ScriptDir 'pkg')"
}
finally {
    Pop-Location
}
