# Builds RSC in release and copies it into build\ as rsc.exe. The Linux and macOS counterpart is build.sh.
#
#   .\build.ps1             build
#   .\build.ps1 -Run        then start it
#   .\build.ps1 -Install    then install it for this user: %LOCALAPPDATA%\Programs\RSC\rsc.exe, and the
#                           RSC shortcut of the Start menu
#   .\build.ps1 -Uninstall  take those away again (nothing is built)
#
# The icon is in rsc.exe, but the Explorer does not draw the big icon of a program that lies on
# \\wsl.localhost: installed on a local disk, RSC shows its logo in the Start menu, the search, the
# taskbar and the Explorer.
#
# Cargo's target directory stays on the Windows side, in %LOCALAPPDATA%\rsc\target:
# compiling inside the WSL share is much slower. ASCII only: Windows PowerShell 5 reads this file in
# the system's code page.

param([switch]$Run, [switch]$Install, [switch]$Uninstall)

$ErrorActionPreference = "Stop"
$installed = Join-Path $env:LOCALAPPDATA "Programs\RSC"
$shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\RSC.lnk"

function Assert-Closed {
    if (Get-Process -Name "rsc" -ErrorAction SilentlyContinue) { Write-Error "RSC est ouvert : fermez-le, puis relancez le script." }
}

if ($Uninstall) {
    Assert-Closed
    Remove-Item $shortcut -ErrorAction SilentlyContinue
    if (Test-Path $installed) { Remove-Item -Recurse $installed }
    Write-Host "RSC retire de $installed et du menu Demarrer"
    exit 0
}

$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA "rsc\target"

cargo build --release --manifest-path (Join-Path $PSScriptRoot "Cargo.toml")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$out = Join-Path $PSScriptRoot "build"
New-Item -ItemType Directory -Force $out | Out-Null
Assert-Closed
$exe = Join-Path $out "rsc.exe"
Copy-Item (Join-Path $env:CARGO_TARGET_DIR "release\rsc.exe") $exe -Force
Write-Host "-> $exe"

if ($Install) {
    New-Item -ItemType Directory -Force $installed | Out-Null
    $exe = Join-Path $installed "rsc.exe"
    Copy-Item (Join-Path $env:CARGO_TARGET_DIR "release\rsc.exe") $exe -Force
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcut)
    $link.TargetPath = $exe
    $link.WorkingDirectory = $installed
    $link.IconLocation = "$exe,0"
    $link.Description = "RSC, Rust Serial Console"
    $link.Save()
    Write-Host "-> $exe"
    Write-Host "-> $shortcut"
}

if ($Run) {
    Start-Process $exe
}
