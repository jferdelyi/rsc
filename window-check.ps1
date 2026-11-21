# Walks RSC's real window (the program as built, drawn by glow) at its least size, then through the
# "Profiles" and "Devices" sections of the settings, the menu of the profile's pill, the "About"
# section and the terminal mode, as a person would, through the accessibility of Windows (UI
# Automation): the mouse does not move, and each button is found by its name. The window is
# captured at each step into build\captures. Windows only.
#
#   .\window-check.ps1 -Port COM29                 light, then dark
#   .\window-check.ps1 -Port COM29 -Theme dark     one theme
#
# The port must carry a USB device without a name: the pill gives the tab a profile, then the
# device is named and keeps it. The port is listed, never opened. RSC runs on a settings file of its
# own: a tab on that port, a profile "GPS 9600" in the library, and a named device that is not
# plugged in, "Balise GPS", given it. The user's settings are not read.
# eframe writes app.ron in the user's data folder: it is copied first and put back if a run changed
# it. The window to save the licence notices is opened, read (its file name, and the kind of file it
# offers, which must be *.txt; not captured: it shows the user's folders), and cancelled: nothing is
# written.
#
# ASCII only: Windows PowerShell 5 reads this file in the system's code page. The names of the
# buttons, in French, are made of character codes.

param(
    [Parameter(Mandatory = $true)][string]$Port,
    [string[]]$Theme = @("light", "dark"),
    [string]$Exe = (Join-Path $PSScriptRoot "build\rsc.exe"),
    [string]$Out = (Join-Path $PSScriptRoot "build\captures")
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path $Exe)) { throw "No $Exe : run .\build.ps1 first." }
if (Get-Process -Name "rsc" -ErrorAction SilentlyContinue) { throw "RSC is running: close it first, app.ron is shared." }

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type -Namespace Check -Name Win -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, System.Text.StringBuilder text, int max);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr hwnd, uint msg, IntPtr wparam, System.Text.StringBuilder lparam);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
[DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
[DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
'@
# The window's real pixels, whatever the display's scale.
[Check.Win]::SetProcessDPIAware() | Out-Null

$Element = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]

$eAcute = [char]0x00E9
$aGrave = [char]0x00C0
$middleDot = [char]0x00B7
$ellipsis = [char]0x2026
$general = "Profil g${eAcute}n${eAcute}ral"
$library = "GPS 9600"
$profiles = "Profils"
$devices = "Appareils"
$makeProfile = "Cr${eAcute}er le profil"
$nameDevice = "Nommer cet appareil"
$nameDeviceMenu = "Nommer cet appareil${ellipsis}"
$settingsButton = "R${eAcute}glages"
$closeSettings = "Fermer"
$terminal = "Terminal"
# The line that says the terminal mode's shortcuts, by its ASCII start.
$shortcutsStart = "Le clavier va "
$about = "${aGrave} propos"
# The component whose licence is shown: its name and version as the record of licences gives them.
$serialport = "serialport 4.10.1 ${middleDot} MPL-2.0"
$save = "Enregistrer les mentions${ellipsis}"
$dialogTitle = "Enregistrer les mentions de licences"
$cancel = "Annuler"

function Title([IntPtr]$hwnd) {
    $text = New-Object System.Text.StringBuilder 64
    [Check.Win]::GetWindowText($hwnd, $text, 64) | Out-Null
    $text.ToString()
}

# The first element under $root whose condition holds, waited for.
function Find-Element($root, $condition, [string]$what, [int]$seconds = 10) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    do {
        $found = $root.FindFirst($Scope::Descendants, $condition)
        if ($null -ne $found) { return $found }
        Start-Sleep -Milliseconds 200
    } while ($clock.Elapsed.TotalSeconds -lt $seconds)
    throw "Not found within $seconds s: $what"
}

function Find-Named($root, [string]$name) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::NameProperty, $name
    Find-Element $root $condition "'$name'"
}

