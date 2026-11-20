<#
.SYNOPSIS
    Measures test coverage of the workspace and creates an HTML report.

.DESCRIPTION
    Uses cargo-llvm-cov. User interfaces (apps/), program start (main.rs),
    test helpers and the tests themselves do not count: they contain only
    controls and wiring, the logic lives in the crates.

    Fails if line coverage is below -Min percent.
    Report: target\llvm-cov\html\index.html

    Install once:
        rustup component add llvm-tools-preview
        cargo install cargo-llvm-cov --locked

.PARAMETER Min
    Minimum line coverage in percent (default 90).

.PARAMETER Open
    Open the report in the browser afterwards.

.EXAMPLE
    .\coverage.ps1 -Open
#>
param([int]$Min = 90, [switch]$Open)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

cargo llvm-cov --version *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "cargo-llvm-cov is missing. Install it with:" -ForegroundColor Red
    Write-Host "  rustup component add llvm-tools-preview"
    Write-Host "  cargo install cargo-llvm-cov --locked"
    exit 1
}

# Keep in sync with the coverage job in .github/workflows/ci.yml.
$ignore = '[\\/]apps[\\/]|[\\/]ac_testutil[\\/]|main\.rs$|[\\/]tests[\\/]'
Write-Host "==> Tests with coverage" -ForegroundColor Cyan
cargo llvm-cov --workspace --lib --locked --html --ignore-filename-regex $ignore
if ($LASTEXITCODE -ne 0) { throw "Tests failed (exit code $LASTEXITCODE)" }

cargo llvm-cov report --summary-only --ignore-filename-regex $ignore --fail-under-lines $Min
$failed = $LASTEXITCODE -ne 0
$report = Join-Path $PSScriptRoot "target\llvm-cov\html\index.html"
Write-Host "Report: $report" -ForegroundColor Green
if ($Open) { Start-Process $report }
if ($failed) { throw "Line coverage below $Min %" }
