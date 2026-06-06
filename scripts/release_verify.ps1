# OpenRollback release verification (Windows)
# 4. Clean build + Release .exe launch
# 5. Chinese/space path smoke (via Rust binary)
# 6. Sad path tests (via Rust binary)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Write-Banner([string]$Text) {
    Write-Host ""
    Write-Host "============================================================"
    Write-Host $Text
    Write-Host "============================================================"
}

Write-Banner "[4] Clean Room Build - npm run tauri build"
npm run tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$ReleaseExe = Join-Path $Root "src-tauri\target\release\openrollback.exe"
$NsisExe = Join-Path $Root "src-tauri\target\release\bundle\nsis\OpenRollback_0.1.0_x64-setup.exe"

if (-not (Test-Path $ReleaseExe)) {
    Write-Error "Release binary not found: $ReleaseExe"
}

Write-Host ""
Write-Host "  Artifacts:"
Write-Host "    EXE:  $ReleaseExe"
if (Test-Path $NsisExe) {
    Write-Host "    NSIS: $NsisExe"
}

Write-Banner "[4b] Release .exe cold start (no dev server)"
$proc = Start-Process -FilePath $ReleaseExe -PassThru -WindowStyle Normal
Start-Sleep -Seconds 10

if ($proc.HasExited) {
    Write-Error "Release .exe exited within 10s (code=$($proc.ExitCode)) - possible crash or white screen"
}

Write-Host "  OK: Release .exe ran stable for 10 seconds"
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

Write-Banner "[5-6] Engine smoke + sad path (release mode)"
cargo run --release --manifest-path src-tauri/Cargo.toml --bin openrollback-release-verify --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Banner "PASS: All release verification checks completed"
