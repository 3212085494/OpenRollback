# OpenRollback E2E autotest (Windows)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host ""
Write-Host "============================================================"
Write-Host "OpenRollback E2E autotest"
Write-Host "============================================================"
Write-Host "Test dir: $env:USERPROFILE\Desktop\openrollback_test"
Write-Host ""

cargo run --manifest-path src-tauri/Cargo.toml --bin openrollback-e2e-autotest
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "PASS: E2E autotest completed"
