# External File Access - Decision Tree

**Quick Guide:** Answer these questions to find the best solution for your needs.

---

## START HERE ⬇️

### Question 1: What editor are you using?

```
┌─────────────────────────────────────────┐
│  What editor are you using?             │
└─────────────────────────────────────────┘
            │
            ├─── Zed Editor ────────────────→ Go to Q2
            │
            └─── Other (VS Code, etc.) ────→ Go to Q3
```

---

## Q2: Zed Editor Users

### Can you add the external directory as a project folder?

```
┌─────────────────────────────────────────┐
│  Is the external directory a complete   │
│  project or logical workspace?          │
└─────────────────────────────────────────┘
            │
            ├─── YES ──→ ✅ SOLUTION: Multiple Project Roots
            │             
            │             File → Add Folder to Project
            │             Both folders now accessible!
            │             No file system changes needed.
            │
            └─── NO ───→ Go to Q3
```

---

## Q3: How often will you reference these files?

```
┌─────────────────────────────────────────┐
│  How often will you need these files?   │
└─────────────────────────────────────────┘
            │
            ├─── Once or rarely ──────────→ Go to Q4
            │
            ├─── Regularly/frequently ────→ Go to Q5
            │
            └─── Just once, right now ────→ Go to Q6
```

---

## Q4: One-Time or Rare Use

### How big are the files?

```
┌─────────────────────────────────────────┐
│  File size?                              │
└─────────────────────────────────────────┘
            │
            ├─── Small (< 100 lines) ─────→ ✅ SOLUTION: Copy-Paste Content
            │                                 
            │                                 Just paste the content in chat:
            │                                 "Help with this config:
            │                                  [paste content]"
            │
            └─── Large files ─────────────→ ✅ SOLUTION: Copy Files
                                              
                                              Windows:
                                              copy H:\Other\file.txt .\temp-file.txt
                                              
                                              Linux/macOS:
                                              cp /path/to/file.txt ./temp-file.txt
                                              
                                              Add to .gitignore: temp-*
```

---

## Q5: Regular/Frequent Use

### Do you have admin rights on Windows?

```
┌─────────────────────────────────────────┐
│  OS and Admin Status?                    │
└─────────────────────────────────────────┘
            │
            ├─── Linux/macOS (any user) ──→ ✅ SOLUTION: Symbolic Links
            │                                 
            │                                 ln -s /path/to/file ./link-name
            │
            ├─── Windows + Admin ─────────→ ✅ SOLUTION: Symbolic Links
            │                                 
            │                                 PowerShell (as Admin):
            │                                 New-Item -ItemType SymbolicLink `
            │                                   -Path ".\link" -Target "H:\path"
            │
            ├─── Windows + No Admin ──────→ Go to Q7
            │
            └─── Unclear ─────────────────→ Go to Q7
```

---

## Q6: Just Once, Right Now

```
┌─────────────────────────────────────────┐
│  Need the info immediately?              │
└─────────────────────────────────────────┘
            │
            ├─── Small snippet ───────────→ ✅ SOLUTION: Copy-Paste
            │                                 
            │                                 Copy content and paste in chat
            │
            └─── Full file ───────────────→ ✅ SOLUTION: Terminal Command
                                              
                                              Ask AI:
                                              "Can you run this command:
                                               type H:\path\to\file.txt"
                                              
                                              AI will read via terminal
```

---

## Q7: Windows Without Admin Rights

### Can you enable Developer Mode?

```
┌─────────────────────────────────────────┐
│  Can you enable Developer Mode?          │
│  (Settings → For Developers)             │
└─────────────────────────────────────────┘
            │
            ├─── YES ─────────────────────→ ✅ SOLUTION: Symbolic Links
            │                                 
            │                                 1. Enable Developer Mode
            │                                 2. Restart terminal
            │                                 3. Create symlinks (no admin!)
            │                                    New-Item -ItemType SymbolicLink
            │
            ├─── NO (Policy restricted) ──→ Go to Q8
            │
            └─── Unsure ──────────────────→ TRY: Enable Developer Mode
                                              If fails, go to Q8
