# Compare justfetch's Windows collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# unsupported, inconclusive, not comparable or not covered. Runs compare.py in
# strict acceptance mode: exit 1 on a mismatch or error, 2 on a setup problem,
# 3 when a module has neither an observation nor a declared exclusion.
#
# The comparison needs Python 3 (scripts/compare.py). Without it the raw
# observations are printed and the script exits 2.
#
# -SelfTest replaces the platform queries with fixed values. It checks the seam
# between this script and compare.py — that the native.json written here is one
# compare.py accepts, that its evidence lists survive serialization, and that
# strict mode then reaches the verdict it should, in both directions. It needs
# no Windows API, so CI runs it on every push; a real Windows run is still the
# only evidence about the collectors themselves.
param([switch]$SelfTest)
$ErrorActionPreference = 'Continue'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path

# A justfetch document and the observations that should match it.
$SelfTestDocument = @'
{"schema_version":1,"environment":{"kind":"native"},
 "gpu":{"status":"available","value":{"devices":[{"name":"Test Adapter","vendor":"Test","id":null,"driver":null,"kind":"discrete"}]},"source":"t","confidence":"confirmed"},
 "battery":{"status":"available","value":{"units":[{"name":"Battery 0","percent":80,"state":"discharging"}],"power":"battery"},"source":"t","confidence":"confirmed"},
 "display":{"status":"available","value":{"displays":[{"name":null,"width":1920,"height":1080,"refresh_hz":60,"primary":true,"kind":"unknown"}],"definition":"t"},"source":"t","confidence":"confirmed"},
 "packages":{"status":"unsupported","reason":"no package manager"},
 "desktop":{"status":"unsupported","reason":"no desktop"},
 "locale":{"status":"available","value":{"process":"en-US","process_source":"GetUserDefaultLocaleName","system":"en-US","system_source":"GetUserDefaultLocaleName"},"source":"t","confidence":"confirmed"}}
'@

function Get-SelfTestObservation {
    [ordered]@{
        tools = [ordered]@{
            gpu = 'self-test'; battery = 'self-test'; display = 'self-test'
        }
        values = [ordered]@{
            gpu = @{ names = @('Test Adapter') }
            battery = @{ percent = 80 }
            display = @('1920x1080')
        }
        # desktop was checked and found absent; locale and packages cannot be
        # compared here. Both lists must survive into the file compare.py reads.
        absent = @('desktop')
        not_applicable = @('locale', 'packages')
    }
}

