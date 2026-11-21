# Flashes RSC's test program onto an Arduino Leonardo, or first keeps the program it holds. Windows.
#
#   tests\arduino\flash.ps1 -Port COM10 -Keep <file.hex>    reads the program it holds into a file
#   tests\arduino\flash.ps1 -Port COM10                     compiles and flashes rsc_essai
#
# The port must carry an Arduino (USB vendor 2341): any other is refused. The tools are arduino-cli's,
# in their folder of their own, %LOCALAPPDATA%\rsc-arduino: its program, its core arduino:avr, its
# data, its downloads. The program is copied and compiled in a local temporary folder: Windows' tools
# take badly a path under \\wsl.localhost. ASCII only: Windows PowerShell 5 reads this file in the
# system's code page.

param(
    [Parameter(Mandatory = $true)][string]$Port,
    [string]$Keep,
    [string]$Tools = (Join-Path $env:LOCALAPPDATA "rsc-arduino")
)

$ErrorActionPreference = "Stop"
$cli = Join-Path $Tools "bin\arduino-cli.exe"
$config = Join-Path $Tools "arduino-cli.yaml"
$fqbn = "arduino:avr:leonardo"
$work = Join-Path ([IO.Path]::GetTempPath()) "rsc-essai"

# The USB identity of the device on a port, as "2341:8036", or nothing.
function Usb-Of([string]$name) {
    foreach ($entity in Get-CimInstance Win32_PnPEntity) {
        if ($entity.Name -like "*($name)" -and $entity.PNPDeviceID -match 'VID_([0-9A-F]{4})&PID_([0-9A-F]{4})') {
            return "$($Matches[1]):$($Matches[2])"
        }
    }
    return $null
}

# A native program, its error output with the rest: Windows PowerShell 5 would take each of its
# lines for an error.
function Run([string]$program, [string[]]$arguments) {
    $line = '"' + $program + '" ' + (($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' ')
    cmd /c "$line 2>&1" | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -ne 0) { throw "Failed ($LASTEXITCODE): $program" }
}

$usb = Usb-Of $Port
if ($null -eq $usb -or -not $usb.StartsWith("2341:")) { throw "$Port carries no Arduino (USB '$usb'): refused." }
Write-Host "$Port : an Arduino, USB $usb"
New-Item -ItemType Directory -Force $work | Out-Null

if ($Keep) {
    # The Leonardo's bootloader starts when its port is opened at 1200 bauds and closed, on a port of
    # its own (USB 2341:0036), for 8 seconds.
    $serial = New-Object System.IO.Ports.SerialPort $Port, 1200
    $serial.Open()
    $serial.Close()
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $boot = $null
    while ($null -eq $boot -and $clock.Elapsed.TotalSeconds -lt 10) {
        Start-Sleep -Milliseconds 200
        foreach ($entity in Get-CimInstance Win32_PnPEntity) {
            if ($entity.PNPDeviceID -match 'VID_2341&PID_0036' -and $entity.Name -match '\((COM\d+)\)') { $boot = $Matches[1] }
        }
    }
    if ($null -eq $boot) { throw "No bootloader port within 10 s." }
    Write-Host "bootloader on $boot"
    $avrdude = Get-ChildItem (Join-Path $Tools "data\packages\arduino\tools\avrdude") -Recurse -Filter "avrdude.exe" | Select-Object -First 1
    $conf = Get-ChildItem (Join-Path $Tools "data\packages\arduino\tools\avrdude") -Recurse -Filter "avrdude.conf" | Select-Object -First 1
    $local = Join-Path $work "kept.hex"
    Remove-Item $local -ErrorAction SilentlyContinue
    Run $avrdude.FullName @("-C", $conf.FullName, "-p", "atmega32u4", "-c", "avr109", "-P", $boot, "-b", "57600", "-D", "-U", "flash:r:${local}:i")
    Copy-Item $local $Keep -Force
    Write-Host ("kept: {0} ({1} bytes)" -f $Keep, (Get-Item $Keep).Length)
    return
}

$sketch = Join-Path $work "rsc_essai"
$build = Join-Path $work "build"
New-Item -ItemType Directory -Force $sketch | Out-Null
Copy-Item (Join-Path $PSScriptRoot "rsc_essai\*") $sketch -Force
Run $cli @("--config-file", $config, "compile", "--fqbn", $fqbn, "--build-path", $build, $sketch)
Run $cli @("--config-file", $config, "upload", "-p", $Port, "--fqbn", $fqbn, "--input-dir", $build, $sketch)
Write-Host "rsc_essai flashed on $Port"
