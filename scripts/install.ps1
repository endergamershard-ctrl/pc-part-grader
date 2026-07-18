# Install the latest PC Part Grader release on Windows (x64).
# Usage:
#   irm https://raw.githubusercontent.com/endergamershard-ctrl/pc-part-grader/master/scripts/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = if ($env:PC_PART_GRADER_REPO) { $env:PC_PART_GRADER_REPO } else { "endergamershard-ctrl/pc-part-grader" }
$Api = "https://api.github.com/repos/$Repo/releases/latest"

Write-Host "Fetching latest release from $Repo..."
$headers = @{
  "User-Agent" = "pc-part-grader-installer"
  "Accept"     = "application/vnd.github+json"
}
$release = Invoke-RestMethod -Uri $Api -Headers $headers

$asset = $release.assets |
  Where-Object { $_.name -match 'x64-setup\.exe$' -or $_.name -match 'setup\.exe$' } |
  Select-Object -First 1

if (-not $asset) {
  Write-Error "Could not find a Windows NSIS installer in the latest release. See https://github.com/$Repo/releases"
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) $asset.name
Write-Host "Downloading $($release.tag_name) ($($asset.name))..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmp -UseBasicParsing

Write-Host "Running installer..."
# NSIS silent install; falls back to interactive if the flag is ignored.
$proc = Start-Process -FilePath $tmp -ArgumentList "/S" -Wait -PassThru
if ($proc.ExitCode -ne 0) {
  Write-Host "Silent install returned exit code $($proc.ExitCode); launching interactive installer..."
  Start-Process -FilePath $tmp -Wait
}

Remove-Item -Force $tmp -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Installed PC Part Grader $($release.tag_name)"
Write-Host "Launch it from the Start Menu as 'PC Part Grader'."
Write-Host ""
Write-Host "Note: Windows builds are currently unsigned. SmartScreen may warn on first run — choose More info > Run anyway."
Write-Host "WebView2 Runtime is required (usually preinstalled on Windows 10/11)."
