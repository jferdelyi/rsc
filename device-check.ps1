# RSC's real window on a real Arduino running RSC's test program (tests/arduino/rsc_essai), driven
# by Windows' real keyboard: the keys go through the system and winit to RSC, and from RSC to the
# board. The one path the tests' harness does not take. Windows only.
#
#   .\device-check.ps1 -Port COM10            the console, its size, a menu, colours, measures
#   .\device-check.ps1 -Port COM10 -Curves    the measures as curves: two charts, time then number;
#                                             then what a screen reader gets (see below)
#
# With -Curves, what a screen reader gets is read through UI Automation, as the Narrator reads it:
# the rows of the terminal's screen, the state line (a status), the lines of the console once the
# terminal mode is off; then the port is closed while the live regions announced by Windows are
# listened to: the state line must be heard. The descriptions (FullDescription) and the live setting
# are not read: Windows PowerShell's UI Automation does not support them.
#
# The port must carry an Arduino (USB vendor 2341): any other is refused. RSC runs on a settings file
# of its own: a tab on that port, in the terminal mode; with -Curves, the curves on, pression in the
# chart below. It is connected through the accessibility of Windows; then the keys are typed with
# RSC's window in front, checked before each key: if another window took the front, the run stops.
# Do not type meanwhile (some 20 s). The window is captured at each step into
# build\captures\appareil-*.png. eframe's app.ron is put back as it was.
#
# ASCII only: Windows PowerShell 5 reads this file in the system's code page.

param(
    [Parameter(Mandatory = $true)][string]$Port,
    [switch]$Curves,
    [string]$Exe = (Join-Path $PSScriptRoot "build\rsc.exe"),
    [string]$Out = (Join-Path $PSScriptRoot "build\captures")
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path $Exe)) { throw "No $Exe : run .\build.ps1 first." }
if (Get-Process -Name "rsc" -ErrorAction SilentlyContinue) { throw "RSC is running: close it first, app.ron is shared." }

$usb = $null
foreach ($entity in Get-CimInstance Win32_PnPEntity) {
    if ($entity.Name -like "*($Port)" -and $entity.PNPDeviceID -match 'VID_([0-9A-F]{4})&PID_([0-9A-F]{4})') { $usb = "$($Matches[1]):$($Matches[2])" }
}
if ($null -eq $usb -or -not $usb.StartsWith("2341:")) { throw "$Port carries no Arduino (USB '$usb'): refused." }

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing, System.Windows.Forms
Add-Type -Namespace Device -Name Win -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, System.Text.StringBuilder text, int max);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
[DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hwnd);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr process);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
'@
[Device.Win]::SetProcessDPIAware() | Out-Null
Add-Type -AssemblyName WindowsBase
Add-Type -ReferencedAssemblies ([System.Windows.Automation.AutomationElement].Assembly.Location), ([System.Windows.Automation.AutomationElementIdentifiers].Assembly.Location), ([System.Windows.Rect].Assembly.Location) -TypeDefinition @'
using System.Collections.Concurrent;
using System.Windows.Automation;
namespace Device {
    // The live regions Windows announces under a window, by the name each one has when announced:
    // what the Narrator hears. Heard on UI Automation's own thread, read afterwards.
    public static class Live {
        static readonly ConcurrentQueue<string> heard = new ConcurrentQueue<string>();
        public static void Listen(AutomationElement root) {
            Automation.AddAutomationEventHandler(AutomationElementIdentifiers.LiveRegionChangedEvent, root, TreeScope.Subtree, (sender, e) => {
                AutomationElement element = sender as AutomationElement;
                string name = "?";
                try { if (element != null) { name = element.Current.Name; } } catch (ElementNotAvailableException) { }
                heard.Enqueue(name);
            });
        }
        public static string[] Heard() { return heard.ToArray(); }
    }
}
'@

$Element = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]
$eAcute = [char]0x00E9
$connect = "Connecter"
$disconnect = "D${eAcute}connecter"

function Title([IntPtr]$hwnd) {
    $text = New-Object System.Text.StringBuilder 64
    [Device.Win]::GetWindowText($hwnd, $text, 64) | Out-Null
    $text.ToString()
}

function Find-Named($root, [string]$name) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::NameProperty, $name
    $clock = [Diagnostics.Stopwatch]::StartNew()
    do {
        $found = $root.FindFirst($Scope::Descendants, $condition)
        if ($null -ne $found) { return $found }
        Start-Sleep -Milliseconds 200
    } while ($clock.Elapsed.TotalSeconds -lt 10)
    throw "Not found within 10 s: '$name'"
}

