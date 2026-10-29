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

    $battery = Get-CimInstance -ClassName Win32_Battery -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($battery) {
        $values['battery'] = @{ percent = [int]$battery.EstimatedChargeRemaining }
        $tools['battery'] = 'Win32_Battery'
    }

    $adapters = @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction SilentlyContinue)
    if ($adapters.Count -gt 0) {
        # compare.py expects {"names": [...]}: a bare list is rejected as bad input.
        $values['gpu'] = @{ names = @($adapters | ForEach-Object { $_.Name }) }
        $tools['gpu'] = 'Win32_VideoController'
    }
    $withMode = $adapters | Where-Object { $_.CurrentHorizontalResolution } | Select-Object -First 1
    if ($withMode) {
        $values['display'] = @("$($withMode.CurrentHorizontalResolution)x$($withMode.CurrentVerticalResolution)")
        $tools['display'] = 'Win32_VideoController'
    }

    $scoop = Join-Path $env:USERPROFILE 'scoop\apps'
    if (Test-Path $scoop) {
        $values['packages'] = @{ scoop = @(Get-ChildItem $scoop -Directory).Count }
        $tools['packages'] = 'scoop/apps'
    }

    # locale is deliberately absent: justfetch reads GetUserDefaultLocaleName,
    # and comparing it with CultureInfo would compare the same source twice.

    $nativeJson = @{ tools = $tools; values = $values } | ConvertTo-Json -Depth 6
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
