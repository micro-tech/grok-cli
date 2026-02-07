# Release Format Documentation

## Overview

Grok CLI releases are distributed as ZIP archives for each supported platform. Each release includes the binary, README, and LICENSE files in a convenient package.

## Supported Platforms

### Windows (x86_64)
- **Filename:** `grok-cli-windows-x86_64.zip`
- **Binary:** `grok.exe`
- **Target:** `x86_64-pc-windows-msvc`

### macOS (x86_64)
- **Filename:** `grok-cli-macos-x86_64.zip`
- **Binary:** `grok`
- **Target:** `x86_64-apple-darwin`

### Linux (x86_64)
- **Filename:** `grok-cli-linux-x86_64.zip`
- **Binary:** `grok`
- **Target:** `x86_64-unknown-linux-gnu`

## ZIP Archive Contents

Each ZIP file contains:
```
grok-cli-{platform}-x86_64.zip
├── grok(.exe)       # The binary executable
├── README.md        # Project documentation
└── LICENSE          # License file
```

## Installation Instructions

### Windows

1. Download `grok-cli-windows-x86_64.zip`
2. Right-click and select "Extract All..."
3. Navigate to the extracted folder
4. Run `grok.exe` from Command Prompt or PowerShell

**Add to PATH (Optional):**
```powershell
# Add the directory containing grok.exe to your PATH
$env:Path += ";C:\path\to\grok"
```

### macOS

1. Download `grok-cli-macos-x86_64.zip`
2. Double-click to extract (or use `unzip` in Terminal)
3. Open Terminal and navigate to the extracted folder
4. Make the binary executable (if needed):
   ```bash
   chmod +x grok
   ```
5. Run the binary:
   ```bash
   ./grok --version
   ```

**Install Globally (Optional):**
```bash
sudo mv grok /usr/local/bin/
grok --version
```

### Linux

1. Download `grok-cli-linux-x86_64.zip`
2. Extract the archive:
   ```bash
   unzip grok-cli-linux-x86_64.zip -d grok-cli
   cd grok-cli
   ```
3. Make the binary executable:
   ```bash
   chmod +x grok
   ```
4. Run the binary:
   ```bash
   ./grok --version
   ```

**Install Globally (Optional):**
```bash
sudo mv grok /usr/local/bin/
grok --version
```

## Verification

After installation, verify the binary works:

```bash
# Check version
grok --version

# Display help
grok --help

# Test basic functionality
grok query "Hello, Grok!"
```

Expected output:
```
grok-cli x.y.z
```

## Release Workflow

### For Maintainers

Releases are automatically created when a version tag is pushed:

```powershell
# Using the build script (recommended)
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push

# Or manually
git tag -a v0.1.4 -m "Release v0.1.4"
git push origin v0.1.4
```

### GitHub Actions Process

1. **Trigger:** Tag push matching `v*` pattern
2. **Build:** Compile binaries for Windows, macOS, Linux
3. **Package:** Create ZIP archives with binaries + docs
4. **Release:** Create GitHub Release with all ZIPs
5. **Artifacts:** Upload ZIPs as release assets

### Build Targets

Each platform is built with specific targets:
- Windows: Stable toolchain + MSVC
- macOS: Stable toolchain + Apple Darwin
- Linux: Stable toolchain + GNU

### Archive Creation

**Unix (Linux/macOS):**
```bash
cd release
zip -r ../grok-cli-{platform}-x86_64.zip .
```

**Windows:**
```powershell
Compress-Archive -Path release\* -DestinationPath grok-cli-windows-x86_64.zip
```

## Download Locations

### GitHub Releases
Primary distribution: https://github.com/microtech/grok-cli/releases

Each release includes:
- ZIP archives for all platforms
- Release notes with changelog
- Installation instructions

### Direct Download Links

Latest release assets:
```
https://github.com/microtech/grok-cli/releases/latest/download/grok-cli-windows-x86_64.zip
https://github.com/microtech/grok-cli/releases/latest/download/grok-cli-macos-x86_64.zip
https://github.com/microtech/grok-cli/releases/latest/download/grok-cli-linux-x86_64.zip
```

Specific version:
```
https://github.com/microtech/grok-cli/releases/download/v0.1.4/grok-cli-windows-x86_64.zip
```

## File Sizes

Typical archive sizes (approximate):
- Windows: 5-8 MB (compressed), 15-20 MB (extracted)
- macOS: 5-8 MB (compressed), 15-20 MB (extracted)
- Linux: 5-8 MB (compressed), 15-20 MB (extracted)

*Note: Sizes may vary depending on features and dependencies.*

## Checksums

Future releases will include SHA256 checksums for verification:

```bash
# Generate checksum (maintainers)
sha256sum grok-cli-*.zip > SHA256SUMS.txt

# Verify checksum (users)
sha256sum -c SHA256SUMS.txt
```

## Security

### Binary Signing

**Planned:** Future releases will include signed binaries
- Windows: Authenticode signature
- macOS: Code signing with Apple Developer ID
- Linux: GPG signatures

### Verification

Always download from official sources:
- ✅ GitHub Releases: https://github.com/microtech/grok-cli/releases
- ❌ Third-party mirrors (not recommended)

## Troubleshooting

### Windows: "Windows protected your PC"

This SmartScreen warning appears for unsigned binaries:
1. Click "More info"
2. Click "Run anyway"

**Future:** Binaries will be signed to avoid this warning.

### macOS: "Cannot be opened because it is from an unidentified developer"

1. Right-click the binary
2. Select "Open"
3. Click "Open" in the dialog

**Alternative:**
```bash
xattr -d com.apple.quarantine grok
```

### Linux: "Permission denied"

Make the binary executable:
```bash
chmod +x grok
```

## Changelog

Release notes for each version are available at:
- GitHub Releases page
- CHANGELOG.md in the repository

## Support

- **Issues:** https://github.com/microtech/grok-cli/issues
- **Email:** john.microtech@gmail.com
- **Discussions:** https://github.com/microtech/grok-cli/discussions
- **Buy Me a Coffee:** https://buymeacoffee.com/micro.tech

## Version Numbering

Grok CLI follows Semantic Versioning (SemVer):
- **MAJOR.MINOR.PATCH** (e.g., 1.2.3)
- MAJOR: Breaking changes
- MINOR: New features (backwards compatible)
- PATCH: Bug fixes

## Future Plans

- [ ] ARM64 support (Apple Silicon, ARM Linux)
- [ ] Binary signing for all platforms
- [ ] Checksums in releases
- [ ] Homebrew formula (macOS)
- [ ] APT repository (Debian/Ubuntu)
- [ ] Windows installer (.msi)
- [ ] Snap package (Linux)

---

**Last Updated:** January 2025  
**Release Format Version:** 1.0  
**Maintained By:** John McConnell