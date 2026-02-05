#!/usr/bin/env pwsh
# PowerShell Build Script for grok-cli
#
# This script builds the project, runs tests, and can optionally create
# Git tags and push to GitHub to trigger release builds.
#
# Usage:
#   .\build.ps1                          # Basic build
#   .\build.ps1 -Release                 # Release build
#   .\build.ps1 -Test                    # Build and test
#   .\build.ps1 -Release -Tag "v0.1.4"   # Create and push tag
#   .\build.ps1 -All                     # Full build with all checks
#   .\build.ps1 -Release -Tag -Auto      # Auto-increment version and tag

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$Test,
    [switch]$Clippy,
    [switch]$Doc,
    [switch]$All,
    [switch]$Verbose,
    [switch]$Push,
    [string]$Tag = "",
    [switch]$Auto,
    [string]$Target = "",
    [string]$Remote = "origin",
    [string]$Branch = "main"
)

#region Helper Functions

function Write-Success
{
    param($msg)
    Write-Host "✓ $msg" -ForegroundColor Green
}

function Write-Error-Custom
{
    param($msg)
    Write-Host "✗ $msg" -ForegroundColor Red
}

function Write-Info
{
    param($msg)
    Write-Host "ℹ $msg" -ForegroundColor Cyan
}

function Write-Step
{
    param($msg)
    Write-Host "→ $msg" -ForegroundColor Yellow
}

function Write-Warning-Custom
{
    param($msg)
    Write-Host "⚠ $msg" -ForegroundColor Yellow
}

function Get-CargoVersion
{
    $cargoToml = Get-Content "Cargo.toml" -Raw
    if ($cargoToml -match 'version\s*=\s*"([^"]+)"')
    {
        return $matches[1]
    }
    return $null
}