function Find-Type($root, $type) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::ControlTypeProperty, $type
    $found = $root.FindFirst($Scope::Descendants, $condition)
    if ($null -eq $found) { throw "No element of the type $($type.ProgrammaticName)" }
    $found
}

# The text elements under an element, as a screen reader goes through them: their names, in order.
function Lines-Of($element) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::ControlTypeProperty, ([System.Windows.Automation.ControlType]::Text)
    @($element.FindAll($Scope::Descendants, $condition) | ForEach-Object { $_.Current.Name })
}

function Capture([IntPtr]$hwnd, [string]$file) {
    $rect = New-Object Device.Win+RECT
    [Device.Win]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
    $bitmap = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Device.Win]::PrintWindow($hwnd, $hdc, 2)
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $bitmap.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    if (-not $ok) { throw "PrintWindow failed for $file" }
    Write-Host "  -> $file"
}

# RSC's window brought in front. Windows lets a program in the background do it only while its input
# is joined to the one of the window in front. No mouse, no key: if it fails, the run stops there.
function Bring-Front([IntPtr]$hwnd) {
    $front = [Device.Win]::GetForegroundWindow()
    if ($front -eq $hwnd) { return }
    $mine = [Device.Win]::GetCurrentThreadId()
    $theirs = [Device.Win]::GetWindowThreadProcessId($front, [IntPtr]::Zero)
    $joined = $theirs -ne 0 -and $theirs -ne $mine -and [Device.Win]::AttachThreadInput($mine, $theirs, $true)
    try {
        [Device.Win]::BringWindowToTop($hwnd) | Out-Null
        [Device.Win]::SetForegroundWindow($hwnd) | Out-Null
    }
    finally {
        if ($joined) { [Device.Win]::AttachThreadInput($mine, $theirs, $false) | Out-Null }
    }
    Start-Sleep -Milliseconds 500
    if ([Device.Win]::GetForegroundWindow() -ne $hwnd) { throw "RSC's window could not be brought in front: nothing typed." }
}

# A button pressed through the accessibility: Invoke, or Select for a choice among a few.
function Press($element) {
    $pattern = $null
    if ($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) { $pattern.Invoke(); return }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$pattern)) { $pattern.Select(); return }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) { $pattern.Toggle(); return }
    throw "Cannot press '$($element.Current.Name)'"
}

# Keys typed by Windows' keyboard, RSC's window in front or nothing typed at all.
function Type-Keys([IntPtr]$hwnd, [string]$keys, [int]$wait = 800) {
    if ([Device.Win]::GetForegroundWindow() -ne $hwnd) { throw "RSC's window is not in front: nothing typed." }
    [System.Windows.Forms.SendKeys]::SendWait($keys)
    Write-Host "  typed: $keys"
    Start-Sleep -Milliseconds $wait
}

$work = Join-Path ([IO.Path]::GetTempPath()) "rsc-device-check"
New-Item -ItemType Directory -Force $work, $Out | Out-Null
$appRon = Join-Path $env:APPDATA "rsc\data\app.ron"
$appRonKept = Join-Path $work "app.ron"
$hadAppRon = Test-Path $appRon
if ($hadAppRon) { Copy-Item $appRon $appRonKept -Force }

