# Remove Windows Start Menu shortcut and temporary icon for TrackPersonalInsights.
# Run from any directory with PowerShell.

$ErrorActionPreference = 'Stop'

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $startMenu 'TrackPersonalInsights.lnk'
$tempIco = Join-Path $env:TEMP 'trackinsights.ico'

$removed = $false

if (Test-Path $shortcutPath) {
  Remove-Item $shortcutPath -Force
  Write-Output "Removed shortcut: $shortcutPath"
  $removed = $true
}

if (Test-Path $tempIco) {
  Remove-Item $tempIco -Force
  Write-Output "Removed temp icon: $tempIco"
  $removed = $true
}

if (-not $removed) {
  Write-Output "Nothing to remove; shortcut/icon not found."
} else {
  Write-Output "Uninstall complete."
}
