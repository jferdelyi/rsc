# Measures RSC's start and rest on the release program, build\rsc.exe (.\build.ps1 first).
#
#   .\measure.ps1                     writes start.csv and rest.csv into build\measures
#   .\measure.ps1 -Out <folder>       elsewhere
#   .\measure.ps1 -Tabs 4             RSC with 4 tabs on no port (1 by default)
#   .\measure.ps1 -Exe <rsc.exe>      another program than build\rsc.exe, an earlier one say
#   .\measure.ps1 -Terminal           its tabs in the terminal mode
#
# Start: from Start-Process to RSC's window shown, polled every millisecond. Not the first window of
# the process: winit shows a 14 x 14 helper window first, some 40 ms before RSC's (17/11/2026); the
# window counted is the one titled "RSC".
# Rest: once the window is shown and Settle seconds have passed, the processor time RSC uses over
# Window seconds, and its memory at the end. The first run of each series is a warm-up, kept in the
# CSV with warmup=1 and left out of the statistics.
#
# Each run uses a settings file of its own (RSC_SETTINGS) in a temporary folder: the user's settings
# are not read. eframe writes the window's size and place to app.ron in the user's data folder
# every 30 s: it is copied first and put back at the end if a run changed it.
#
# CSV: semicolons and decimal commas, as Excel reads them in French. ASCII only: Windows PowerShell 5
# reads this file in the system's code page.

param(
    [string]$Out = (Join-Path $PSScriptRoot "build\measures"),
    [int]$Starts = 15,
    [int]$Rests = 10,
    [int]$Settle = 10,
    [int]$Window = 30,
    [int]$Tabs = 1,
    [string]$Exe = (Join-Path $PSScriptRoot "build\rsc.exe"),
    [switch]$Terminal
)

$ErrorActionPreference = "Stop"
$exe = $Exe
if (-not (Test-Path $exe)) { throw "No $exe : run .\build.ps1 first." }
if (Get-Process -Name "rsc" -ErrorAction SilentlyContinue) { throw "RSC is running: close it first, it would be measured too." }

Add-Type -Namespace Measure -Name Window -MemberDefinition '[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, System.Text.StringBuilder text, int max);'
function Title([IntPtr]$hwnd) {
    $text = New-Object System.Text.StringBuilder 64
    [Measure.Window]::GetWindowText($hwnd, $text, 64) | Out-Null
    $text.ToString()
}

$fr = [Globalization.CultureInfo]::GetCultureInfo("fr-FR")
function Number([double]$value, [string]$format) { $value.ToString($format, $fr) }

New-Item -ItemType Directory -Force $Out | Out-Null
$work = Join-Path ([IO.Path]::GetTempPath()) "rsc-measure"
New-Item -ItemType Directory -Force $work | Out-Null

$appRon = Join-Path $env:APPDATA "rsc\data\app.ron"
$appRonKept = Join-Path $work "app.ron"
$hadAppRon = Test-Path $appRon
if ($hadAppRon) { Copy-Item $appRon $appRonKept -Force }

# Starts RSC on a settings file of its own; returns the process and the milliseconds to its window.
# The error output redirected: started without the shell, so without Windows' warning for a program
# on a network path.
function Start-Rsc([string]$name) {
    $env:RSC_SETTINGS = Join-Path $work "$name.json"
    Remove-Item $env:RSC_SETTINGS -ErrorAction SilentlyContinue
    # Several tabs, each on no port, in the terminal mode or not: what the settings file holds for them.
    if ($Tabs -gt 1 -or $Terminal) {
        $general = if ($Terminal) { '{"terminal":true}' } else { '{}' }
        '{"general":' + $general + ',"tabs":[' + ((1..$Tabs | ForEach-Object { '{}' }) -join ',') + ']}' | Set-Content -Encoding ascii $env:RSC_SETTINGS
    }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $exe -RedirectStandardError (Join-Path $work "$name.err") -PassThru
    while ($true) {
        $process.Refresh()
        if ($process.MainWindowHandle -ne [IntPtr]::Zero -and (Title $process.MainWindowHandle) -eq "RSC") { break }
        if ($process.HasExited) { throw "RSC ended before showing its window (run $name)." }
        if ($clock.Elapsed.TotalSeconds -gt 30) { Stop-Process -Id $process.Id -Force; throw "No window within 30 s (run $name)." }
        [Threading.Thread]::Sleep(1)
    }
    $clock.Stop()
    return @($process, $clock.Elapsed.TotalMilliseconds)
}

function Stop-Rsc($process) {
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit(10000) | Out-Null
    Start-Sleep -Milliseconds 500
}

try {
    $lines = @("run;warmup;ms")
    for ($run = 0; $run -le $Starts; $run++) {
        $process, $ms = Start-Rsc "start-$run"
        Stop-Rsc $process
        $lines += "$run;$([int]($run -eq 0));$(Number $ms '0.0')"
        Write-Host ("start {0}/{1}: {2:0.0} ms" -f $run, $Starts, $ms)
    }
    $lines | Set-Content -Encoding ascii (Join-Path $Out "start.csv")

    $lines = @("run;warmup;cpu_ms_per_s;private_mb;working_set_mb")
    for ($run = 0; $run -le $Rests; $run++) {
        $process, $ms = Start-Rsc "rest-$run"
        Start-Sleep -Seconds $Settle
        $process.Refresh()
        $before = $process.TotalProcessorTime.TotalMilliseconds
        $clock = [Diagnostics.Stopwatch]::StartNew()
        Start-Sleep -Seconds $Window
        $process.Refresh()
        $cpu = ($process.TotalProcessorTime.TotalMilliseconds - $before) / $clock.Elapsed.TotalSeconds
        $private = $process.PrivateMemorySize64 / 1MB
        $working = $process.WorkingSet64 / 1MB
        Stop-Rsc $process
        $lines += "$run;$([int]($run -eq 0));$(Number $cpu '0.00');$(Number $private '0.0');$(Number $working '0.0')"
        Write-Host ("rest {0}/{1}: {2:0.00} ms of processor per s, {3:0.0} MB private, {4:0.0} MB working set" -f $run, $Rests, $cpu, $private, $working)
    }
    $lines | Set-Content -Encoding ascii (Join-Path $Out "rest.csv")
}
finally {
    Remove-Item Env:\RSC_SETTINGS -ErrorAction SilentlyContinue
    if ($hadAppRon -and ((Get-FileHash $appRon).Hash -ne (Get-FileHash $appRonKept).Hash)) {
        Copy-Item $appRonKept $appRon -Force
        Write-Host "app.ron changed by the runs: put back as it was."
    }
}
Write-Host "-> $Out"