try {
    $settings = Join-Path $work "settings.json"
    $general = if ($Curves) { '{"terminal":true,"plot":true,"plot_below":["pression"]}' } else { '{"terminal":true}' }
    ('{"general":' + $general + ',"tabs":[{"port":"' + $Port + '"}]}') | Set-Content -Encoding ascii $settings
    $env:RSC_SETTINGS = $settings
    $process = Start-Process -FilePath $Exe -RedirectStandardError (Join-Path $work "err.txt") -PassThru
    try {
        $clock = [Diagnostics.Stopwatch]::StartNew()
        while ($true) {
            $process.Refresh()
            if ($process.MainWindowHandle -ne [IntPtr]::Zero -and (Title $process.MainWindowHandle) -eq "RSC") { break }
            if ($process.HasExited -or $clock.Elapsed.TotalSeconds -gt 30) { throw "No window of RSC within 30 s" }
            Start-Sleep -Milliseconds 50
        }
        $hwnd = $process.MainWindowHandle
        Start-Sleep -Seconds 2
        $window = $Element::FromHandle($hwnd)
        $shot = { param($step) Capture $hwnd (Join-Path $Out "appareil-$step.png") }

        $button = Find-Named $window $connect
        $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
        Find-Named $window $disconnect | Out-Null
        Write-Host "  connected to $Port"
        # RSC in front: the keys typed go to it. (The accessibility's SetFocus refuses RSC's window:
        # "the target element cannot receive the focus".)
        Bring-Front $hwnd

        if ($Curves) {
            # temp and compteur above, pression below on its own scale; along the time, then the
            # numbers of the measures.
            Type-Keys $hwnd "{ENTER}"
            Type-Keys $hwnd "mesures{ENTER}" 4000
            & $shot "courbes-1-temps"
            # Ctrl+C without a selection: interrupts the board; the curves stay.
            Type-Keys $hwnd "^c" 1000
            Press (Find-Named $window ("N" + [char]0x00B0))
            Start-Sleep -Milliseconds 800
            & $shot "courbes-2-numero"

            # What a screen reader gets, read as the Narrator reads it.
            $screen = Find-Named $window ([string][char]0x00C9 + "cran du terminal")
            $rows = Lines-Of $screen
            Write-Host "  terminal: $($screen.Current.ControlType.ProgrammaticName), $($rows.Count) row(s) with text"
            $rows | Select-Object -Last 3 | ForEach-Object { Write-Host "    | $_" }
            if ($rows.Count -eq 0) { throw "The terminal's screen gives no row." }
            $status = Find-Type $window ([System.Windows.Automation.ControlType]::StatusBar)
            # Windows PowerShell's UI Automation cannot read the live setting ("property not supported",
            # on any element, 19/11/2026): the announcement heard below is the proof.
            $live = $status.GetCurrentPropertyValue([System.Windows.Automation.AutomationElementIdentifiers]::LiveSettingProperty)
            if ($live -is [Exception]) { $live = "unreadable here" }
            Write-Host "  state line: '$($status.Current.Name)', live setting $live"
            if ("$live" -eq "Off") { throw "The state line is not announced." }
            Press (Find-Named $window "Terminal")
            Start-Sleep -Milliseconds 800
            $console = Find-Named $window "Console"
            $lines = Lines-Of $console
            Write-Host "  console: $($console.Current.ControlType.ProgrammaticName), $($lines.Count) line(s) in view"
            $lines | Select-Object -Last 3 | ForEach-Object { Write-Host "    | $_" }
            if ($lines.Count -eq 0) { throw "The console gives no line." }
            & $shot "courbes-3-console"
            [Device.Live]::Listen($window)
            Press (Find-Named $window $disconnect)
            Find-Named $window $connect | Out-Null
            Start-Sleep -Milliseconds 1500
            $heard = [Device.Live]::Heard()
            Write-Host "  announced: $($heard.Count) time(s)"
            $heard | ForEach-Object { Write-Host "    > $_" }
            if ($heard -notcontains "D${eAcute}connect${eAcute}") { throw "Closing the port did not announce the state line." }
            return
        }

        Type-Keys $hwnd "{ENTER}"
        & $shot "1-invite"
        Type-Keys $hwnd "taille{ENTER}" 1200
        & $shot "2-taille"
        Type-Keys $hwnd "menu{ENTER}" 1200
        & $shot "3-menu"
        Type-Keys $hwnd "{DOWN}"
        & $shot "4-menu-ligne-2"
        Type-Keys $hwnd "{ENTER}"
        & $shot "5-choix"
        Type-Keys $hwnd "couleurs{ENTER}" 1200
        & $shot "6-couleurs"
        Type-Keys $hwnd "mesures{ENTER}" 1500
        # Ctrl+C without a selection: interrupts the board.
        Type-Keys $hwnd "^c" 1000
        & $shot "7-mesures-arretees"
    }
    finally {
        [System.Windows.Automation.Automation]::RemoveAllEventHandlers()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
            $process.WaitForExit(10000) | Out-Null
        }
    }
}
finally {
    Remove-Item Env:\RSC_SETTINGS -ErrorAction SilentlyContinue
    if ($hadAppRon -and ((Get-FileHash $appRon).Hash -ne (Get-FileHash $appRonKept).Hash)) {
        Copy-Item $appRonKept $appRon -Force
        Write-Host "app.ron changed by the run: put back as it was."
    }
}
