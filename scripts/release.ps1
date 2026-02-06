# Release helper script for grok-cli (PowerShell version for Windows)
# Author: John McConnell john.microtech@gmail.com
# Repository: https://github.com/microtech/grok-cli

param(
    [Parameter(Position=0)]
    [string]$Version
)

# Enable strict mode
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info
{
    param([string]$Message)
    Write-Host "ℹ $Message" -ForegroundColor Blue
}

function Write-Success
{
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Warning2
{
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Write-Error2
{
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Check if we're in the project root
if (-not (Test-Path "Cargo.toml"))
{
    Write-Error2 "Must be run from the project root directory"
    exit 1
}

# Get current version from Cargo.toml
$CargoContent = Get-Content "Cargo.toml" -Raw
if ($CargoContent -match 'version\s*=\s*"([^"]+)"')
{
    $CurrentVersion = $Matches[1]
    Write-Info "Current version in Cargo.toml: $CurrentVersion"
} else
{
    Write-Error2 "Could not parse version from Cargo.toml"
    exit 1
}

# Check if version is provided as argument
if ([string]::IsNullOrEmpty($Version))
{
    Write-Info "Usage: .\scripts\release.ps1 <version>"
    Write-Info "Example: .\scripts\release.ps1 0.1.4"
    Write-Host ""
    Write-Warning2 "No version specified. Using current version: v$CurrentVersion"
    $Version = $CurrentVersion
} else
{
    # Remove 'v' prefix if provided
    $Version = $Version -replace '^v', ''
}

$TagName = "v$Version"

Write-Info "Preparing release: $TagName"
Write-Host ""

# Check for uncommitted changes
$GitStatus = git status --porcelain
if ($GitStatus)
{
    Write-Error2 "You have uncommitted changes. Please commit or stash them first."
    git status --short
    exit 1
}

# Check if we're on the right branch
$CurrentBranch = git rev-parse --abbrev-ref HEAD
Write-Info "Current branch: $CurrentBranch"

if ($CurrentBranch -ne "master" -and $CurrentBranch -ne "main")
{
    Write-Warning2 "You're not on master/main branch. Continue? (y/N)"
    $Response = Read-Host
    if ($Response -notmatch '^[Yy]$')
    {
        Write-Info "Release cancelled"
        exit 0
    }
}

# Check if tag already exists locally
$LocalTagExists = git rev-parse $TagName 2>$null
if ($LASTEXITCODE -eq 0)
{
    Write-Warning2 "Tag $TagName already exists locally"
    Write-Info "Delete and recreate? (y/N)"
    $Response = Read-Host
    if ($Response -match '^[Yy]$')
    {
        git tag -d $TagName
        Write-Success "Local tag deleted"
    } else
    {
        Write-Error2 "Cannot continue with existing tag"
        exit 1
    }
}

# Check if tag exists on remote
$RemoteTags = git ls-remote --tags origin
if ($RemoteTags -match "refs/tags/$TagName")
{
    Write-Warning2 "Tag $TagName already exists on remote"
    Write-Info "Delete remote tag and recreate? (y/N)"
    $Response = Read-Host
    if ($Response -match '^[Yy]$')
    {
        git push origin ":refs/tags/$TagName"
        Write-Success "Remote tag deleted"
    } else
    {
        Write-Error2 "Cannot continue with existing tag"
        exit 1
    }
}

# Update version in Cargo.toml if different
if ($Version -ne $CurrentVersion)
{
    Write-Info "Updating Cargo.toml version to $Version"
    $CargoContent = Get-Content "Cargo.toml" -Raw
    $CargoContent = $CargoContent -replace 'version\s*=\s*"[^"]+"', "version = `"$Version`""
    Set-Content -Path "Cargo.toml" -Value $CargoContent -NoNewline

    # Commit the version change
    git add Cargo.toml
    git commit -m "Bump version to $Version"
    Write-Success "Version updated and committed"
}

# Update CHANGELOG.md reminder
Write-Warning2 "Have you updated CHANGELOG.md for this release? (y/N)"
$Response = Read-Host
if ($Response -notmatch '^[Yy]$')
{
    Write-Info "Please update CHANGELOG.md and run this script again"
    exit 0
}

# Run tests
Write-Info "Running tests..."
$TestOutput = cargo test --quiet 2>&1
if ($LASTEXITCODE -eq 0)
{
    Write-Success "Tests passed"
} else
{
    Write-Error2 "Tests failed. Fix tests before releasing."
    Write-Host $TestOutput
    exit 1
}

# Build release binaries locally to verify
Write-Info "Building release binary locally..."
$BuildOutput = cargo build --release --quiet 2>&1
if ($LASTEXITCODE -eq 0)
{
    Write-Success "Local build successful"
} else
{
    Write-Error2 "Build failed. Fix build errors before releasing."
    Write-Host $BuildOutput
    exit 1
}

# Create the tag
Write-Info "Creating tag $TagName"
git tag -a $TagName -m "Release $TagName"
Write-Success "Tag created"

# Push changes
Write-Info "Pushing changes to origin..."
git push origin $CurrentBranch
if ($LASTEXITCODE -ne 0)
{
    Write-Error2 "Failed to push changes"
    exit 1
}
Write-Success "Changes pushed"

# Push tag
Write-Info "Pushing tag to origin..."
git push origin $TagName
if ($LASTEXITCODE -ne 0)
{
    Write-Error2 "Failed to push tag"
    exit 1
}
Write-Success "Tag pushed"

Write-Host ""
Write-Success "Release $TagName initiated!"
Write-Host ""
Write-Info "GitHub Actions will now build binaries for:"
Write-Host "  - Linux (x86_64)"
Write-Host "  - macOS (x86_64)"
Write-Host "  - Windows (x86_64)"
Write-Host ""
Write-Info "Check the GitHub Actions workflow at:"
Write-Host "  https://github.com/microtech/grok-cli/actions"
Write-Host ""
Write-Info "Once complete, the release will be available at:"
Write-Host "  https://github.com/microtech/grok-cli/releases/tag/$TagName"
Write-Host ""
Write-Success "Done! 🚀"