```

---

## Q8: Windows Without Admin or Dev Mode

### Are you linking directories or files?

```
┌─────────────────────────────────────────┐
│  Linking directories or individual files?│
└─────────────────────────────────────────┘
            │
            ├─── Directories only ────────→ ✅ SOLUTION: Junction Points
            │                                 
            │                                 mklink /J link-name H:\path\to\dir
            │                                 
            │                                 ⚠️ Directories only, no admin needed
            │
            └─── Individual files ────────→ ✅ SOLUTION: Copy Files
                                              
                                              copy H:\Other\*.txt .\
                                              
                                              Create setup script:
                                              setup-external-files.bat
```

---

## Q9: Do the external files change frequently?

```
┌─────────────────────────────────────────┐
│  Do external files change often?         │
└─────────────────────────────────────────┘
            │
            ├─── YES (need auto-sync) ────→ ✅ SOLUTION: Symbolic Links
            │                                 
            │                                 Changes sync automatically
            │                                 No manual updates needed
            │
            └─── NO (stable/static) ──────→ ✅ SOLUTION: Copy Files
                                              
                                              One-time copy is sufficient
                                              Simpler setup
```

---

## Visual Decision Map

```
┌──────────────────────────────────────────────────────────────────┐
│                      EXTERNAL FILE ACCESS                         │
│                    What's your situation?                         │
└──────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
        ┌───────▼──────┐ ┌─────▼─────┐ ┌──────▼──────┐
        │  Using Zed?  │ │  Regular  │ │  One-time   │
        │  Multi-root! │ │   Use?    │ │    Use?     │
        └──────────────┘ └─────┬─────┘ └──────┬──────┘
                               │               │
                    ┌──────────┼──────┐       │
                    │          │      │       │
            ┌───────▼──┐  ┌────▼──┐  │  ┌────▼────┐
            │ Symlinks │  │ Copy  │  │  │  Paste  │
            │  (Best)  │  │ Files │  │  │ Content │
            └──────────┘  └───────┘  │  └─────────┘
                                     │
                            ┌────────▼────────┐
                            │ Windows no admin?│
                            │ → Developer Mode │
                            │ → Junctions      │
                            └──────────────────┘
```

---

## Quick Reference Table

| Your Situation | Best Solution | Difficulty |
|----------------|---------------|------------|
| Zed editor + logical workspace | Multiple Project Roots | ⭐ Easy |
| Regular use + admin rights | Symbolic Links | ⭐⭐ Medium |
| Regular use + no admin | Developer Mode → Symlinks | ⭐⭐ Medium |
| Windows + directories only | Junction Points | ⭐⭐ Medium |
| One-time + small file | Copy-Paste Content | ⭐ Easy |
| One-time + large file | Copy Files or Terminal | ⭐ Easy |
| Immediate info needed | Terminal Command | ⭐⭐ Medium |
| Files change frequently | Symbolic Links | ⭐⭐ Medium |
| Files rarely change | Copy Files | ⭐ Easy |
| Can't use any method | Wait for future feature | ⏳ Pending |

---

## Solution Details

### 🏆 Symbolic Links (Most Versatile)

**When to use:**
- Regular/frequent access needed
- Files change and need auto-sync
- Have admin rights OR Developer Mode enabled

**Command:**
```powershell
# Windows
New-Item -ItemType SymbolicLink -Path ".\ext-file.txt" -Target "H:\Other\file.txt"

# Linux/macOS
ln -s /path/to/file ./ext-file
```

**Pros:** ✅ Auto-sync, ✅ No duplication, ✅ Original location  
**Cons:** ❌ Requires admin (Windows) or Dev Mode

---

### 📋 Copy Files (Simplest)

**When to use:**
- One-time or infrequent access
- Files don't change often
- No admin rights available

**Command:**
```bash
# Windows
copy H:\Other\file.txt .\temp-file.txt

