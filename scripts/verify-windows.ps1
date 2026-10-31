# Compare justfetch's Windows collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# unsupported, inconclusive, not comparable or not covered. Runs compare.py in
# strict acceptance mode: exit 1 on a mismatch or error, 2 on a setup problem,
# 3 when a module has neither an observation nor a declared exclusion, or when
# nothing was confirmed at all.
#
# The comparison needs Python 3 (scripts/compare.py). Without it the raw
# observations are printed and the script exits 2.
#
# -SelfTest checks the seam between this script and compare.py: that a fixed
# document and observation pair exits 0, that the evidence lists survive
# serialization, that the real collector's branches reach the file, that a
# disagreeing document exits 1, and that an undeclared gap exits 3. It also
# drives `Get-WindowsObservation` itself, whose Windows-only calls fail
# harmlessly off Windows. A real Windows run is still the only evidence about
# what those calls return.
param([switch]$SelfTest)
$ErrorActionPreference = 'Continue'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path

# The fixture both self-test scenarios are built from, so a laptop and a desktop
# differ in one property rather than in two copies of the same JSON.
function New-SelfTestDocument {
    [ordered]@{
        schema_version = 1
        environment = [ordered]@{ kind = 'native' }
        gpu = [ordered]@{
            status = 'available'; source = 't'; confidence = 'confirmed'
            value = [ordered]@{ devices = @([ordered]@{
                name = 'Test Adapter'; vendor = 'Test'; id = $null
                driver = $null; kind = 'discrete'
            }) }
        }
        battery = [ordered]@{
            status = 'available'; source = 't'; confidence = 'confirmed'
            value = [ordered]@{
                units = @([ordered]@{ name = 'Battery 0'; percent = 80; state = 'discharging' })
                power = 'battery'
            }
        }
        display = [ordered]@{
            status = 'available'; source = 't'; confidence = 'confirmed'
            value = [ordered]@{
                displays = @([ordered]@{
                    name = $null; width = 1920; height = 1080
                    refresh_hz = 60; primary = $true; kind = 'unknown'
                })
                definition = 't'
            }
        }
        packages = [ordered]@{ status = 'unsupported'; reason = 'no package manager' }
        desktop = [ordered]@{ status = 'unsupported'; reason = 'no desktop' }
        locale = [ordered]@{
            status = 'available'; source = 't'; confidence = 'confirmed'
            value = [ordered]@{
                process = 'en-US'; process_source = 'GetUserDefaultLocaleName'
                system = 'en-US'; system_source = 'GetUserDefaultLocaleName'
            }
        }
    }
}

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
        # desktop and packages were checked and found absent; locale cannot be
        # compared here. Both lists must survive into the file compare.py reads.
        absent = @('desktop', 'packages')
        not_applicable = @('locale')
    }
}