function Get-WindowsObservation {
    $values = [ordered]@{}
    $tools = [ordered]@{}
    $absent = @()
    $notApplicable = @()

    # A failed CIM query is not evidence of absence: `-ErrorAction SilentlyContinue`
    # returns nothing either way, so the query is attempted explicitly and only a
    # successful empty result may declare the battery absent.
    try {
        $battery = @(Get-CimInstance -ClassName Win32_Battery -ErrorAction Stop) |
            Select-Object -First 1
        if (-not $battery) {
            $absent += 'battery'
        } else {
            $values['battery'] = @{ percent = [int]$battery.EstimatedChargeRemaining }
            $tools['battery'] = 'Win32_Battery'
        }
    } catch {
        Write-Warning "Win32_Battery could not be queried: $_"
    }

    try {
        $adapters = @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction Stop)
    } catch {
        $adapters = @()
        Write-Warning "Win32_VideoController could not be queried: $_"
    }
    if ($adapters.Count -gt 0) {
        # compare.py expects {"names": [...]}: a bare list is rejected as bad input.
        $values['gpu'] = @{ names = @($adapters | ForEach-Object { $_.Name }) }
        $tools['gpu'] = 'Win32_VideoController'
    }

    # justfetch lists monitors (EnumDisplayMonitors), not adapters, so the
    # comparison uses that unit. Win32_VideoController reports one mode per
    # adapter and cannot see two monitors on one GPU.
    #
    # Limits, written down rather than guessed at: `Screen.Bounds` is the
    # monitor's rectangle in physical pixels once the process is DPI aware,
    # which is what GetMonitorInfoW reports; refresh rate is not compared
    # because Screen has no equivalent and justfetch reads it from
    # EnumDisplaySettingsW.
    try {
        Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
        $null = Add-Type -Namespace Jf -Name Dpi -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern bool SetProcessDPIAware();
'@ -ErrorAction Stop
        [Jf.Dpi]::SetProcessDPIAware() | Out-Null
        $screens = @([System.Windows.Forms.Screen]::AllScreens)
        if ($screens.Count -gt 0) {
            $values['display'] = @($screens | ForEach-Object {
                "$($_.Bounds.Width)x$($_.Bounds.Height)" })
            $tools['display'] = 'Screen.AllScreens'
        }
    } catch {
        # Without the WinForms assembly there is no monitor-level API here.
        $notApplicable += 'display'
        Write-Warning "monitor enumeration unavailable: $_"
    }
    # Adapter names come from a different API than justfetch's EnumDisplayDevicesW,
    # so a GPU mismatch here is a wording difference more often than a defect.

    # USERPROFILE is always set on Windows; the guard lets the self-test exercise
    # this function's shape on a host that has no Windows environment.
    if ($env:USERPROFILE) {
        $scoop = Join-Path $env:USERPROFILE 'scoop\apps'
        if (Test-Path $scoop) {
            $values['packages'] = @{ scoop = @(Get-ChildItem $scoop -Directory).Count }
            $tools['packages'] = 'scoop/apps'
        }
    }

    # locale is declared not comparable: justfetch reads GetUserDefaultLocaleName,
    # and comparing it with CultureInfo would compare the same source twice.
    $notApplicable += 'locale'

    # Windows has no desktop environment in justfetch's sense. Evidence: none of
    # the variables a desktop session exports are present.
    $sessionVars = @('XDG_CURRENT_DESKTOP', 'XDG_SESSION_DESKTOP', 'DESKTOP_SESSION', 'WAYLAND_DISPLAY')
    if (-not ($sessionVars | Where-Object { [Environment]::GetEnvironmentVariable($_) })) {
        $absent += 'desktop'
    }

    [ordered]@{
        tools = $tools
        values = $values
        absent = @($absent)
        not_applicable = @($notApplicable)
    }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("jf-windows-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
$exit = 2
try {
    if ($SelfTest) {
        $docJson = $SelfTestDocument
        $observation = Get-SelfTestObservation
    } else {
        $bin = if ($env:BIN) { $env:BIN } else { Join-Path (Get-Location) 'target\release\justfetch.exe' }
        if (-not (Test-Path $bin)) {
            Write-Error "Build first: cargo build --release --locked (looked for $bin)"
            exit 2
        }
        $docJson = & $bin --format json --modules gpu,battery,display,packages,desktop,locale |
            Out-String
        if ($LASTEXITCODE -ne 0) {
            Write-Error "justfetch exited $LASTEXITCODE"
            exit 2
        }
        $observation = Get-WindowsObservation
    }

    # UTF-8 without a BOM: Windows PowerShell 5.1's Out-File adds one, and a BOM
    # is not welcome in every JSON reader.
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    $docPath = Join-Path $work 'doc.json'
    $nativePath = Join-Path $work 'native.json'
    [System.IO.File]::WriteAllText($docPath, $docJson, $utf8)
    [System.IO.File]::WriteAllText($nativePath, ($observation | ConvertTo-Json -Depth 6), $utf8)

    # The Store's `python` stub resolves but does not run; test each candidate.
    $python = $null
    foreach ($candidate in @(Get-Command python3, python -ErrorAction SilentlyContinue)) {
        & $candidate.Source -c 'import sys' 2>$null
        if ($LASTEXITCODE -eq 0) { $python = $candidate; break }
    }
    if (-not $python) {
        if ($SelfTest) {
            Write-Error 'self-test needs python3 to run compare.py'
            exit 2
        }
        Write-Host 'Python 3 is not installed, so the comparison cannot run. Raw values:'
        $doc = $docJson | ConvertFrom-Json
        foreach ($module in 'gpu', 'battery', 'display', 'packages', 'desktop', 'locale') {
            $field = $doc.$module
            $detail = if ($field.PSObject.Properties['reason']) { $field.reason } else { $field.source }
            Write-Host ("{0,-9} {1,-12} {2}" -f $module, $field.status, $detail)
        }
        Write-Host 'Platform observations:'
        Get-Content $nativePath -Raw | Write-Host
        exit 2
    }

    $compare = Join-Path $here 'compare.py'
    & $python.Source $compare --strict --justfetch $docPath --native $nativePath
    $exit = $LASTEXITCODE

    if ($SelfTest) {
        $failures = @()
        if ($exit -ne 0) { $failures += "a document that matches its observations exited $exit, expected 0" }

        # The lists this script produces must be the ones compare.py reads back.
        $round = Get-Content $nativePath -Raw | ConvertFrom-Json
        if (-not $round.PSObject.Properties['absent']) { $failures += 'native.json has no absent list' }
        if (-not $round.PSObject.Properties['not_applicable']) { $failures += 'native.json has no not_applicable list' }
        if ($round.absent -notcontains 'desktop') { $failures += 'absent lost desktop' }
        if ($round.not_applicable -notcontains 'locale') { $failures += 'not_applicable lost locale' }

        # The same must hold for the structure the real collector returns, which
        # is the one a Windows run serializes. Its queries are best-effort on a
        # non-Windows host, but the shape is the same.
        $producedPath = Join-Path $work 'produced.json'
        $produced = Get-WindowsObservation
        [System.IO.File]::WriteAllText($producedPath, ($produced | ConvertTo-Json -Depth 6), $utf8)
        $back = Get-Content $producedPath -Raw | ConvertFrom-Json
        foreach ($key in 'tools', 'values', 'absent', 'not_applicable') {
            if (-not $back.PSObject.Properties[$key]) {
                $failures += "the collector's output loses `$key when serialized"
            }
        }
        # Keys alone are not enough: the Phase 17 defect was a list that stayed
        # empty because the line that filled it was never reached. What the
        # collector is expected to conclude on this host must be in the file.
        # `locale` is declared unconditionally; `desktop` only when no session
        # variable is set, which is the condition repeated here.
        if ($back.not_applicable -notcontains 'locale') {
            $failures += "the collector's not_applicable list does not carry locale"
        }
        $sessionSet = @('XDG_CURRENT_DESKTOP', 'XDG_SESSION_DESKTOP', 'DESKTOP_SESSION', 'WAYLAND_DISPLAY') |
            Where-Object { [Environment]::GetEnvironmentVariable($_) }
        if (-not $sessionSet -and ($back.absent -notcontains 'desktop')) {
            $failures += "the collector's absent list does not carry desktop"
        }
        & $python.Source $compare --justfetch $docPath --native $producedPath | Out-Null
        $shapeCode = $LASTEXITCODE
        if ($shapeCode -eq 2) {
            $failures += "compare.py rejects the collector's output as bad input (exit 2)"
        }

        # And a document that disagrees must not pass.
        $wrong = $SelfTestDocument -replace '"percent":80', '"percent":55'
        [System.IO.File]::WriteAllText($docPath, $wrong, $utf8)
        & $python.Source $compare --strict --justfetch $docPath --native $nativePath | Out-Null
        if ($LASTEXITCODE -ne 1) { $failures += "a mismatched document exited $LASTEXITCODE, expected 1" }

        if ($failures.Count -gt 0) {
            Write-Host 'self-test FAILED:'
            $failures | ForEach-Object { Write-Host "  - $_" }
            $exit = 1
        } else {
            Write-Host 'self-test OK: the native document is accepted, the evidence lists survive, and a mismatch is caught.'
            $exit = 0
        }
    }
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
exit $exit
