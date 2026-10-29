# Cross-check justfetch's Windows collectors against the system's own tools.
#
# Read-only. Run on a real Windows 10/11 machine in PowerShell and paste the
# output into docs/validation.md under a dated heading.
$ErrorActionPreference = 'Continue'

$bin = if ($env:BIN) { $env:BIN } else { '.\target\release\justfetch.exe' }
if (-not (Test-Path $bin)) { Write-Error "Build first: cargo build --release --locked (looked for $bin)"; exit 1 }

function Section($name) { Write-Host "`n=== $name ===" }

Section 'environment'
Write-Host "os:        $([System.Environment]::OSVersion.VersionString)"
Write-Host "arch:      $env:PROCESSOR_ARCHITECTURE"
Write-Host "session:   $([System.Environment]::UserInteractive)"

Section 'justfetch --format json'
$doc = & $bin --format json | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { Write-Error "justfetch exited $LASTEXITCODE"; exit 1 }
foreach ($k in 'gpu','battery','display','packages','desktop','locale') {
    $item = $doc.$k
    $detail = if ($item.PSObject.Properties['reason']) { $item.reason } else { $item.source }
    Write-Host ("{0,-9} {1,-12} {2}" -f $k, $item.status, $detail)
}

Section 'battery (compare with justfetch)'
Get-CimInstance -ClassName Win32_Battery |
    Select-Object Name, EstimatedChargeRemaining, BatteryStatus |
    Format-Table -AutoSize | Out-String | Write-Host

Section 'gpu (compare with justfetch)'
Get-CimInstance -ClassName Win32_VideoController |
    Select-Object Name, AdapterCompatibility, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate |
    Format-Table -AutoSize | Out-String | Write-Host

Section 'packages (compare with justfetch)'
$scoopApps = Join-Path $env:USERPROFILE 'scoop\apps'
if (Test-Path $scoopApps) {
    Write-Host "scoop apps: $((@(Get-ChildItem $scoopApps -Directory)).Count)"
} else { Write-Host 'scoop: not installed' }
$winget = Get-Command winget -ErrorAction SilentlyContinue
if ($winget) {
    # winget list includes system packages; the number is context, not a count
    # justfetch claims to match.
    Write-Host "winget list entries: $((& winget list --accept-source-agreements 2>$null | Measure-Object -Line).Lines)"
} else { Write-Host 'winget: not available' }

Section 'locale (compare with justfetch)'
Write-Host "GetUserDefaultLocaleName equivalent: $([System.Globalization.CultureInfo]::CurrentCulture.Name)"
Write-Host "LANG=$env:LANG LC_ALL=$env:LC_ALL LC_MESSAGES=$env:LC_MESSAGES"

Section 'paste into docs/validation.md'
Write-Host 'Record: date, Windows edition and build, whether it was a VM or RDP'
Write-Host 'session, the six statuses above, and any line where justfetch and the'
Write-Host 'native tool disagree.'
