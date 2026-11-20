<#
.SYNOPSIS
    Copies the release build to dist\ and creates a desktop shortcut.

.DESCRIPTION
    Called by `mise run install` after the release build. The shortcut
    "API Client" always points to dist\API Client.exe, so after every run it
    starts the newest version. A copy is used instead of target\release so
    that a running app does not block the next build. A running app from
    dist\ is closed first and started again afterwards.
#>
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

$name = "API Client"
$built = Join-Path $root "target\release\api-client.exe"
$dist = Join-Path $root "dist"
$target = Join-Path $dist "$name.exe"

if (-not (Test-Path $built)) { throw "Release build missing: $built (run 'mise run release')" }
New-Item -ItemType Directory -Force $dist | Out-Null

# Close a running copy from dist\, otherwise the file is locked.
$wasRunning = $false
Get-Process | Where-Object { $_.Path -and $_.Path.StartsWith($dist, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object {
    Write-Host "Closing running app (PID $($_.Id))" -ForegroundColor Yellow
    $script:wasRunning = $true
    if (-not $_.CloseMainWindow() -or -not $_.WaitForExit(5000)) { $_ | Stop-Process -Force }
    $_.WaitForExit(5000) | Out-Null
}

Copy-Item $built $target -Force

$link = Join-Path ([Environment]::GetFolderPath("Desktop")) "$name.lnk"
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($link)
$shortcut.TargetPath = $target
$shortcut.WorkingDirectory = $dist
$shortcut.IconLocation = "$target,0"
$shortcut.Description = "Open-source API client"
$shortcut.Save()

$sizeMb = [math]::Round((Get-Item $target).Length / 1MB, 1)
Write-Host "Shortcut updated: $link" -ForegroundColor Green
Write-Host "Program: $target ($sizeMb MB)" -ForegroundColor Green
if ($wasRunning) {
    Start-Process $target -WorkingDirectory $dist
    Write-Host "App restarted" -ForegroundColor Green
}
