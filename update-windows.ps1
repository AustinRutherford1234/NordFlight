param([switch]$SkipPull)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (-not $SkipPull) {
    git pull --ff-only
    if ($LASTEXITCODE -ne 0) { throw 'Git pull failed. Your installed game has not been changed.' }
}
& "$PSScriptRoot\build-windows.ps1" -Install
