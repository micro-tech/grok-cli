# .grok Directory Documentation

This directory contains documentation for the grok-cli project's recent fixes and configuration.

## 📚 Documentation Files

### Quick Start
- **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - Quick reference card for file access fixes and common commands

### Testing & Verification
- **[TESTING_GUIDE.md](TESTING_GUIDE.md)** - Step-by-step testing guide for verifying file access and Zed integration fixes

### Configuration
- **[ENV_CONFIG_GUIDE.md](ENV_CONFIG_GUIDE.md)** - Complete guide for `.env` configuration options

### Technical Summary
- **[COMPLETE_FIX_SUMMARY.md](COMPLETE_FIX_SUMMARY.md)** - Comprehensive summary of all fixes (file access & Zed integration)

## 🎯 What Was Fixed

### File Access with Relative Paths
✅ CLI can now access files using relative paths (`src/main.rs`, `./README.md`, `../file.txt`)
✅ Symlinks are properly resolved
✅ Parent directory access works
✅ Security is maintained

### Zed Editor Integration
✅ Workspace context is extracted from ACP session initialization
✅ grok-cli properly trusts the workspace directory
✅ File operations work within the project context

## 🚀 Getting Started

1. **Configure your model:**
   ```bash
   echo GROK_MODEL=grok-code-fast-1 > .env
   ```

2. **Test file access:**
   ```bash
   grok query "read README.md"
   ```

3. **Configure Zed:**
   See [ENV_CONFIG_GUIDE.md](ENV_CONFIG_GUIDE.md) and `docs/ZED_INTEGRATION.md`

4. **Run tests:**
   Follow [TESTING_GUIDE.md](TESTING_GUIDE.md)

## 📖 Which Doc Should I Read?

- **Just want to get started?** → [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
- **Setting up configuration?** → [ENV_CONFIG_GUIDE.md](ENV_CONFIG_GUIDE.md)
- **Testing the fixes?** → [TESTING_GUIDE.md](TESTING_GUIDE.md)
- **Want full technical details?** → [COMPLETE_FIX_SUMMARY.md](COMPLETE_FIX_SUMMARY.md)

## 📝 Configuration Priority

Settings are loaded in this order (later overrides earlier):
1. Built-in defaults
2. System config (`~/.grok/.env`)
3. Project config (`.grok/.env`) ← This directory
4. Environment variables
5. CLI arguments (highest priority)

## ✅ Verification

```bash
# Check configuration
grok config show

# Should show:
# Model: grok-code-fast-1
# Configuration: Project (.grok/.env) or Hierarchical
```

## 🔒 Security Note

Never commit `.env` files with API keys to git! They're in `.gitignore` by default.

Store API keys in:
- System-wide: `~/.grok/.env`
- Project: `.grok/.env` (gitignored)
- Environment: `export GROK_API_KEY=xai-...`

## 📦 What's in .grok/.env?

Example minimal configuration:
```env
# Model selection
GROK_MODEL=grok-code-fast-1

# API key (optional - better in ~/.grok/.env)
# GROK_API_KEY=xai-your-key-here
```

See [ENV_CONFIG_GUIDE.md](ENV_CONFIG_GUIDE.md) for all available options.

---

**Last Updated:** 2025
**Status:** Ready for Production
**Author:** John McConnell (john.microtech@gmail.com)
**Repository:** https://github.com/microtech/grok-cli