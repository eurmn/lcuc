$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$cargoBin = "$env:USERPROFILE\.cargo\bin"
$env:Path = "$cargoBin;$env:Path"

Write-Host "=== 1. Running rust-analyzer analysis-stats ===" -ForegroundColor Cyan
Set-Location "$scriptDir\src-tauri"
if (Test-Path "$cargoBin\rust-analyzer.exe") {
    & "$cargoBin\rust-analyzer.exe" analysis-stats .
} elseif (Get-Command rust-analyzer -ErrorAction SilentlyContinue) {
    rust-analyzer analysis-stats .
}

Write-Host "=== 2. Running cargo check ===" -ForegroundColor Cyan
& "$cargoBin\cargo.exe" check

Write-Host "=== 3. Running cargo test ===" -ForegroundColor Cyan
& "$cargoBin\cargo.exe" test

Write-Host "=== 4. Running svelte-check ===" -ForegroundColor Cyan
Set-Location "$scriptDir"
bun run check

Write-Host "=== 5. Running frontend build ===" -ForegroundColor Cyan
bun run build

Write-Host "=== All validation checks passed successfully! ===" -ForegroundColor Green
