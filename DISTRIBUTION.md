# MineHeads - Distribution Guide

## 🚀 Building for Release

### 1. Build the Game

```powershell
cargo build --release
```

This creates an optimized executable at:
```
target/release/falling-heads.exe
```

**Build time**: ~5-10 minutes (first time), ~1-2 minutes (incremental)

---

## 📦 Creating Distribution Package

### Required Files

Your game needs **3 things** to run on another computer:

1. **Executable**: `falling-heads.exe`
2. **Assets folder**: `assets/` (textures, sounds, etc.)
3. **Dynamic Link Library**: `falling-heads.dll` (if exists)

### Step 1: Create Distribution Folder

```powershell
# Create distribution folder
mkdir MineHeads-v0.2.5-Windows

# Copy executable
copy target\release\falling-heads.exe MineHeads-v0.2.5-Windows\

# Copy assets folder
xcopy assets MineHeads-v0.2.5-Windows\assets /E /I

# Copy README (optional)
copy README.md MineHeads-v0.2.5-Windows\
```

### Step 2: Check Dependencies

```powershell
# Navigate to release folder
cd target/release

# List all DLL files
dir *.dll
```

**Copy any DLLs** found to your distribution folder.

### Step 3: Create ZIP Archive

**Option A: Windows Explorer**
1. Right-click `MineHeads-v0.2.5-Windows` folder
2. Send to → Compressed (zipped) folder

**Option B: PowerShell**
```powershell
Compress-Archive -Path MineHeads-v0.2.5-Windows -DestinationPath MineHeads-v0.2.5-Windows.zip
```

---

## 🎮 Distribution Folder Structure

Your final package should look like:

```
MineHeads-v0.2.5-Windows/
├── falling-heads.exe          # Main executable (~50-100 MB)
├── assets/                     # Game assets
│   ├── textures/
│   │   ├── blocks/            # Block textures
│   │   ├── entities/          # Character textures
│   │   ├── effects/           # VFX textures
│   │   └── projectiles/       # Arrow, etc.
│   └── sounds/                # Audio files (if any)
└── README.md                  # Optional: Instructions
```

---

## 📝 Create README.txt for Players

Create a file `README.txt` in your distribution folder:

```text
===========================================
       MineHeads v0.2.5 - Windows
===========================================

SYSTEM REQUIREMENTS:
- Windows 10/11 (64-bit)
- Graphics card with OpenGL 3.3+ or DirectX 11
- 4 GB RAM
- 200 MB disk space

HOW TO RUN:
1. Extract the ZIP file
2. Double-click falling-heads.exe
3. Enjoy!

CONTROLS:
- A/D or Arrow Keys: Move left/right
- (Controls are automatic AI movement by default)

KNOWN ISSUES:
- First launch may take 10-20 seconds to load assets
- If game doesn't start, install Visual C++ Redistributable:
  https://aka.ms/vs/17/release/vc_redist.x64.exe

SUPPORT:
Report bugs at: [Your GitHub Issues URL]

LICENSE:
[Your License Here]

© 2025 - Built with Bevy Engine
===========================================
```

---

## 🌐 Publishing Options

### Option 1: Itch.io (Recommended for Indie Games)

**Pros**:
- Easy upload
- Built-in download analytics
- Community features
- Optional pay-what-you-want pricing

**Steps**:
1. Create account at https://itch.io
2. Click "Upload new project"
3. Upload your ZIP file
4. Set pricing (free or paid)
5. Publish!

**Example page**: https://youtu.itch.io/mineheads

---

### Option 2: GitHub Releases

**Pros**:
- Free hosting
- Version control
- Easy for developers

**Steps**:
```bash
# Tag your version
git tag -a v0.2.5 -m "Performance & Stability Update"
git push origin v0.2.5

# Create release on GitHub
# Go to: https://github.com/[username]/[repo]/releases/new
# Upload: MineHeads-v0.2.5-Windows.zip
```

---

### Option 3: Steam (Advanced)