# Linux/macOS
cp /path/to/file ./temp-file
```

**Pros:** ✅ Simple, ✅ No admin needed, ✅ Works everywhere  
**Cons:** ❌ Manual sync, ❌ Duplication

---

### 💬 Copy-Paste Content (Fastest)

**When to use:**
- Small snippets (< 100 lines)
- Immediate one-time need
- Don't want file system changes

**How:**
Just paste content directly in chat with context.

**Pros:** ✅ Instant, ✅ Zero setup, ✅ No files created  
**Cons:** ❌ Only for small content, ❌ Not reusable

---

### 💻 Terminal Command (Quick Check)

**When to use:**
- Need to peek at file contents
- One-time verification
- Don't want to create links/copies

**How:**
Ask AI: "Can you run: `type H:\path\to\file.txt`"

**Pros:** ✅ Bypasses restrictions, ✅ No file changes  
**Cons:** ❌ Less structured, ❌ Manual each time

---

### 📂 Multiple Project Roots (Zed Only)

**When to use:**
- Using Zed editor
- External directory is a logical project
- Want clean workspace

**How:**
File → Add Folder to Project

**Pros:** ✅ Natural, ✅ No file changes, ✅ Clean  
**Cons:** ❌ Zed-specific only

---

### 🔧 Junction Points (Windows Fallback)

**When to use:**
- Windows without admin or Dev Mode
- Linking directories (not individual files)
- Need persistent access

**Command:**
```cmd
mklink /J link-name H:\path\to\directory
```

**Pros:** ✅ No admin needed, ✅ Works for directories  
**Cons:** ❌ Directories only, ❌ Windows-specific

---

## Common Scenarios

### Scenario 1: Team Shared Config Files
**Situation:** ESLint/TSConfig shared across team projects  
**Solution:** Symbolic Links with setup script  
**Why:** Regular use, auto-sync needed, documents setup for team

---

### Scenario 2: Quick API Reference Check
**Situation:** Need to reference API spec from another project  
**Solution:** Terminal Command  
**Why:** One-time check, don't need persistent access

---

### Scenario 3: Corporate Laptop (Restricted)
**Situation:** No admin rights, can't enable Dev Mode  
**Solution:** Junction Points (for dirs) or Copy Files (for files)  
**Why:** Only available options without admin

---

### Scenario 4: Working in Zed on Related Projects
**Situation:** Frontend and backend in separate directories  
**Solution:** Multiple Project Roots  
**Why:** Zed native feature, cleanest approach

---

### Scenario 5: Getting Quick Help on Config Snippet
**Situation:** 10-line config causing issues  
**Solution:** Copy-Paste Content  
**Why:** Fastest, no file management needed

---

## Still Not Sure?

### Default Recommendation

**Try this order:**

1. **If using Zed:** Try Multiple Project Roots
2. **If admin rights:** Try Symbolic Links
3. **If Windows no admin:** Try Developer Mode → Symlinks
4. **If that fails:** Try Junction Points (directories) or Copy Files
5. **If just once:** Use Copy-Paste or Terminal Command

---

## Future Solution

⏳ **Configurable External Access** (Proposed)

Will allow configuration-based external directory access:
```toml
[security.external_access]
enabled = true
allowed_paths = ["H:\\GitHub\\shared", "H:\\Docs"]
require_approval = true
```

See `Doc/PROPOSAL_EXTERNAL_ACCESS.md` for details.

---

## Need Help?

📄 **Full Documentation:**
- Quick Ref: `.zed/EXTERNAL_FILES_QUICK_REF.md`
- Complete Guide: `Doc/EXTERNAL_FILE_REFERENCE.md`
- Summary: `EXTERNAL_FILE_ACCESS_SUMMARY.md`

💬 **Get Support:**
- GitHub Issues: https://github.com/microtech/grok-cli/issues
- Email: john.microtech@gmail.com

---

**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Buy me a coffee:** https://buymeacoffee.com/micro.tech