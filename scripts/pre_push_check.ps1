# Pre-push checklist: verify artifacts and ignored paths
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$ok = $true
function Check($cond, $msg) {
    if ($cond) { Write-Host "  OK  $msg" -ForegroundColor Green }
    else { Write-Host "  FAIL $msg" -ForegroundColor Red; $script:ok = $false }
}

Write-Host ""
Write-Host "OpenRollback pre-push checklist"
Write-Host "==============================="

Check (Test-Path "LICENSE") "LICENSE exists"
Check (Test-Path "README.md") "README.md exists"
Check (Test-Path ".gitignore") ".gitignore exists"
Check (-not (Test-Path "node_modules\.git")) "node_modules not tracked by git (N/A if no git yet)"

$nsis = "src-tauri\target\release\bundle\nsis\OpenRollback_0.1.0_x64-setup.exe"
$msi = "src-tauri\target\release\bundle\msi\OpenRollback_0.1.0_x64_en-US.msi"
Check (Test-Path $nsis) "NSIS installer built"
Check (Test-Path $msi) "MSI installer built"

$readme = Get-Content "README.md" -Raw
if ($readme -match "YOUR_ORG") {
    Write-Host "  WARN README still contains YOUR_ORG - replace with your GitHub username before/after push" -ForegroundColor Yellow
}

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "  WARN git not found in PATH - install from https://git-scm.com/download/win" -ForegroundColor Yellow
    $ok = $false
} else {
    Check $true "git is installed"
}

Write-Host ""
if ($ok) { Write-Host "Ready for git init + push (see docs/GITHUB_PUBLISH.md)" -ForegroundColor Green }
else { Write-Host "Fix failures above before pushing" -ForegroundColor Red; exit 1 }