**Requirements**:
- $100 Steam Direct fee
- More complex setup
- Best for commercial releases

**Steps**: https://partner.steamgames.com/

---

## 🔧 Advanced: Reduce File Size

### 1. Strip Debug Symbols

Add to `Cargo.toml`:
```toml
[profile.release]
strip = true              # Strip debug symbols
lto = true                # Link-time optimization
codegen-units = 1         # Better optimization
opt-level = 'z'           # Optimize for size (use 3 for speed)
```

**Result**: ~50% smaller executable

### 2. Compress Textures

Your textures in `assets/textures/` can be compressed:
- Use PNG with transparency (already efficient)
- Consider WebP format for 25-35% size reduction
- Optimize with tools like `pngquant` or `oxipng`

### 3. Use UPX Compression

```powershell
# Download UPX: https://upx.github.io/
upx --best --lzma falling-heads.exe
```

**Warning**: Some antivirus may flag UPX-compressed executables as suspicious.

---

## 🐧 Building for Other Platforms

### Linux

```bash
# On Windows with WSL, or on Linux directly
cargo build --release --target x86_64-unknown-linux-gnu

# Output: target/x86_64-unknown-linux-gnu/release/falling-heads
```

### macOS (requires macOS to build)

```bash
cargo build --release --target x86_64-apple-darwin

# For Apple Silicon
cargo build --release --target aarch64-apple-darwin
```

### Cross-Compilation (Advanced)

Install `cross`:
```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

---

## ✅ Release Checklist

Before distributing:

- [ ] Build with `cargo build --release`
- [ ] Test executable runs on clean Windows install
- [ ] All assets load correctly
- [ ] No debug logs spamming console
- [ ] Create README.txt with instructions
- [ ] Zip/archive the distribution folder
- [ ] Test extracting and running the ZIP
- [ ] Create GitHub release tag
- [ ] Upload to itch.io / GitHub releases
- [ ] Share download link!

---

## 🎯 Quick Distribution Commands

```powershell
# Build release
cargo build --release

# Create distribution folder
mkdir MineHeads-v0.2.5-Windows
copy target\release\falling-heads.exe MineHeads-v0.2.5-Windows\
xcopy assets MineHeads-v0.2.5-Windows\assets /E /I
copy README.md MineHeads-v0.2.5-Windows\README.txt

# Create ZIP
Compress-Archive -Path MineHeads-v0.2.5-Windows -DestinationPath MineHeads-v0.2.5-Windows.zip

# Upload to itch.io or GitHub!
```

---

## 📊 Expected File Sizes

- **Executable**: 50-100 MB (Bevy engine included)
- **Assets**: 10-50 MB (textures)
- **Total ZIP**: 60-150 MB

**Note**: Bevy games are larger because they include the entire engine. This is normal!

---

## 🆘 Troubleshooting

### "VCRUNTIME140.dll was not found"
**Solution**: Players need Visual C++ Redistributable
- Download: https://aka.ms/vs/17/release/vc_redist.x64.exe

### "Failed to load assets"
**Solution**: Ensure `assets/` folder is in same directory as `.exe`

### Game crashes on startup
**Solution**: Check graphics drivers are updated
- NVIDIA: https://www.nvidia.com/Download/index.aspx
- AMD: https://www.amd.com/en/support

### Antivirus blocks executable
**Solution**:
- Sign your executable (costs money)
- Or users must manually allow it
- Upload to itch.io (helps with trust)

---

## 📜 License Recommendation

Add a `LICENSE` file. Popular options:

**MIT License** (permissive):
```
Copyright (c) 2025 [Your Name]

Permission is hereby granted, free of charge...
```

**Apache 2.0** (with patent protection)

**GPL 3.0** (copyleft - requires derivative works to be open source)

**Choose at**: https://choosealicense.com/

---

## 🎉 You're Ready to Distribute!

Your game is now ready to share with the world. Good luck! 🚀

For questions, check:
- Bevy Discord: https://discord.com/invite/bevy
- Bevy Documentation: https://bevyengine.org/learn/
