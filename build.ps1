<#
.SYNOPSIS
    Runs all checks of the workspace in one go.

.DESCRIPTION
    Steps: check formatting, Clippy (warnings are errors), tests, optionally
    test coverage. Run it before every commit.

    The desktop app (Tauri frontend and bundle) joins this script once it
    exists (milestone M1).

.PARAMETER Coverage
    Also measure test coverage (coverage.ps1, at least 90 %).

.PARAMETER Release
    Also build the release version.

.EXAMPLE
    .\build.ps1 -Coverage
#>
param([switch]$Coverage, [switch]$Release)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Step($name, [scriptblock]$cmd) {
    Write-Host "==> $name" -ForegroundColor Cyan
    & $cmd
    if ($LASTEXITCODE -ne 0) { throw "$name failed (exit code $LASTEXITCODE)" }
}

Step "Check formatting" { cargo fmt --all -- --check }
Step "Clippy"           { cargo clippy --workspace --all-targets --locked -- -D warnings }
Step "Tests"            { cargo test --workspace --locked }
if ($Coverage) { & (Join-Path $PSScriptRoot "coverage.ps1") }
if ($Release)  { Step "Release build" { cargo build --workspace --release --locked } }

Write-Host "All checks passed" -ForegroundColor Green
