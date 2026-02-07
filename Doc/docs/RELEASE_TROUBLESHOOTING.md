# Release Troubleshooting Guide

## Issue: Builds Complete But Only Source Code Appears in Releases

### Symptoms
- GitHub Actions workflow completes successfully
- Release is created on GitHub
- Only source code zip files appear (no binaries)
- Expected platform-specific binaries are missing

### Root Causes

#### 1. Artifact Download Path Issue
**Problem**: The `actions/download-artifact@v4` action creates subdirectories for each artifact.

When you upload an artifact named `grok-cli-windows-x86_64`, it gets downloaded into:
```
artifacts/
  grok-cli-windows-x86_64/
    grok-cli-windows-x86_64.zip
```

If your workflow tries to find files directly in `artifacts/*.zip`, it won't find them.

**Solution**: Use `find` command to recursively locate ZIP files:
```yaml
- name: Prepare release assets
  run: |
    mkdir -p release-assets
    find artifacts -name "*.zip" -type f -exec cp {} release-assets/ \;
    ls -lh release-assets/
```

#### 2. Tag Naming Inconsistency
**Problem**: Using unconventional tag formats like `v.0.1.3` instead of `v0.1.3`.

- Standard format: `v0.1.3`, `v1.2.0`
- Problematic format: `v.0.1.3`, `0.1.3`, `ver0.1.3`

**Solution**: Always use the `vMAJOR.MINOR.PATCH` format (e.g., `v0.1.4`).

#### 3. Tag Not Pushed to Remote
**Problem**: Creating tags locally but forgetting to push them.

```bash
# This only creates a local tag
git tag v0.1.4

# GitHub Actions never runs because GitHub doesn't see the tag
```

**Solution**: Always push tags to trigger the workflow:
```bash
git tag v0.1.4
git push origin v0.1.4
```

#### 4. Workflow Conditional Not Met
**Problem**: The release job has a condition that might not be satisfied:
```yaml
release:
  needs: build
  if: startsWith(github.ref, 'refs/tags/')
```

If the workflow is triggered manually (`workflow_dispatch`) instead of by a tag push, this condition fails.

**Solution**: Either push a proper tag or modify the workflow to handle manual triggers.

#### 5. Artifact Retention Expired
**Problem**: Artifacts have a retention period (default: 90 days, workflow sets to 1 day).

```yaml
- name: Upload artifact
  uses: actions/upload-artifact@v4
  with:
    retention-days: 1  # Too short!
```

If the release job runs after artifacts expire, they won't be available.

**Solution**: Set appropriate retention or ensure release job runs immediately after build.

### Debugging Steps

#### Step 1: Check Workflow Logs
1. Go to GitHub repository
2. Click "Actions" tab
3. Find the workflow run for your tag
4. Check both "Build" and "Create GitHub Release" jobs

Look for:
- Did all build jobs complete successfully?
- Did artifacts upload successfully?
- Did the release job run?
- Are there any error messages in "Prepare release assets"?

#### Step 2: Verify Tag Format
```bash
# List all tags
git tag -l

# Check remote tags
git ls-remote --tags origin
```

Ensure tags follow `vX.Y.Z` format.

#### Step 3: Check Artifact Structure
In the workflow logs, look at the "Display structure of downloaded files" step:
```
=== Artifacts directory structure ===
artifacts/
  grok-cli-linux-x86_64/
    grok-cli-linux-x86_64.zip
  grok-cli-macos-x86_64/
    grok-cli-macos-x86_64.zip
  grok-cli-windows-x86_64/
    grok-cli-windows-x86_64.zip
```

If this is empty or different, artifacts weren't downloaded properly.

#### Step 4: Verify Release Assets
In the "Prepare release assets" step, check:
```
=== Release assets prepared ===
-rw-r--r-- 1 runner docker 5.2M Jan 10 12:34 grok-cli-linux-x86_64.zip
-rw-r--r-- 1 runner docker 4.8M Jan 10 12:34 grok-cli-macos-x86_64.zip
-rw-r--r-- 1 runner docker 5.1M Jan 10 12:34 grok-cli-windows-x86_64.zip

=== File count check ===
Found 3 ZIP files
```

If file count is 0, assets weren't prepared correctly.

### Quick Fix Guide

#### Immediate Fix (For Existing Failed Release)

1. **Delete the broken release on GitHub:**
   - Go to repository → Releases
   - Find the release
   - Click "Delete"

