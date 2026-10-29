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
    & $bin --format json --modules gpu,battery,display,packages,desktop,locale |
        Out-File -Encoding utf8 (Join-Path $work 'doc.json')
    if ($LASTEXITCODE -ne 0) { Write-Error "justfetch exited $LASTEXITCODE"; exit 2 }

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
        $values['gpu'] = @($adapters | ForEach-Object { $_.Name })
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

    @{ tools = $tools; values = $values } | ConvertTo-Json -Depth 6 |
        Out-File -Encoding utf8 (Join-Path $work 'native.json')

    $python = Get-Command python3, python -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($python) {
        & $python.Source (Join-Path $here 'compare.py') `
            --justfetch (Join-Path $work 'doc.json') --native (Join-Path $work 'native.json')
        exit $LASTEXITCODE
    }

    Write-Host 'Python 3 is not installed, so the comparison cannot run. Raw values:'
    Get-Content (Join-Path $work 'doc.json') | Write-Host
    Get-Content (Join-Path $work 'native.json') | Write-Host
    exit 2
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
