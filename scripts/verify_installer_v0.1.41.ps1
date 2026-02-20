# Installer Verification Script for v0.1.41
# Author: john mcconnell john.microtech@gmail.com
# Repository: https://github.com/microtech/grok-cli
# Purpose: Verify all installer components are correctly updated to v0.1.41

param(
    [switch]$Quick,
    [switch]$Full,
    [switch]$Verbose
)

# Enable strict mode
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:PassCount = 0
$script:FailCount = 0
$script:WarnCount = 0

# Colors for output
function Write-Pass
{
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $Message
    $script:PassCount++
}

function Write-Fail
{
    param([string]$Message)
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host $Message
    $script:FailCount++
}

function Write-Warn2
{
    param([string]$Message)
    Write-Host "⚠ " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
    $script:WarnCount++
}

function Write-Info2
{
    param([string]$Message)
    Write-Host "ℹ " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Section
{
    param([string]$Title)
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan -NoNewline
    Write-Host " " -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
}

# Check if we're in project root
if (-not (Test-Path "Cargo.toml"))
{
    Write-Fail "Must be run from project root directory (where Cargo.toml is)"
    exit 1
}

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Magenta
Write-Host "║  Grok CLI Installer Verification - v0.1.41      ║" -ForegroundColor Magenta
Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Magenta
Write-Host ""

# ============================================================================
# Test 1: Version Numbers
# ============================================================================
Write-Section "1. Version Number Verification"

# Check Cargo.toml
if (Test-Path "Cargo.toml")
{
    $cargoContent = Get-Content "Cargo.toml" -Raw
    if ($cargoContent -match 'version\s*=\s*"0\.1\.41"')
    {
        Write-Pass "Cargo.toml version is 0.1.41"
    } else
    {
        Write-Fail "Cargo.toml version is NOT 0.1.41"
    }
} else
{
    Write-Fail "Cargo.toml not found"
}

# Check package.json
if (Test-Path "package.json")
{
    $packageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
    if ($packageJson.version -eq "0.1.41")
    {
        Write-Pass "package.json version is 0.1.41"
    } else
    {
        Write-Fail "package.json version is $($packageJson.version), expected 0.1.41"
    }

    # Check description includes new features
    if ($packageJson.description -match "external file access|audit logging|tool loop")
    {
        Write-Pass "package.json description mentions new features"
    } else
    {
        Write-Warn2 "package.json description doesn't mention new features"
    }
} else
{
    Write-Fail "package.json not found"
}

# Check installer.rs
if (Test-Path "src/bin/installer.rs")
{
    $installerContent = Get-Content "src/bin/installer.rs" -Raw
    if ($installerContent -match 'v0\.1\.41|0\.1\.41')
    {
        Write-Pass "installer.rs references version 0.1.41"
    } else
    {
        Write-Warn2 "installer.rs may not reference version 0.1.41"
    }
} else
{
    Write-Fail "src/bin/installer.rs not found"
}

# ============================================================================
# Test 2: install.js Network Retry Logic
# ============================================================================
Write-Section "2. npm Installer (install.js) Features"

if (Test-Path "install.js")
{
    $installJsContent = Get-Content "install.js" -Raw

    # Check for retry configuration
    if ($installJsContent -match "RETRY_CONFIG|maxRetries|baseDelay|maxDelay")
    {
        Write-Pass "install.js has retry configuration"
    } else
    {
        Write-Fail "install.js missing retry configuration"
    }

    # Check for exponential backoff
    if ($installJsContent -match "Math\.pow|exponential|backoff")
    {
        Write-Pass "install.js implements exponential backoff"
    } else
    {
        Write-Fail "install.js missing exponential backoff logic"
    }

    # Check for network error detection
    if ($installJsContent -match "ETIMEDOUT|ECONNRESET|ENOTFOUND|network")
    {
        Write-Pass "install.js detects network errors"
    } else
    {
        Write-Fail "install.js missing network error detection"
    }

    # Check for async/await
    if ($installJsContent -match "async function|await ")
    {
        Write-Pass "install.js uses async/await pattern"
    } else
    {
        Write-Fail "install.js not using async/await"
    }

    # Check for version display
    if ($installJsContent -match "0\.1\.41")
    {
        Write-Pass "install.js displays version 0.1.41"
    } else
    {
        Write-Fail "install.js doesn't display version 0.1.41"
    }

    # Check for feature announcement
    if ($installJsContent -match "New features|external file access|audit logging")
    {
        Write-Pass "install.js announces new features"
    } else
    {
        Write-Warn2 "install.js doesn't announce new features"
    }
} else
{
    Write-Fail "install.js not found"
}

# ============================================================================
# Test 3: Windows Installer (installer.rs) Features
# ============================================================================
Write-Section "3. Windows Installer (installer.rs) Features"

if (Test-Path "src/bin/installer.rs")
{
    $installerRsContent = Get-Content "src/bin/installer.rs" -Raw

    # Check for audit directory setup
    if ($installerRsContent -match "setup_audit_directory|\.grok.*audit")
    {
        Write-Pass "installer.rs includes audit directory setup"
    } else
    {
        Write-Fail "installer.rs missing audit directory setup"
    }

    # Check for external_access configuration
    if ($installerRsContent -match "\[external_access\]")
    {
        Write-Pass "installer.rs includes external_access config section"
    } else
    {
        Write-Fail "installer.rs missing external_access config section"
    }

    # Check for enhanced network config
    if ($installerRsContent -match "base_retry_delay|max_retry_delay")
    {
        Write-Pass "installer.rs includes enhanced network config"
    } else
    {
        Write-Fail "installer.rs missing enhanced network config"
    }

    # Check for grok-2-latest model
    if ($installerRsContent -match 'grok-2-latest')
    {
        Write-Pass "installer.rs uses grok-2-latest as default model"
    } else
    {
        Write-Warn2 "installer.rs may not use grok-2-latest as default model"
    }

    # Check for new documentation files
    $requiredDocs = @(
        "EXTERNAL_FILE_ACCESS_SUMMARY.md",
        "EXTERNAL_FILE_REFERENCE.md",
        "PROPOSAL_EXTERNAL_ACCESS.md",
        "TROUBLESHOOTING_TOOL_LOOPS.md",
        "SYSTEM_CONFIG_NOTES.md"
    )

    $foundDocs = 0
    foreach ($doc in $requiredDocs)
    {
        if ($installerRsContent -match [regex]::Escape($doc))
        {
            $foundDocs++
        }
    }

    if ($foundDocs -eq $requiredDocs.Count)
    {
        Write-Pass "installer.rs installs all $($requiredDocs.Count) new documentation files"
    } elseif ($foundDocs -gt 0)
    {
        Write-Warn2 "installer.rs installs $foundDocs of $($requiredDocs.Count) new docs"
    } else
    {
        Write-Fail "installer.rs missing new documentation installation"
    }
} else
{
    Write-Fail "src/bin/installer.rs not found"
}

# ============================================================================
# Test 4: Documentation Files Exist
# ============================================================================
Write-Section "4. New Documentation Files"

$docChecks = @{
    "EXTERNAL_FILE_ACCESS_SUMMARY.md" = "EXTERNAL_FILE_ACCESS_SUMMARY.md"
    "Doc/EXTERNAL_FILE_REFERENCE.md" = "Doc/EXTERNAL_FILE_REFERENCE.md"
    "Doc/PROPOSAL_EXTERNAL_ACCESS.md" = "Doc/PROPOSAL_EXTERNAL_ACCESS.md"
    "Doc/TROUBLESHOOTING_TOOL_LOOPS.md" = "Doc/TROUBLESHOOTING_TOOL_LOOPS.md"
    "Doc/SYSTEM_CONFIG_NOTES.md" = "Doc/SYSTEM_CONFIG_NOTES.md"
}

foreach ($path in $docChecks.Keys)
{
    if (Test-Path $path)
    {
        Write-Pass "$($docChecks[$path]) exists"
    } else
    {
        Write-Fail "$($docChecks[$path]) NOT found at $path"
    }
}

# ============================================================================
# Test 5: Configuration Template
# ============================================================================
Write-Section "5. Configuration Template (config.example.toml)"

if (Test-Path "config.example.toml")
{
    $configExample = Get-Content "config.example.toml" -Raw

    # Check for external_access section (commented or not)
    if ($configExample -match "\[external_access\]")
    {
        Write-Fail "config.example.toml should not have [external_access] - it's in installer only"
    } else
    {
        Write-Pass "config.example.toml correctly excludes [external_access] section"
    }

    # Check for MCP server configuration
    if ($configExample -match "\[mcp\.servers\.|mcp\]")
    {
        Write-Pass "config.example.toml includes MCP server configuration"
    } else
    {
        Write-Warn2 "config.example.toml may be missing MCP configuration"
    }

    # Check for env field requirement
    if ($configExample -match 'env\s*=\s*\{\}|env\s*=\s*\{')
    {
        Write-Pass "config.example.toml shows env field in MCP examples"
    } else
    {
        Write-Warn2 "config.example.toml may not demonstrate env field"
    }

    # Check for max_tool_loop_iterations
    if ($configExample -match "max_tool_loop_iterations\s*=\s*25")
    {
        Write-Pass "config.example.toml sets max_tool_loop_iterations to 25"
    } else
    {
        Write-Warn2 "config.example.toml may not have correct max_tool_loop_iterations"
    }
} else
{
    Write-Fail "config.example.toml not found"
}

# ============================================================================
# Test 6: CHANGELOG Updates
# ============================================================================
Write-Section "6. CHANGELOG Documentation"

if (Test-Path "CHANGELOG.md")
{
    $changelog = Get-Content "CHANGELOG.md" -Raw

    # Check for installer updates mentioned
    if ($changelog -match "Installer Updates|installer.*0\.1\.41")
    {
        Write-Pass "CHANGELOG documents installer updates"
    } else
    {
        Write-Warn2 "CHANGELOG may not document installer updates"
    }

    # Check for network retry mention
    if ($changelog -match "network retry|exponential backoff|Starlink.*retry")
    {
        Write-Pass "CHANGELOG documents network retry improvements"
    } else
    {
        Write-Warn2 "CHANGELOG may not document network retry"
    }

    # Check for audit directory mention
    if ($changelog -match "audit.*directory|\.grok/audit")
    {
        Write-Pass "CHANGELOG documents audit directory setup"
    } else
    {
        Write-Warn2 "CHANGELOG may not document audit directory"
    }
} else
{
    Write-Fail "CHANGELOG.md not found"
}

# ============================================================================
# Test 7: Scripts and Tools
# ============================================================================
Write-Section "7. Supporting Scripts"

$scriptChecks = @{
    "scripts/analyze_tool_loops.ps1" = "Tool loop analyzer script"
    "scripts/test_tool_loop_debug.sh" = "Tool loop debug test script"
    "scripts/update_system_config.ps1" = "System config updater"
}

foreach ($path in $scriptChecks.Keys)
{
    if (Test-Path $path)
    {
        Write-Pass "$($scriptChecks[$path]) exists"
    } else
    {
        Write-Warn2 "$($scriptChecks[$path]) NOT found at $path"
    }
}

# ============================================================================
# Test 8: Build Test (Optional - Only if Full flag)
# ============================================================================
if ($Full)
{
    Write-Section "8. Build Test"

    Write-Info2 "Building release binary to verify..."
    try
    {
        $buildOutput = cargo build --release --bin grok 2>&1
        if ($LASTEXITCODE -eq 0)
        {
            Write-Pass "Release binary builds successfully"

            # Check if binary exists
            if (Test-Path "target/release/grok.exe")
            {
                Write-Pass "grok.exe created in target/release/"

                # Try to get version
                try
                {
                    $versionOutput = & "target/release/grok.exe" --version 2>&1
                    if ($versionOutput -match "0\.1\.41")
                    {
                        Write-Pass "Binary reports version 0.1.41"
                    } else
                    {
                        Write-Warn2 "Binary version output: $versionOutput"
                    }
                } catch
                {
                    Write-Warn2 "Could not get version from binary: $_"
                }
            } else
            {
                Write-Fail "grok.exe not found after build"
            }
        } else
        {
            Write-Fail "Build failed with exit code $LASTEXITCODE"
            if ($Verbose)
            {
                Write-Host $buildOutput
            }
        }
    } catch
    {
        Write-Fail "Build error: $_"
    }
}

# ============================================================================
# Test 9: Summary Documentation
# ============================================================================
Write-Section "9. Update Documentation"

$updateDocs = @{
    ".zed/installer_update_v0.1.41.md" = "Detailed installer update documentation"
    ".zed/INSTALLER_UPDATE_COMPLETE.md" = "Quick reference update summary"
}

foreach ($path in $updateDocs.Keys)
{
    if (Test-Path $path)
    {
        Write-Pass "$($updateDocs[$path]) exists"
    } else
    {
        Write-Warn2 "$($updateDocs[$path]) NOT found at $path"
    }
}

# ============================================================================
# Final Summary
# ============================================================================
Write-Section "Verification Summary"

Write-Host ""
$totalTests = $script:PassCount + $script:FailCount + $script:WarnCount
$passPercent = if ($totalTests -gt 0)
{ [math]::Round(($script:PassCount / $totalTests) * 100, 1) 
} else
{ 0 
}

Write-Host "Total Checks: $totalTests" -ForegroundColor Cyan
Write-Host "  Passed:  $($script:PassCount) " -ForegroundColor Green -NoNewline
Write-Host "($passPercent%)"
Write-Host "  Failed:  $($script:FailCount)" -ForegroundColor $(if ($script:FailCount -gt 0)
    { "Red" 
    } else
    { "Gray" 
    })
Write-Host "  Warnings: $($script:WarnCount)" -ForegroundColor $(if ($script:WarnCount -gt 0)
    { "Yellow" 
    } else
    { "Gray" 
    })
Write-Host ""

if ($script:FailCount -eq 0 -and $script:WarnCount -eq 0)
{
    Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║  ✓ ALL CHECKS PASSED - READY FOR RELEASE        ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Green
    exit 0
} elseif ($script:FailCount -eq 0)
{
    Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Yellow
    Write-Host "║  ⚠ PASSED WITH WARNINGS - REVIEW RECOMMENDED    ║" -ForegroundColor Yellow
    Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Yellow
    exit 0
} else
{
    Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║  ✗ FAILURES DETECTED - FIXES REQUIRED           ║" -ForegroundColor Red
    Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Red
    exit 1
}