# Any element whose name starts with $start.
function Find-Starting($root, [string]$start) {
    $clock = [Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($element in $root.FindAll($Scope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
            if ($element.Current.Name.StartsWith($start)) { return $element }
        }
        Start-Sleep -Milliseconds 200
    } while ($clock.Elapsed.TotalSeconds -lt 10)
    throw "Nothing named '$start...' within 10 s"
}

# A button whose name starts with $start, other than $but.
function Find-Button-Starting($root, [string]$start, [string]$but) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::ControlTypeProperty, ([System.Windows.Automation.ControlType]::Button)
    $clock = [Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($button in $root.FindAll($Scope::Descendants, $condition)) {
            $name = $button.Current.Name
            if ($name.StartsWith($start) -and $name -ne $but) { return $button }
        }
        Start-Sleep -Milliseconds 200
    } while ($clock.Elapsed.TotalSeconds -lt 10)
    throw "No button starting with '$start' within 10 s"
}

# What a field of a window to save shows, found by its automation id: the file name typed, or the
# kind of file chosen. The accessibility gives these fields only their label ("Type :"); each is a
# native window of Windows (an edit box, a list box), whose text a message reads (WM_GETTEXT).
# Found on a window to save opened by PowerShell itself, without RSC (18/11/2026).
function Read-Field($dialog, [string]$id) {
    $condition = New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::AutomationIdProperty, $id
    $field = $dialog.FindFirst($Scope::Descendants, $condition)
    if ($null -eq $field) { return "(no field $id)" }
    $hwnd = $field.Current.NativeWindowHandle
    if ($hwnd -eq 0) { return "(no native window, only its label: '$($field.Current.Name)')" }
    $text = New-Object System.Text.StringBuilder 256
    [Check.Win]::SendMessage([IntPtr]$hwnd, 0x000D, [IntPtr]256, $text) | Out-Null
    $text.ToString()
}

# A click asked through the accessibility: egui counts it as a click of the button. A button is
# invoked; what egui shows as chosen or not (a section of the settings, a component in a list),
# AccessKit gives as a toggle, which clicks it the same.
function Click($element) {
    $name = $element.Current.Name
    $pattern = $null
    if ($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        $pattern.Invoke()
    } elseif ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
        $pattern.Toggle()
    } else {
        throw "Cannot be clicked: '$name'"
    }
    Write-Host "  clicked: $name"
    # A frame or two, and the first pass of a menu, which only measures it.
    Start-Sleep -Milliseconds 800
}

# The window as drawn, even under another one.
function Capture([IntPtr]$hwnd, [string]$file) {
    $rect = New-Object Check.Win+RECT
    [Check.Win]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
    $bitmap = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Check.Win]::PrintWindow($hwnd, $hdc, 2)
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $bitmap.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    if (-not $ok) { throw "PrintWindow failed for $file" }
    Write-Host "  -> $file"
}

$work = Join-Path ([IO.Path]::GetTempPath()) "rsc-window-check"
New-Item -ItemType Directory -Force $work | Out-Null
New-Item -ItemType Directory -Force $Out | Out-Null
$appRon = Join-Path $env:APPDATA "rsc\data\app.ron"
$appRonKept = Join-Path $work "app.ron"
$hadAppRon = Test-Path $appRon
if ($hadAppRon) { Copy-Item $appRon $appRonKept -Force }

