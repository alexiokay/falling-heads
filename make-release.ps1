# FallingHeads Release Script
# Creates a distributable ZIP package

param(
    [string]$Version = "0.2.5"
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Building FallingHeads v$Version for distribution..." -ForegroundColor Cyan

# Step 1: Build release
Write-Host "`n📦 Step 1: Building release executable..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Build complete!" -ForegroundColor Green

# Step 2: Create distribution folder
$DistFolder = "FallingHeads-v$Version-Windows"
Write-Host "`n📁 Step 2: Creating distribution folder: $DistFolder" -ForegroundColor Yellow

if (Test-Path $DistFolder) {
    Write-Host "⚠️  Removing old distribution folder..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $DistFolder
}

New-Item -ItemType Directory -Path $DistFolder | Out-Null

# Step 3: Copy executable
Write-Host "`n📋 Step 3: Copying files..." -ForegroundColor Yellow
Copy-Item "target\release\falling-heads.exe" -Destination $DistFolder
Write-Host "  ✓ Copied falling-heads.exe" -ForegroundColor Green

# Step 4: Copy assets
if (Test-Path "assets") {
    Copy-Item -Recurse "assets" -Destination "$DistFolder\assets"
    Write-Host "  ✓ Copied assets folder" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  No assets folder found" -ForegroundColor Yellow
}

# Step 5: Copy DLLs (if any)
$DllFiles = Get-ChildItem "target\release\*.dll" -ErrorAction SilentlyContinue
if ($DllFiles) {
    foreach ($dll in $DllFiles) {
        Copy-Item $dll.FullName -Destination $DistFolder
        Write-Host "  ✓ Copied $($dll.Name)" -ForegroundColor Green
    }
}

# Step 6: Create README.txt
Write-Host "`n📝 Step 4: Creating README.txt..." -ForegroundColor Yellow

$ReadmeLines = @(
    "==========================================="
    "       FallingHeads v$Version - Windows"
    "==========================================="
    ""
    "SYSTEM REQUIREMENTS:"
    "- Windows 10/11 (64-bit)"
    "- Graphics card with OpenGL 3.3+ or DirectX 11"
    "- 4 GB RAM"
    "- 200 MB disk space"
    ""
    "HOW TO RUN:"
    "1. Extract this folder anywhere"
    "2. Double-click falling-heads.exe"
    "3. Enjoy!"
    ""
    "CONTROLS:"
    "- A/D or Arrow Keys: Move hero left/right"
    "- Hero has automatic AI movement"
    ""
    "PERFORMANCE:"
    "- First launch may take 10-20s to load assets"
    "- If you get low FPS, try closing other apps"
    ""
    "TROUBLESHOOTING:"
    "If game doesn't start:"
    "1. Install Visual C++ Redistributable:"
    "   https://aka.ms/vs/17/release/vc_redist.x64.exe"
    "2. Update graphics drivers"
    ""
    "WHAT'S NEW IN v$Version`:"
    "- Major performance improvements (CPU usage down 80% when idle)"
    "- Fixed entity despawn crashes"
    "- Death animations now exactly 1.0 second"
    "- Enhanced performance diagnostics"
    ""
    "LICENSE:"
    "This game is provided as-is for free."
    ""
    "Built with Bevy Engine (https://bevyengine.org)"
    "© 2025"
    ""
    "==========================================="
)

$ReadmeLines -join "`r`n" | Set-Content -Path "$DistFolder\README.txt"
Write-Host "  ✓ Created README.txt" -ForegroundColor Green

# Step 7: Calculate size
$TotalSize = (Get-ChildItem -Recurse $DistFolder | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "`n📊 Distribution folder size: $([math]::Round($TotalSize, 2)) MB" -ForegroundColor Cyan

# Step 8: Create ZIP
Write-Host "`n🗜️  Step 5: Creating ZIP archive..." -ForegroundColor Yellow
$ZipFile = "$DistFolder.zip"
if (Test-Path $ZipFile) {
    Remove-Item $ZipFile
}

Compress-Archive -Path $DistFolder -DestinationPath $ZipFile
$ZipSize = (Get-Item $ZipFile).Length / 1MB
Write-Host "  ✓ Created $ZipFile ($([math]::Round($ZipSize, 2)) MB)" -ForegroundColor Green

# Success!
Write-Host "`n✅ Release package created successfully!" -ForegroundColor Green
Write-Host "`n📦 Distribution folder: $DistFolder" -ForegroundColor Cyan
Write-Host "📦 ZIP archive: $ZipFile" -ForegroundColor Cyan
Write-Host "`nℹ️  Upload $ZipFile to itch.io or GitHub releases!" -ForegroundColor Yellow
Write-Host "`n🎮 Test the game by running: .\$DistFolder\falling-heads.exe" -ForegroundColor Cyan
