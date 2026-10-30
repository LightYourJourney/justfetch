# Compare justfetch's Windows collectors with the system's own tools.
#
# Read-only and repeatable. Prints one verdict per module: match, mismatch,
# uncomparable or unsupported. Exit 1 on any mismatch, 2 on a setup problem.
# The comparison itself needs Python 3 (scripts/compare.py); without it the
# raw observations are printed for a manual comparison and the script exits 2.
$ErrorActionPreference = 'Continue'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$bin = if ($env:BIN) { $env:BIN } else { Join-Path (Get-Location) 'target\release\justfetch.exe' }
if (-not (Test-Path $bin)) { Write-Error "Build first: cargo build --release --locked (looked for $bin)"; exit 2 }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("jf-windows-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $docJson = & $bin --format json --modules gpu,battery,display,packages,desktop,locale | Out-String
    if ($LASTEXITCODE -ne 0) { Write-Error "justfetch exited $LASTEXITCODE"; exit 2 }
    # UTF-8 without a BOM: Windows PowerShell 5.1's Out-File adds one, and a BOM
    # is not welcome in every JSON reader.
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText((Join-Path $work 'doc.json'), $docJson, $utf8)

    # Only non-identifying observations: percentages, adapter names, resolutions
    # and counts. No serial numbers, user names or addresses.
    $values = [ordered]@{}
    $tools = [ordered]@{}
    $absent = @()
    $notApplicable = @()

    # A failed CIM query is not evidence of absence: `-ErrorAction SilentlyContinue`
    # returns nothing either way, so the query is attempted explicitly and only a
    # successful empty result may declare the battery absent.
    try {
        $batteryQuery = @(Get-CimInstance -ClassName Win32_Battery -ErrorAction Stop)
        $battery = $batteryQuery | Select-Object -First 1
        if (-not $battery) {
            # The class exists and lists nothing: this machine has no battery.
            $absent += 'battery'
        } else {
            $values['battery'] = @{ percent = [int]$battery.EstimatedChargeRemaining }
            $tools['battery'] = 'Win32_Battery'
        }
    } catch {
        Write-Warning "Win32_Battery could not be queried: $_"
    }

    $adapters = @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction SilentlyContinue)
    if ($adapters.Count -gt 0) {
        # compare.py expects {"names": [...]}: a bare list is rejected as bad input.
        $values['gpu'] = @{ names = @($adapters | ForEach-Object { $_.Name }) }
        $tools['gpu'] = 'Win32_VideoController'
    }
    # justfetch lists monitors (EnumDisplayMonitors), not adapters, so the
    # comparison uses the same unit. Win32_VideoController reports one mode per
    # adapter and cannot see two monitors on one GPU.
    try {
        Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
        # GetMonitorInfoW reports physical pixels; the managed Bounds are DPI
        # virtualized unless the process is DPI aware, so ask for that first.
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

    $scoop = Join-Path $env:USERPROFILE 'scoop\apps'
    if (Test-Path $scoop) {
        $values['packages'] = @{ scoop = @(Get-ChildItem $scoop -Directory).Count }
        $tools['packages'] = 'scoop/apps'
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

    # The evidence lists must reach compare.py, or an `unsupported` verdict has
    # nothing behind it and `not comparable` declarations are lost.
    $nativeJson = @{
        tools = $tools; values = $values
        absent = @($absent); not_applicable = @($notApplicable)
    } | ConvertTo-Json -Depth 6
    [System.IO.File]::WriteAllText((Join-Path $work 'native.json'), $nativeJson, $utf8)

    # The Store's `python` stub resolves but does not run; test each candidate.
    $python = $null
    foreach ($candidate in @(Get-Command python3, python -ErrorAction SilentlyContinue)) {
        & $candidate.Source -c 'import sys' 2>$null
        if ($LASTEXITCODE -eq 0) { $python = $candidate; break }
    }
    if ($python) {
        & $python.Source (Join-Path $here 'compare.py') --strict `
            --justfetch (Join-Path $work 'doc.json') --native (Join-Path $work 'native.json')
        exit $LASTEXITCODE
    }

    # Print the six modules only. The full document carries the user and host
    # name in `identity`, and this script promises not to print those.
    Write-Host 'Python 3 is not installed, so the comparison cannot run. Raw values:'
    $doc = $docJson | ConvertFrom-Json
    foreach ($module in 'gpu','battery','display','packages','desktop','locale') {
        $field = $doc.$module
        $detail = if ($field.PSObject.Properties['reason']) { $field.reason } else { $field.source }
        Write-Host ("{0,-9} {1,-12} {2}" -f $module, $field.status, $detail)
    }
    Write-Host 'Platform observations:'
    Get-Content (Join-Path $work 'native.json') -Raw | Write-Host
    exit 2
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
