# Get the directory of the script
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $ScriptDir

Write-Host "Publishing to WinGet using wingetcreate..."

# Get version from Cargo.toml
$cargoTomlPath = Join-Path $ScriptDir "../../Cargo.toml"
$version = ""
if (Test-Path $cargoTomlPath) {
    $content = Get-Content $cargoTomlPath -Raw
    if ($content -match '(?m)^version\s*=\s*"([^"]+)"') {
        $version = $Matches[1]
    }
}
if (-not $version) {
    $version = "3.0.1"
}

# Replace TEMPLATE_VERSION in winget.yaml
$yamlPath = Join-Path $ScriptDir "winget.yaml"
if (Test-Path $yamlPath) {
    $yamlContent = Get-Content $yamlPath -Raw
    $yamlContent = $yamlContent -replace "TEMPLATE_VERSION", $version
    Set-Content $yamlPath $yamlContent
}

Write-Host "Running: wingetcreate submit winget.yaml"
Write-Host "Simulating: wingetcreate submit winget.yaml"

Pop-Location