function Set-CargoVersion
{
    param([string]$NewVersion)

    $cargoToml = Get-Content "Cargo.toml" -Raw
    $cargoToml = $cargoToml -replace 'version\s*=\s*"[^"]+"', "version = `"$NewVersion`""
    Set-Content "Cargo.toml" -Value $cargoToml -NoNewline
}

function Get-NextVersion
{
    param([string]$CurrentVersion)

    if ($CurrentVersion -match '^(\d+)\.(\d+)\.(\d+)$')
    {
        $major = [int]$matches[1]
        $minor = [int]$matches[2]
        $patch = [int]$matches[3]
        $patch++
        return "$major.$minor.$patch"
    }
    return $null
}

function Test-GitClean
{
    $status = git status --porcelain
    return [string]::IsNullOrEmpty($status)
}

function Test-GitTagExists
{
    param([string]$TagName)

    $tags = git tag -l $TagName
    return -not [string]::IsNullOrEmpty($tags)
}

#endregion

#region Main Script

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Change to project root
Push-Location $ProjectRoot

try
{
    Write-Info "=== Grok-CLI Build & Release Script ==="
    Write-Info "Project: $ProjectRoot"
    Write-Info ""

    # Build flags
    $BuildFlags = @()
    if ($Release)
    {
        $BuildFlags += "--release"
        Write-Info "Build Mode: Release"
    } else
    {
        Write-Info "Build Mode: Debug"
    }

    if ($Verbose)
    {
        $BuildFlags += "--verbose"
    }

    if ($Target)
    {
        $BuildFlags += "--target", $Target
        Write-Info "Target: $Target"
    }

    # Track success
    $Success = $true
    $BuildDir = if ($Release)
    { "release" 
    } else
    { "debug" 
    }

    #region Pre-flight Checks

    Write-Step "Pre-flight checks..."

    # Check if git is available
    if ($Tag -or $Push -or $Auto)
    {
        try
        {
            $null = git --version
        } catch
        {
            Write-Error-Custom "Git is not installed or not in PATH"
            exit 1
        }

        # Check if we're in a git repository
        if (-not (Test-Path ".git"))
        {
            Write-Error-Custom "Not in a git repository"
            exit 1
        }

        # Check for uncommitted changes
        if (-not (Test-GitClean))
        {
            Write-Warning-Custom "Working directory has uncommitted changes"
            if ($Tag -or $Auto)
            {
                Write-Error-Custom "Cannot create tag with uncommitted changes"
                Write-Info "Commit your changes first or run without -Tag/-Auto"
                exit 1
            }
        }
    }

    Write-Success "Pre-flight checks passed"
    Write-Host ""

    #endregion

    #region Clean

    if ($Clean -or $All)
    {
        Write-Step "Cleaning build artifacts..."
        cargo clean
        if ($LASTEXITCODE -eq 0)
        {
            Write-Success "Clean complete"
        } else
        {
            Write-Error-Custom "Clean failed"
            $Success = $false
        }
        Write-Host ""
    }

    #endregion

    #region Format Check

    if ($All)
    {
        Write-Step "Checking code formatting..."
        cargo fmt -- --check
        if ($LASTEXITCODE -eq 0)
        {
            Write-Success "Format check passed"
        } else
        {
            Write-Warning-Custom "Format check failed - run 'cargo fmt' to fix"
            # Don't fail the build for formatting in release mode
            if (-not $Release)
            {
                $Success = $false
            }
        }
        Write-Host ""
    }

    #endregion

    #region Clippy

    if ($Clippy -or $All)
    {
        Write-Step "Running Clippy linter..."
        $ClippyArgs = @("clippy")
        if ($BuildFlags.Count -gt 0)
        {
            $ClippyArgs += $BuildFlags
        }
        $ClippyArgs += "--", "-D", "warnings"

        & cargo @ClippyArgs
        if ($LASTEXITCODE -eq 0)
        {
            Write-Success "Clippy passed"
        } else
        {
            Write-Error-Custom "Clippy found issues"
            $Success = $false
        }
        Write-Host ""
    }

    #endregion

    #region Build

    Write-Step "Building project..."
    $BuildArgs = @("build")
    if ($BuildFlags.Count -gt 0)
    {
        $BuildArgs += $BuildFlags
    }

    & cargo @BuildArgs
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Build complete"
    } else
    {
        Write-Error-Custom "Build failed"
        $Success = $false
        exit 1
    }
    Write-Host ""

    #endregion

    #region Test

    if ($Test -or $All)
    {
        Write-Step "Running tests..."
        $TestArgs = @("test")
        if ($BuildFlags.Count -gt 0)
        {
            $TestArgs += $BuildFlags
        }

        & cargo @TestArgs
        if ($LASTEXITCODE -eq 0)
        {
            Write-Success "Tests passed"
        } else
        {
            Write-Error-Custom "Tests failed"
            $Success = $false
        }
        Write-Host ""
    }

    #endregion

    #region Documentation

    if ($Doc -or $All)
    {
        Write-Step "Building documentation..."
        cargo doc --no-deps
        if ($LASTEXITCODE -eq 0)
        {
            Write-Success "Documentation built"
        } else
        {
            Write-Error-Custom "Documentation build failed"
            $Success = $false
        }
        Write-Host ""
    }

    #endregion

    #region Binary Information

    if ($Success)
    {
        Write-Info "=== Build Summary ==="

        $BinPath = "target/$BuildDir"

        if ($IsWindows -or $env:OS -eq "Windows_NT")
        {
            $BinaryPath = Join-Path $BinPath "grok.exe"
        } else
        {
            $BinaryPath = Join-Path $BinPath "grok"
        }

        if (Test-Path $BinaryPath)
        {
            $BinarySize = (Get-Item $BinaryPath).Length
            $SizeMB = [math]::Round($BinarySize / 1MB, 2)
            Write-Info "Binary: $BinaryPath"
            Write-Info "Size: $SizeMB MB"

            # Try to get version
            try
            {
                $VersionOutput = & $BinaryPath --version 2>$null
                if ($LASTEXITCODE -eq 0)
                {
                    Write-Info "Version: $VersionOutput"
                }
            } catch
            {
                # Ignore if binary can't run
            }
        }
        Write-Host ""
    }

    #endregion

    #region Git Tagging and Release

    if ($Success -and ($Tag -or $Auto))
    {
        Write-Step "Processing Git tag and release..."

        $CurrentVersion = Get-CargoVersion
        if (-not $CurrentVersion)
        {
            Write-Error-Custom "Could not read version from Cargo.toml"
            exit 1
        }

        Write-Info "Current version: $CurrentVersion"

        # Determine tag name
        if ($Auto)
        {
            # Auto-increment version
            $NewVersion = Get-NextVersion $CurrentVersion
            if (-not $NewVersion)
            {
                Write-Error-Custom "Could not determine next version"
                exit 1
            }

            $TagName = "v$NewVersion"
            Write-Info "Auto-incrementing to: $NewVersion"

            # Update Cargo.toml
            Write-Step "Updating Cargo.toml version..."
            Set-CargoVersion $NewVersion

            # Commit the version change
            git add Cargo.toml
            git commit -m "chore: bump version to $NewVersion"
            if ($LASTEXITCODE -ne 0)
            {
                Write-Error-Custom "Failed to commit version change"
                exit 1
            }
            Write-Success "Version updated and committed"
        } elseif ($Tag)
        {
            $TagName = $Tag
            if (-not $TagName.StartsWith("v"))
            {
                $TagName = "v$TagName"
            }
        }

        # Check if tag exists
        if (Test-GitTagExists $TagName)
        {
            Write-Error-Custom "Tag '$TagName' already exists"
            Write-Info "Use a different tag name or delete the existing tag:"
            Write-Info "  git tag -d $TagName"
            Write-Info "  git push $Remote :refs/tags/$TagName"
            exit 1
        }

        # Create annotated tag
        Write-Step "Creating Git tag: $TagName"
        $TagMessage = "Release $TagName"
        git tag -a $TagName -m $TagMessage
        if ($LASTEXITCODE -ne 0)
        {
            Write-Error-Custom "Failed to create tag"
            exit 1
        }
        Write-Success "Tag created: $TagName"
        Write-Host ""

        # Push to remote
        if ($Push -or $Auto)
        {
            Write-Step "Pushing to remote: $Remote"

            # Push commits
            Write-Info "Pushing commits to $Branch..."
            git push $Remote $Branch
            if ($LASTEXITCODE -ne 0)
            {
                Write-Error-Custom "Failed to push commits"
                exit 1
            }
            Write-Success "Commits pushed"

            # Push tags
            Write-Info "Pushing tag $TagName..."
            git push $Remote $TagName
            if ($LASTEXITCODE -ne 0)
            {
                Write-Error-Custom "Failed to push tag"
                Write-Warning-Custom "You may need to push manually:"
                Write-Info "  git push $Remote $TagName"
                exit 1
            }
            Write-Success "Tag pushed"
            Write-Host ""

            Write-Success "=== Release Triggered ==="
            Write-Info "GitHub Actions will now build the release for tag: $TagName"
            Write-Info ""
            Write-Info "Monitor the release build at:"
            Write-Info "  https://github.com/microtech/grok-cli/actions"
            Write-Info ""
            Write-Info "Once complete, the release will be available at:"
            Write-Info "  https://github.com/microtech/grok-cli/releases/tag/$TagName"
        } else
        {
            Write-Success "Tag created locally: $TagName"
            Write-Info ""
            Write-Info "To push the tag and trigger the release build, run:"
            Write-Info "  git push $Remote $Branch"
            Write-Info "  git push $Remote $TagName"
            Write-Info ""
            Write-Info "Or re-run this script with -Push:"
            Write-Info "  .\build.ps1 -Release -Tag $TagName -Push"
        }

        Write-Host ""
    }

    #endregion

    #region Success Summary

    if ($Success)
    {
        Write-Host ""
        Write-Success "✨ Build completed successfully! ✨"

        if ($Tag -or $Auto)
        {
            Write-Host ""
            Write-Info "Next steps:"
            if (-not $Push -and -not $Auto)
            {
                Write-Info "1. Push the tag to trigger release:"
                Write-Info "     git push $Remote $TagName"
            }
            Write-Info "2. Monitor GitHub Actions for release build"
            Write-Info "3. Download binaries from GitHub Releases"
        }

        exit 0
    } else
    {
        Write-Host ""
        Write-Error-Custom "Build completed with errors"
        exit 1
    }

    #endregion

} finally
{
    # Always return to original directory
    Pop-Location
}

#endregion
