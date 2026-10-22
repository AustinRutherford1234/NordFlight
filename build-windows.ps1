param(
    [switch]$Install,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

if (-not $SkipBuild) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Install stable Rust (MSVC) before building NordFlight.'
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($visualStudio) {
            $devCommand = '"{0}\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 && set' -f $visualStudio
            cmd.exe /d /s /c $devCommand | ForEach-Object {
                if ($_ -match '^([^=]+)=(.*)$') {
                    [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
                }
            }
            if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the Microsoft C++ build tools.' }
        }
    }
    if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '4' }
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
}

if (-not (Test-Path 'target/release/gripen-flight.exe')) {
    throw 'No compiled game found. Run this script without -SkipBuild first.'
}
New-Item -ItemType Directory -Force -Path 'dist/assets' | Out-Null
Copy-Item 'target/release/gripen-flight.exe' 'dist/NordFlight.exe' -Force
Copy-Item 'assets/*' 'dist/assets/' -Recurse -Force
Copy-Item 'README.md', 'docs' 'dist/' -Recurse -Force
& "$PSScriptRoot/tools/New-WindowsIcon.ps1" -SourceImage "$PSScriptRoot/assets/branding/nord-flight-icon.png" -DestinationIcon "$PSScriptRoot/dist/NordFlight.ico"
$version = (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$revision = & git rev-parse --short HEAD
if ($LASTEXITCODE -ne 0) { throw 'Could not read the Git revision.' }
Set-Content 'dist/build-version.txt' -Value "Version: $version`nGit commit: $revision`nBuilt: $([DateTimeOffset]::Now.ToString('o'))"

function Find-InnoCompiler {
    if ($env:ISCC_PATH -and (Test-Path -LiteralPath $env:ISCC_PATH)) { return $env:ISCC_PATH }
    $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($compiler) { return $compiler.Source }
    foreach ($base in @("$env:LOCALAPPDATA\Programs", ${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        $candidate = Join-Path $base 'Inno Setup 6\ISCC.exe'
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
}
$iscc = Find-InnoCompiler
if (-not $iscc) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw 'Install Inno Setup 6, or set ISCC_PATH to its ISCC.exe compiler.'
    }
    winget install --id JRSoftware.InnoSetup --exact --source winget --scope user --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup installation failed.' }
    $iscc = Find-InnoCompiler
    if (-not $iscc) { throw 'Inno Setup compiler was not found after installation.' }
}
& $iscc /Q "/DSourceDir=$PSScriptRoot\dist" "/DAppVersion=$version" "/O$PSScriptRoot\dist\installer" "$PSScriptRoot\tools\windows-installer.iss"
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
$installer = Join-Path $PSScriptRoot 'dist\installer\NordFlight-Setup.exe'
Write-Host "Installer ready: $installer"

if ($Install) {
    $installedExe = Join-Path $env:LOCALAPPDATA 'Programs\NordFlight\NordFlight.exe'
    if (Get-Process NordFlight -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installedExe }) {
        throw 'Close NordFlight, then run dist\installer\NordFlight-Setup.exe to finish the update.'
    }
    $setup = Start-Process -FilePath $installer -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOCLOSEAPPLICATIONS', '/NORESTARTAPPLICATIONS', '/SP-' -WindowStyle Hidden -Wait -PassThru
    if ($setup.ExitCode -ne 0) { throw "Installer failed (exit code $($setup.ExitCode))." }
    $builtHash = (Get-FileHash 'dist/NordFlight.exe' -Algorithm SHA256).Hash
    if ((Get-FileHash $installedExe -Algorithm SHA256).Hash -ne $builtHash) {
        throw 'The installed executable does not match the build.'
    }
    $shortcutShell = New-Object -ComObject WScript.Shell
    $programs = Join-Path ([Environment]::GetFolderPath('Programs')) 'NordFlight'
    New-Item -ItemType Directory -Force -Path $programs | Out-Null
    foreach ($folder in @([Environment]::GetFolderPath('Desktop'), $programs)) {
        $shortcut = $shortcutShell.CreateShortcut((Join-Path $folder 'Update NordFlight.lnk'))
        $shortcut.TargetPath = Join-Path $PSScriptRoot 'Update NordFlight.cmd'
        $shortcut.WorkingDirectory = $PSScriptRoot
        $shortcut.Description = 'Pull, rebuild, package, and install NordFlight from this checkout'
        $shortcut.IconLocation = Join-Path (Split-Path $installedExe) 'NordFlight.ico'
        $shortcut.Save()
    }
    Write-Host "NordFlight updated: $installedExe"
}
