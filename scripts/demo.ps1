# OpenRollback end-to-end demo script (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "OpenRollback: AI Time Machine"
Write-Host "=============================="
Write-Host ""

Write-Host "[1/2] Running end-to-end demo..."
cargo run --manifest-path src-tauri/Cargo.toml --bin openrollback-demo --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "[2/2] Running unit tests..."
cargo test --manifest-path src-tauri/Cargo.toml --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "PASS: All demo checks passed"