# One definition of "no desktop session", used by the collector and by the
# self-test that checks the collector's conclusion reached the file. Two copies
# of this list would let one go stale while the other still passed.
function Test-NoDesktopSession {
    $vars = @('XDG_CURRENT_DESKTOP', 'XDG_SESSION_DESKTOP', 'DESKTOP_SESSION', 'WAYLAND_DISPLAY')
    -not ($vars | Where-Object { [Environment]::GetEnvironmentVariable($_) })
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
        } elseif ($null -ne $battery.EstimatedChargeRemaining) {
            $values['battery'] = @{ percent = [int]$battery.EstimatedChargeRemaining }
            $tools['battery'] = 'Win32_Battery'
        } else {
            # A battery with no percentage is not a battery at 0%.
            $notApplicable += 'battery'
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
    # Limits, written down rather than guessed at: `Screen.Bounds` is in
    # physical pixels once the process calls SetProcessDPIAware. justfetch has
    # no DPI manifest, so GetMonitorInfoW gives *it* virtualized coordinates
    # under display scaling — the two can legitimately disagree on a scaled
    # display, and that is a limit of the comparison, not a defect. Refresh rate
    # is not compared at all: Screen has no equivalent.
    # Only compare at 100% scaling. justfetch has no DPI manifest, so
    # GetMonitorInfoW gives it virtualized coordinates while this process is DPI
    # aware; on a scaled display the two describe different rectangles and a
    # comparison would report a mismatch that is not a defect.
    $scaling = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' `
        -Name AppliedDPI -ErrorAction SilentlyContinue).AppliedDPI
    if ($scaling -and $scaling -ne 96) {
        $notApplicable += 'display'
    } else {
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
    }
    # Adapter names come from a different API than justfetch's EnumDisplayDevicesW,
    # so a GPU mismatch here is a wording difference more often than a defect.

    # USERPROFILE is always set on Windows; the guard lets the self-test exercise
    # this function's shape on a host that has no Windows environment.
    # `JF_SCOOP_APPS` lets the self-test drive this branch on a host that is not
    # Windows. Without it the collector's Scoop logic could only be tested by
    # owning a Windows machine, which is how the Phase 17 defect survived.
    $scoop = if ($env:JF_SCOOP_APPS) {
        $env:JF_SCOOP_APPS
    } elseif ($env:USERPROFILE) {
        Join-Path $env:USERPROFILE 'scoop\apps'
    } else {
        $null
    }
    if ($scoop) {
        $scoopApps = if (Test-Path $scoop) { @(Get-ChildItem $scoop -Directory).Count } else { 0 }
        if ($scoopApps -gt 0) {
            $values['packages'] = @{ scoop = $scoopApps }
            $tools['packages'] = 'scoop/apps'
        } else {
            # Looked and found nothing: that is what justfetch's "unsupported"
            # rests on. Without this, a stock Windows machine reports a gap and
            # fails strict for a package manager it does not use.
            $absent += 'packages'
        }
    }

    # locale is declared not comparable: justfetch reads GetUserDefaultLocaleName,
    # and comparing it with CultureInfo would compare the same source twice.
    $notApplicable += 'locale'

    # Windows has no desktop environment in justfetch's sense. Evidence: none of
    # the variables a desktop session exports are present.
    if (Test-NoDesktopSession) {
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
        $docJson = New-SelfTestDocument | ConvertTo-Json -Depth 8 -Compress
        $observation = Get-SelfTestObservation
    } else {
        $bin = if ($env:BIN) { $env:BIN } else { Join-Path $here '..\target\release\justfetch.exe' }
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
        # collector concludes on this host must reach the file.
        if ($back.not_applicable -notcontains 'locale') {
            $failures += "the collector's not_applicable list does not carry locale"
        }

        # The collector's own branches, driven through the seam rather than
        # restated: a machine with no Scoop must declare it absent, and one with
        # two apps must report the count.
        $missing = Join-Path $work 'no-scoop-here'
        $env:JF_SCOOP_APPS = $missing
        $withoutScoop = Get-WindowsObservation
        if ($withoutScoop.absent -notcontains 'packages') {
            $failures += 'the collector does not declare packages absent when Scoop is missing'
        }
        $env:JF_SCOOP_APPS = $null

        $withScoop = Join-Path $work 'scoop-apps'
        New-Item -ItemType Directory -Path (Join-Path $withScoop 'app-one') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $withScoop 'app-two') -Force | Out-Null
        $env:JF_SCOOP_APPS = $withScoop
        $producedScoop = Get-WindowsObservation
        if ($producedScoop.values['packages']['scoop'] -ne 2) {
            $failures += "the collector did not count two Scoop apps (got $($producedScoop.values['packages']['scoop']))"
        }
        if ($producedScoop.absent -contains 'packages') {
            $failures += 'the collector declares packages absent while Scoop has apps'
        }
        $env:JF_SCOOP_APPS = $null
        if ((Test-NoDesktopSession) -and ($back.absent -notcontains 'desktop')) {
            $failures += "the collector's absent list does not carry desktop"
        }
        & $python.Source $compare --justfetch $docPath --native $producedPath | Out-Null
        $shapeCode = $LASTEXITCODE
        if ($shapeCode -eq 2) {
            $failures += "compare.py rejects the collector's output as bad input (exit 2)"
        }

        # And a document that disagrees must not pass.
        $wrongDoc = New-SelfTestDocument
        $wrongDoc.battery.value.units[0].percent = 55
        $wrong = $wrongDoc | ConvertTo-Json -Depth 8 -Compress
        [System.IO.File]::WriteAllText($docPath, $wrong, $utf8)
        & $python.Source $compare --strict --justfetch $docPath --native $nativePath | Out-Null
        if ($LASTEXITCODE -ne 1) { $failures += "a mismatched document exited $LASTEXITCODE, expected 1" }

        # The combination the script exists for: a stock Windows machine with no
        # Scoop, no battery and no desktop environment must pass strict. Building
        # it here rather than trusting the collector on this host is the point —
        # the Phase 17 style of defect was a correct component and a failing run.
        # A desktop machine: an adapter and a monitor, no battery, no Scoop, no
        # desktop environment. justfetch reports the first two as values and the
        # rest as unsupported.
        # The same fixture as a desktop machine: no battery to report.
        $stockFixture = New-SelfTestDocument
        $stockFixture.battery = [ordered]@{ status = 'unsupported'; reason = 'no battery' }
        $stockDoc = $stockFixture | ConvertTo-Json -Depth 8 -Compress
        $stockNative = [ordered]@{
            tools = [ordered]@{ gpu = 'self-test'; display = 'self-test' }
            values = [ordered]@{
                gpu = @{ names = @('Test Adapter') }
                display = @('1920x1080')
            }
            absent = @('battery', 'desktop', 'packages')
            not_applicable = @('locale')
        }
        $stockDocPath = Join-Path $work 'stock-doc.json'
        $stockNativePath = Join-Path $work 'stock-native.json'
        [System.IO.File]::WriteAllText($stockDocPath, $stockDoc, $utf8)
        [System.IO.File]::WriteAllText($stockNativePath, ($stockNative | ConvertTo-Json -Depth 6), $utf8)
        & $python.Source $compare --strict --justfetch $stockDocPath --native $stockNativePath | Out-Null
        $stockCode = $LASTEXITCODE
        if ($stockCode -ne 0) {
            $failures += "a stock Windows machine exited $stockCode, expected 0 (gap in the evidence lists)"
        }

        # And strict must reject a run where a module has neither observation nor
        # declaration: exit 3 is the code this script promises and never used to
        # reach.
        $gapNative = [ordered]@{
            tools = [ordered]@{}; values = [ordered]@{}
            absent = @('desktop'); not_applicable = @('locale')
        }
        $gapPath = Join-Path $work 'gap-native.json'
        [System.IO.File]::WriteAllText($gapPath, ($gapNative | ConvertTo-Json -Depth 6), $utf8)
        & $python.Source $compare --strict --justfetch $stockDocPath --native $gapPath | Out-Null
        if ($LASTEXITCODE -ne 3) {
            $failures += "an undeclared gap exited $LASTEXITCODE, expected 3"
        }

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
