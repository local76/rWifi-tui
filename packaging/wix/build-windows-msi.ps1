# Get the directory of the script
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $ScriptDir

Write-Host "Building Windows MSI Package using WiX..."

# Navigate to project root to run cargo wix
Push-Location ..\..
cargo wix --wxs packaging/wix/main.wxs

# Ensure dist/packages exists
New-Item -ItemType Directory -Force -Path "dist/packages" | Out-Null

# Copy build result to dist/packages/rwif.msi
$msiPath = Get-ChildItem -Path "target/wix/*.msi" | Select-Object -First 1
if ($msiPath) {
    Copy-Item $msiPath.FullName -Destination "dist/packages/rwif.msi" -Force
    Write-Host "MSI copied successfully to dist/packages/rwif.msi"
} else {
    Write-Error "Error: No MSI installer found in target/wix/"
}

Pop-Location
Pop-Location
