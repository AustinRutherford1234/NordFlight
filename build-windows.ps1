$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }
New-Item -ItemType Directory -Force -Path "dist" | Out-Null
Copy-Item "target/release/gripen-flight.exe" "dist/NordFlight.exe" -Force
New-Item -ItemType Directory -Force -Path "dist/assets" | Out-Null
Copy-Item "assets/*" "dist/assets/" -Recurse -Force
Write-Host "Built dist/NordFlight.exe. Double-click it to play."
