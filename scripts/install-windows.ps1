# Install a Windows Start Menu shortcut for TrackPersonalInsights that opens in a terminal with an icon.
# Usage: run in PowerShell (with appropriate execution policy) from repo root.

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
$bin = Join-Path $repo 'target\release\TrackPersonalInsights.exe'
$iconSrc = Join-Path $repo 'assets\trackinsights.svg'

if (-not (Test-Path $bin)) {
  Write-Error "Binary not found at $bin. Build it with: cargo build --release --target x86_64-pc-windows-gnu"
}

# Convert SVG to ICO using PowerShell if `magick` or `inkscape` is present; otherwise fallback to svg.
$tempIco = Join-Path $env:TEMP 'trackinsights.ico'
if (Get-Command magick -ErrorAction SilentlyContinue) {
  magick convert $iconSrc -resize 256x256 $tempIco
} elseif (Get-Command inkscape -ErrorAction SilentlyContinue) {
  inkscape $iconSrc --export-filename=$tempIco --export-width=256 --export-height=256
} else {
  $tempIco = $iconSrc  # fallback to svg
}

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $startMenu 'TrackPersonalInsights.lnk'

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = 'powershell.exe'
$shortcut.Arguments = "-NoLogo -NoExit -Command \"\"$bin\"\""
$shortcut.WorkingDirectory = $repo
$shortcut.IconLocation = "$tempIco,0"
$shortcut.Save()

Write-Output "Shortcut created at $shortcutPath"
Write-Output "Launch 'TrackPersonalInsights' from Start Menu."