try {
    foreach ($look in $Theme) {
        Write-Host "== $look"
        $settings = Join-Path $work "settings-$look.json"
        # As RSC writes it: "general" always, even empty.
        ('{"general":{},"theme":"' + $look + '","tabs":[{"port":"' + $Port + '"}],"profiles":[{"name":"' + $library + '","values":{"baud_rate":9600}}],"devices":[{"name":"Balise GPS","usb":{"vid":1659,"pid":8963},"last_port":"COM7","profile":"' + $library + '"}]}') | Set-Content -Encoding ascii $settings
        $env:RSC_SETTINGS = $settings
        # The error output redirected: started without the shell, so without Windows' warning for a
        # program on a network path.
        $process = Start-Process -FilePath $Exe -RedirectStandardError (Join-Path $work "err-$look.txt") -PassThru
        try {
            $clock = [Diagnostics.Stopwatch]::StartNew()
            while ($true) {
                $process.Refresh()
                if ($process.MainWindowHandle -ne [IntPtr]::Zero -and (Title $process.MainWindowHandle) -eq "RSC") { break }
                if ($process.HasExited -or $clock.Elapsed.TotalSeconds -gt 30) { throw "No window of RSC within 30 s" }
                Start-Sleep -Milliseconds 50
            }
            $hwnd = $process.MainWindowHandle
            # The ports listed, the first frames drawn.
            Start-Sleep -Seconds 3
            $window = $Element::FromHandle($hwnd)
            $shot = { param($step) Capture $hwnd (Join-Path $Out "vraie-$look-$step.png") }

            & $shot "1-fenetre"
            # The window at RSC's least size (MIN_SIZE in src/ui/mod.rs: 900 x 430 points), as a person
            # makes it by dragging its border to the end. Set here by its size: SetWindowPos does not
            # ask the window its least size, and made it 0 x 0 when asked for 1 x 1 (19/11/2026). The
            # controls from the top to the bottom must stand in it, from the tab to the actions; then
            # the window is put back at its size.
            $before = New-Object Check.Win+RECT
            [Check.Win]::GetWindowRect($hwnd, [ref]$before) | Out-Null
            $client = New-Object Check.Win+RECT
            [Check.Win]::GetClientRect($hwnd, [ref]$client) | Out-Null
            $scale = [Check.Win]::GetDpiForWindow($hwnd) / 96.0
            $width = ($before.Right - $before.Left) - $client.Right + [int][math]::Ceiling(900 * $scale)
            $height = ($before.Bottom - $before.Top) - $client.Bottom + [int][math]::Ceiling(430 * $scale)
            # SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE
            [Check.Win]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, $width, $height, 0x0016) | Out-Null
            Start-Sleep -Seconds 1
            [Check.Win]::GetClientRect($hwnd, [ref]$client) | Out-Null
            $corner = New-Object Check.Win+POINT
            [Check.Win]::ClientToScreen($hwnd, [ref]$corner) | Out-Null
            Write-Host ("  least size: {0} x {1} pixels at {2} %, {3:N1} x {4:N1} points" -f $client.Right, $client.Bottom, [math]::Round($scale * 100), ($client.Right / $scale), ($client.Bottom / $scale))
            $inside = New-Object System.Windows.Rect $corner.X, $corner.Y, $client.Right, $client.Bottom
            $inside.Inflate(1, 1)
            foreach ($name in @($Port, "Connecter", "Ligne", $settingsButton)) {
                $box = (Find-Named $window $name).Current.BoundingRectangle
                Write-Host ("  '{0}' from {1:N0},{2:N0} to {3:N0},{4:N0}" -f $name, ($box.Left - $corner.X), ($box.Top - $corner.Y), ($box.Right - $corner.X), ($box.Bottom - $corner.Y))
                if (-not $inside.Contains($box)) { throw "'$name' goes out of the window at its least size" }
            }
            & $shot "1a-taille-minimale"
            # SWP_NOZORDER | SWP_NOACTIVATE
            [Check.Win]::SetWindowPos($hwnd, [IntPtr]::Zero, $before.Left, $before.Top, $before.Right - $before.Left, $before.Bottom - $before.Top, 0x0014) | Out-Null
            Start-Sleep -Seconds 1
            # The settings on "Profiles", then on "Devices", while the device on the tab's port has no
            # name: the buttons that make a profile and name a device end before the window does,
            # where "Close" ends.
            Click (Find-Named $window $settingsButton)
            $closeRight = (Find-Named $window $closeSettings).Current.BoundingRectangle.Right
            foreach ($section in @(@($profiles, $makeProfile, "1b-profils"), @($devices, $nameDevice, "1c-appareils"))) {
                Click (Find-Named $window $section[0])
                $right = (Find-Named $window $section[1]).Current.BoundingRectangle.Right
                Write-Host ("  '{0}' ends at {1}, '{2}' at {3}" -f $section[1], $right, $closeSettings, $closeRight)
                if ($right -gt $closeRight + 1) { throw "The button '$($section[1])' goes past the window" }
                & $shot $section[2]
            }
            Click (Find-Named $window $closeSettings)
            # The pill on the bare USB port: a profile of the library, which the tab keeps.
            Click (Find-Named $window $general)
            & $shot "2-pastille-port"
            Click (Find-Named $window $library)
            $pill = Find-Named $window "Profil $library"
            & $shot "3-profil-choisi"
            # The device named from the pill's menu: it keeps the profile the tab used.
            Click $pill
            Click (Find-Named $window $nameDeviceMenu)
            Click (Find-Named $window $nameDevice)
            & $shot "4-appareil-nomme"
            Click (Find-Named $window $closeSettings)
            $pill = Find-Named $window "Profil $library"
            Click $pill
            & $shot "5-pastille-appareil"
            # The menu closed again, before the settings.
            Click $pill
            Click (Find-Named $window $settingsButton)
            Click (Find-Named $window $about)
            & $shot "6-a-propos"
            Click (Find-Named $window $serialport)
            & $shot "7-licence"

            # The window to save the notices is modal: RSC waits for it to close.
            Click (Find-Named $window $save)
            $dialogCondition = New-Object System.Windows.Automation.AndCondition -ArgumentList `
                (New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::ProcessIdProperty, $process.Id), `
                (New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::NameProperty, $dialogTitle)
            $dialog = Find-Element $Element::RootElement $dialogCondition "the window '$dialogTitle'"
            # Not captured: the window shows the user's folders. Its file name and the kind of file it
            # offers, read; the notices are text.
            $kind = Read-Field $dialog "FileTypeControlHost"
            Write-Host ("  save window: file name '{0}', kind '{1}'" -f (Read-Field $dialog "FileNameControlHost"), $kind)
            if (-not $kind.Contains("*.txt")) { throw "The window to save the notices does not offer *.txt: '$kind'" }
            # Its "Cancel" button, among the buttons (the first element of that name is not one);
            # else the window closed as by its cross.
            $cancelCondition = New-Object System.Windows.Automation.AndCondition -ArgumentList `
                (New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::NameProperty, $cancel), `
                (New-Object System.Windows.Automation.PropertyCondition -ArgumentList $Element::ControlTypeProperty, ([System.Windows.Automation.ControlType]::Button))
            $cancelButton = $dialog.FindFirst($Scope::Descendants, $cancelCondition)
            if ($null -ne $cancelButton) {
                Click $cancelButton
            } else {
                $dialog.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
                Write-Host "  closed: $dialogTitle"
                Start-Sleep -Milliseconds 800
            }
            & $shot "9-apres-annulation"
            # The terminal mode: its switch, then the line of its shortcuts where the send line was.
            Click (Find-Named $window $closeSettings)
            Click (Find-Named $window $terminal)
            $shortcuts = (Find-Starting $window $shortcutsStart).Current.Name
            Write-Host ("  shortcuts: '{0}'" -f $shortcuts)
            if (-not ($shortcuts.Contains("Ctrl+C") -and $shortcuts.Contains("Ctrl+V"))) { throw "The shortcuts are not said" }
            & $shot "10-terminal"
        }
        finally {
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force
                $process.WaitForExit(10000) | Out-Null
            }
        }
        # The device named, as RSC wrote it: with the profile its tab used.
        $written = Get-Content -Raw -Encoding UTF8 $settings | ConvertFrom-Json
        $named = @($written.devices | Where-Object { $_.name -ne "Balise GPS" })
        foreach ($made in $named) {
            Write-Host ("  device named: '{0}', USB {1:X4}:{2:X4}, serial {3}, last on {4}, profile '{5}'" -f $made.name, $made.usb.vid, $made.usb.pid, $made.usb.serial, $made.last_port, $made.profile)
        }
        if ($named.Count -ne 1 -or $named[0].profile -ne $library) { throw "The device named does not keep the profile '$library'" }
    }
}
finally {
    Remove-Item Env:\RSC_SETTINGS -ErrorAction SilentlyContinue
    if ($hadAppRon -and ((Get-FileHash $appRon).Hash -ne (Get-FileHash $appRonKept).Hash)) {
        Copy-Item $appRonKept $appRon -Force
        Write-Host "app.ron changed by the runs: put back as it was."
    }
}
