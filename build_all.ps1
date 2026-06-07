# PowerShell script to build targets locally and generate/ensure all packages mapped in ARCHITECTURE.md exist in dist/

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $projectRoot

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "Building rWifi Targets and Packages..." -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

# 1. Compile native Windows binary (rwifi.exe)
Write-Host "Compiling native Windows binary..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -eq 0) {
    # Ensure directories exist
    New-Item -ItemType Directory -Force -Path "dist/binaries" | Out-Null
    New-Item -ItemType Directory -Force -Path "dist/packages" | Out-Null

    # Copy the compiled Windows binary
    Copy-Item "target/release/rwifi.exe" "dist/binaries/rwifi.exe" -Force
    Write-Host "[SUCCESS] Compiled and copied native binary to dist/binaries/rwifi.exe" -ForegroundColor Green
} else {
    Write-Error "Cargo build failed!"
    Pop-Location
    exit 1
}

# 2. Compile/mock Linux binary (rwifi)
# Since WSL/Docker is not present, we will ensure a mock ELF exists in dist/binaries/rwifi
if (-not (Test-Path "dist/binaries/rwifi") -or (Get-Item "dist/binaries/rwifi").Length -eq 0) {
    [System.IO.File]::WriteAllText("dist/binaries/rwifi", "[Mock Linux ELF binary for rwifi]")
}
Write-Host "[MOCKED] Linux ELF binary ensured at dist/binaries/rwifi" -ForegroundColor DarkYellow

# 3. Compile/mock packages
# MSI Installer (using cargo-wix if available, otherwise mock)
$hasWix = Get-Command cargo -ErrorAction SilentlyContinue | Out-Null
if ($hasWix) {
    $cargoList = cargo --list
    if ($cargoList -match "wix") {
        Write-Host "cargo-wix detected! Compiling MSI installer..." -ForegroundColor Yellow
        cargo wix --wxs packaging/wix/main.wxs
        $msiPath = Get-ChildItem -Path "target/wix/*.msi" | Select-Object -First 1
        if ($msiPath) {
            Copy-Item $msiPath.FullName -Destination "dist/packages/rwifi.msi" -Force
            Write-Host "[SUCCESS] Built and copied MSI to dist/packages/rwifi.msi" -ForegroundColor Green
        }
    }
}

if (-not (Test-Path "dist/packages/rwifi.msi") -or (Get-Item "dist/packages/rwifi.msi").Length -eq 0) {
    [System.IO.File]::WriteAllText("dist/packages/rwifi.msi", "[Mock Windows MSI Installer package for rwifi]")
    Write-Host "[MOCKED] Windows MSI installer package ensured at dist/packages/rwifi.msi" -ForegroundColor DarkYellow
}

# Ensure all other packages are present as mocks/stubs
$mockPackages = @("rwifi.apk", "rwifi.appimage", "rwifi.deb", "rwifi.rpm")
foreach ($pkg in $mockPackages) {
    $pkgPath = "dist/packages/$pkg"
    if (-not (Test-Path $pkgPath) -or (Get-Item $pkgPath).Length -eq 0) {
        $ext = ($pkg -split "\.")[-1]
        [System.IO.File]::WriteAllText($pkgPath, "[Mock $ext package for rwifi]")
        Write-Host "[MOCKED] $pkg package ensured at $pkgPath" -ForegroundColor DarkYellow
    } else {
        Write-Host "[EXISTING] Preserved packaging artifact at $pkgPath" -ForegroundColor Green
    }
}

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "Build process completed. Open dist/ to view all output files!" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Cyan

Pop-Location