2. **Delete the tag locally and remotely:**
   ```bash
   # Delete local tag
   git tag -d v0.1.3
   
   # Delete remote tag
   git push origin :refs/tags/v0.1.3
   ```

3. **Create a new tag with proper format:**
   ```bash
   # Ensure you're on the right commit
   git log --oneline -5
   
   # Create tag
   git tag -a v0.1.4 -m "Release v0.1.4"
   
   # Push tag
   git push origin v0.1.4
   ```

4. **Monitor the workflow:**
   - Go to Actions tab
   - Watch the build and release jobs
   - Verify artifacts are uploaded and attached to release

#### Using the Release Script (Recommended)

Use the provided release helper scripts to avoid manual errors:

**On Windows:**
```powershell
.\scripts\release.ps1 0.1.4
```

**On Linux/macOS/Git Bash:**
```bash
chmod +x scripts/release.sh
./scripts/release.sh 0.1.4
```

The script will:
- ✓ Check for uncommitted changes
- ✓ Validate you're on the right branch
- ✓ Update version in Cargo.toml
- ✓ Run tests
- ✓ Build locally to verify
- ✓ Create properly formatted tag
- ✓ Push everything to GitHub

### Prevention Checklist

Before creating a release:

- [ ] All changes are committed
- [ ] Version in `Cargo.toml` matches release version
- [ ] `CHANGELOG.md` is updated
- [ ] Tests pass locally (`cargo test`)
- [ ] Build succeeds locally (`cargo build --release`)
- [ ] Use proper tag format: `vX.Y.Z`
- [ ] Push tag to remote: `git push origin vX.Y.Z`
- [ ] Monitor GitHub Actions for completion
- [ ] Verify binaries appear in release

### Common Error Messages

#### "No ZIP files found!"
```
=== File count check ===
Found 0 ZIP files
ERROR: No ZIP files found!
```

**Cause**: Artifacts weren't downloaded or path is wrong.

**Fix**: Check earlier steps in logs to see if artifacts downloaded.

#### "fail_on_unmatched_files: No files found"
```
Error: No files found for 'release-assets/*.zip'
```

**Cause**: The glob pattern didn't match any files.

**Fix**: Verify files exist in `release-assets/` directory.

#### "Resource not accessible by integration"
```
Error: Resource not accessible by integration
```

**Cause**: Workflow doesn't have permission to create releases.

**Fix**: Ensure workflow has `contents: write` permission (already in your workflow).

### Workflow Improvements Applied

The updated `release.yml` workflow includes:

1. **Better debugging output:**
   - Shows full directory structure
   - Lists found ZIP files
   - Counts files before proceeding
   - Fails early if no files found

2. **Robust file collection:**
   - Uses `find` to recursively locate ZIPs
   - Handles nested artifact directories
   - Validates files exist before proceeding

3. **Clear error messages:**
   - Explicit checks with meaningful output
   - Early exit on failures
   - Helpful debugging information

### Network Considerations (Starlink)

Since you're using Starlink (which can drop), the workflow already includes:
- Retry mechanisms in actions
- Timeout handling
- Artifact persistence between jobs

If a workflow fails due to network issues:
1. Check if artifacts were uploaded (they persist)
2. Re-run the failed job (not the entire workflow)
3. Artifacts from successful builds will still be available

### Testing the Workflow

To test without creating a real release:

1. **Use workflow_dispatch:**
   - Go to Actions → Build Cross-Platform Binaries
   - Click "Run workflow"
   - This builds binaries but won't create a release (no tag)

2. **Create a test tag:**
   ```bash
   git tag v0.0.0-test
   git push origin v0.0.0-test
   ```
   
3. **Delete test release after verification:**
   - Delete release on GitHub
   - Delete tag: `git push origin :refs/tags/v0.0.0-test`

### Additional Resources

- [GitHub Actions: Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)
- [GitHub Actions: Upload Artifact](https://github.com/actions/upload-artifact)
- [GitHub Actions: Download Artifact](https://github.com/actions/download-artifact)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)

## Summary

The main issue was that `actions/download-artifact@v4` creates subdirectories for each artifact, but the workflow was looking for files directly in the top level. The fix involves using `find` to recursively locate ZIP files and copy them to the release assets directory.

Use the provided release scripts to automate the process and avoid common mistakes!

---

**Author:** John McConnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Support:** https://buymeacoffee.com/micro.tech