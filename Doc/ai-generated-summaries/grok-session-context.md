## User

lets look at skills!  there is a agent skills page here "https://agentskills.io/home" gohead and research skill and then build a task list  to add skills then do the tasks
@rule

## Assistant

<thinking>
**Considering Agent Skills**

I've begun exploring agentskills.io. Initial impressions suggest a comprehensive listing of agent capabilities. My next step will be extracting a task list to integrate these skills into the grok-cli project. The goal is to identify and prioritize the most relevant skills for implementation.


**Fetching Website Content**

I've successfully retrieved the content from the agentskills.io website. Now I'm focused on parsing this data to understand the skill classifications and how they relate to the potential functionality within grok-cli. After a careful review, I'll move on to a preliminary analysis of the grok-cli code structure.



</thinking>

**Tool Call: Processing URLs and instructions from prompt: "Summarize the content of https://agentskills.io/home. Focus on understanding what "agent skills" ..."**
Status: Completed

Content processed from prompt.

## Assistant

<thinking>
**Exploring Agent Skills Structure**

I'm currently delving into the specifics of "Agent Skills". It looks like they're essentially organized folders, following the Anthropic model, containing crucial instructions, scripts, and supporting resources. My immediate focus is on the `SKILL.md` file. I'm keen to understand its precise role and internal structure for a comprehensive overview.


**Pinpointing Skill File Details**

I've been examining the `SKILL.md` file within the agent skills structure. I've now identified relevant information, including the `SKILL.md` location, within the available skills XML structure of my system prompt, which helps me to refine understanding of its internal organization. I also see an `activate_skill` tool and `available_skills` in my system prompt, which points to my agent supporting these skills directly.


**Defining Skill Implementation Strategy**

I'm now formulating a plan for the implementation. My system prompt hints at a basic structure. I'll search the codebase for "skill" to gauge any existing concepts. Next, a web search for the "Agent Skills format spec" is next to define the layout, especially `SKILL.md`. Finally, I'll define the directory structure, implement `SKILL.md` file reading, and integrate skill activation. I need to handle activation using the `activate_skill` tool, as indicated by the system prompt.


**Outlining Implementation Roadmap**

I'm solidifying my approach now. I've re-examined the context and system prompt, confirming the agent skill's structure. I'll search the codebase for relevant keywords like "skill" to understand the existing context. After that, I'll identify the formal specifications, most likely around `SKILL.md` file structure. Finally, I'll then create a directory structure, create a way to read `SKILL.md` files, and activate skills with appropriate prompt integration.



</thinking>

**Tool Call: 'skill' within ./**
Status: Completed

No matches found

**Tool Call: Processing URLs and instructions from prompt: "Get all links from https://agentskills.io/home and summarize what "Agent Skills" are. Also list a..."**
Status: Completed

Content processed from prompt.

## Assistant

<thinking>
**Defining the Specification**

I've homed in on the specification at `/specification` as the crucial starting point. My next step involves fetching this file. Analyzing its contents will provide the definitive blueprint for the project's structure and the exact files I need to create. This will guide my implementation plan.


**Analyzing the Blueprint**

I've successfully identified the `/specification` file as the crucial first step. Now, I plan to retrieve it. Afterwards, the goal is to pinpoint the exact directory structure and files to be implemented. I will then devise an execution plan that includes a `skills` directory and functionalities for skill discovery, reading instructions, injecting them into the prompt, and possibly a skill creation command.



</thinking>

**Tool Call: Processing URLs and instructions from prompt: "Get the content of https://agentskills.io/specification to understand the file structure and form..."**
Status: Completed

Content processed from prompt.

## Assistant

<thinking>
**Structuring the Skills**

\n\n

I'm solidifying the skill directory structure. The `SKILL.md` file will use YAML frontmatter for metadata, including a name, description, and optional fields like license and compatibility. Markdown instructions will be the core content. Optional directories like `scripts`, `references`, and `assets` will provide flexibility.


**Outlining the Implementation**

\n\n

My current thinking is focused on turning the skill specification into actionable steps. I've distilled the requirements for the `grok-cli` integration into a task list. I'm prioritizing the core functionality: skill discovery, loading, listing, and a way to activate them. Then I can consider the creation of a skill from the template. The specifics for context integration are still a bit hazy, so I will need to iterate.


**Clarifying the Tasks**

\n\n

My current thinking is focused on concretizing the implementation details. I've broken down the project into a comprehensive task list, detailing the directory structure, struct definitions, discovery functions, CLI commands, and context integration. I've re-examined the `grok-cli` file structure, deciding `src/skills/` is the best location. Now, I need to verify dependencies, primarily `serde` and `serde_yaml`. I'll start by checking the `Cargo.toml` file.



</thinking>

**Tool Call: Cargo.toml**
Status: Completed

## Assistant

<thinking>
**Evaluating Dependencies**

I'm leaning towards adding `serde_yaml` to the project. The spec requires parsing YAML frontmatter in `SKILL.md`. I could parse it manually, but that feels brittle. The project already uses `serde` with `toml` and `serde_json`, which supports the 'popular Rust crates' convention. Adding `serde_yaml` aligns with existing practices.



</thinking>

**Tool Call: 'yaml' within ./**
Status: Completed

Found 11 matches

## Assistant

<thinking>
**Examining YAML Integration**

I'm currently investigating the YAML implementation. I've noted a discrepancy: the codebase references YAML but lacks a corresponding library import. My immediate task is to introduce `serde_yaml` to the dependencies. Following that, I'll structure a new module to handle skills, encompassing the `Skill` struct definition and parsing logic, alongside a manager for overall skills management.


**Scoping the Context Builder**

I've located the context builder in `src/utils/context.rs`, just as anticipated. This confirms the point of integration for active skills. My next step will be to implement the skill module, starting with dependency addition and struct definition, which is the immediate priority.



</thinking>

**Tool Call: src\utils\context.rs**
Status: Completed

## Assistant

<thinking>
**Defining Skill Loading Strategy**

I'm leaning towards a dedicated `src/skills/` module for skill loading. The initial plan involves creating a separate module to handle this aspect, as it seems a more organized approach. The goal is to load skills independently and then incorporate them into the prompt generation process, which will likely take place within `src/utils/context.rs`. Now, I must check the `src/cli/commands/chat.rs` file.


**Investigating Contextual Integration**

I've decided to refine the skill loading integration. Instead of directly merging skill loading with static context loading, I will create a dedicated `src/skills/` module. I am now exploring how `load_project_context` is invoked; I need to find the specific files where this function is called, starting with `src/cli/commands/chat.rs` and `src/cli/app.rs`, to understand the best place to call my skill loader.



</thinking>

**Tool Call: 'load_project_context' within ./**
Status: Completed

Found 14 matches

## Assistant

<thinking>
**Reviewing Interactive Module**

I'm now examining `src/display/interactive.rs`. I'm especially focused on the `load_project_context_for_session` function. My objective is to fully understand its purpose and how it functions. This should give me a solid foundation for further exploration.



</thinking>

**Tool Call: src\display\interactive.rs**
Status: Completed

## Assistant

<thinking>
**Planning Session Structuring**

I'm structuring the `InteractiveSession` struct to hold the `system_prompt`. The next step is to get the project context loaded using `load_project_context_for_session`. Now, I will need to implement the `skills` module to load and track active skills within `InteractiveSession` or `InteractiveConfig`.


**Integrating Skill Logic**

I've revised my task list. First, I'll add `serde_yaml`. Then, the `skills` module, complete with a `config.rs` for skill configurations. I need to get the logic for loading and listing skills in place before I build the CLI command, specifically in `src/cli/commands/skills.rs`. Finally, I'll need to integrate this throughout the program.


**Prioritizing Skill Implementation**

I've streamlined the task list, focusing on skill implementation. First, I'm adding `serde_yaml` to `Cargo.toml`. Next, I'll create the `src/skills/` module with `config.rs`, `manager.rs`, and `mod.rs`. After that, implementing `load_skill` and `list_skills` will be the priority before implementing the CLI command. I'll need to remember to integrate skills into interactive mode and construct the prompt appropriately.



</thinking>

**Tool Call: cargo add serde_yaml [current working directory H:\GitHub\grok-cli]**
Status: Completed

    Updating crates.io index
      Adding serde_yaml v0.9.34 to dependencies
    Updating crates.io index
     Locking 2 packages to latest Rust 1.94.0-nightly compatible versions
      Adding serde_yaml v0.9.34+deprecated
      Adding unsafe-libyaml v0.2.11

## Assistant

<thinking>
**Crafting Skill Modules**

I'm currently building the `skills` module. I've just laid out the basic structure and am now focusing on `src/skills/config.rs`. The YAML format outlined in the specification will be the blueprint for the `SkillConfig` struct. I am now translating that into Rust code.



</thinking>

**Tool Call: mkdir -p src/skills [current working directory H:\GitHub\grok-cli]**
Status: Completed



    Directory: H:\GitHub\grok-cli\src


Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
d-----          2/5/2026  10:07 PM                skills

## Assistant

<thinking>
**Building Configuration File**

I've started by defining the file path for the configuration settings and am focusing on building the `src/skills/config.rs` file. This is where I'll manage the foundational components required for the skills module.



</thinking>

**Tool Call: Writing to src\skills\config.rs**
Status: Completed

Diff: config.rs
```
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill configuration (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Unique name of the skill
    pub name: String,
    
    /// Description of what the skill does
    pub description: String,
    
    /// License for the skill (optional)
    pub license: Option<String>,
    
    /// Compatibility information (optional)
    pub compatibility: Option<Vec<String>>,
    
    /// Additional metadata (optional)
    pub metadata: Option<HashMap<String, String>>,
    
    /// Allowed tools for this skill (optional)
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,
}

/// Skill structure combining configuration and instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Configuration from frontmatter
    pub config: SkillConfig,
    
    /// Instructions from markdown content
    pub instructions: String,
    
    /// Path to the skill directory
    pub path: std::path::PathBuf,
}

```


## Assistant

<thinking>
**Developing Skill Loading Functions**

I'm currently working on implementing the loading functions for skills within `src/skills/manager.rs`. Specifically, I'm focusing on creating `load_skill()` to handle individual skill loading, and `list_skills()` to enumerate skills from a given directory. The validation logic using `validate_skill()` is also being considered.


**Constructing the Loading Process**

I've been mapping out the specifics of skill loading now. My intention is to use `walkdir` to recursively locate `SKILL.md` files, which will then be loaded. I'm focusing on defining `load_skill()` and `list_skills()` within `src/skills/manager.rs`, ensuring `validate_skill()` is used for checking the integrity of loaded skills. I'm planning to use the `walkdir` crate, which is already in `Cargo.toml`.



</thinking>

**Tool Call: Writing to src\skills\manager.rs**
Status: Completed

Diff: manager.rs
```
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::skills::config::{Skill, SkillConfig};

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        anyhow::bail!("No SKILL.md found in {}", skill_dir.display());
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content
    // Frontmatter is between '---' lines at the start
    let mut parts = content.splitn(3, "---\n");
    
    // First part is empty (before the first ---)
    let _ = parts.next();
    
    let frontmatter = parts.next().context("Missing frontmatter in SKILL.md")?;
    let instructions = parts.next().context("Missing instructions in SKILL.md")?;

    let config: SkillConfig = serde_yaml::from_str(frontmatter)
        .with_context(|| format!("Failed to parse YAML frontmatter in {}", skill_file.display()))?;

    Ok(Skill {
        config,
        instructions: instructions.trim().to_string(),
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    for entry in WalkDir::new(base_dir).min_depth(1).max_depth(2).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let skill_file = entry.path().join("SKILL.md");
            if skill_file.exists() {
                match load_skill(entry.path()) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => eprintln!("Warning: Failed to load skill at {}: {}", entry.path().display(), e),
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}

```


## Assistant

<thinking>
**Revising Skill Loading Logic**

I'm currently wrestling with the skill loading mechanism in `src/skills/manager.rs`. The current implementation using `splitn(3, "---\n")` seems fragile. It incorrectly assumes the delimiter `---` will always be at the beginning of the skill file, based on the `SKILL.md` format. This format is as described, so I must fix the loading algorithm.


**Evaluating Alternative Parsing Strategies**

I'm now contemplating whether to switch away from `splitn` altogether. The current reliance on it feels brittle. After further analysis, it seems better to manually slice the string or utilize a proper frontmatter parser. Testing is essential, but I lean toward keeping `splitn` and improving its resilience. The main objective is to handle the frontmatter and content sections reliably.


**Finalizing Skill File Parsing**

My current thinking leans towards a more robust parsing strategy. I'm moving away from `splitn` and aiming for a combination of string slicing and manual parsing to extract frontmatter and content. After carefully reviewing the parsing logic, I've decided to incorporate a regex-based or manual parse-based approach to accurately find the end of the frontmatter and extract relevant portions. This should provide greater resilience to variations in skill file formatting, so I am going to update the `src/skills/manager.rs` module. Also, the `mod.rs` must be created.



</thinking>

**Tool Call: Writing to src\skills\manager.rs**
Status: Completed

Diff: manager.rs
```
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::skills::config::SkillConfig;

/// Struct representing a loaded skill
#[derive(Debug, Clone)]
pub struct Skill {
    pub config: SkillConfig,
    pub instructions: String,
    pub path: PathBuf,
}

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(anyhow!("No SKILL.md found in {}", skill_dir.display()));
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content manually to be robust
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of the frontmatter
    // We look for the second "---" after the first one
    let end_fm_idx = content[3..].find("\n---")
        .map(|i| i + 3) // Add back the offset
        .ok_or_else(|| anyhow!("Could not find end of frontmatter in SKILL.md"))?;

    let frontmatter = &content[3..end_fm_idx].trim();
    
    // The instructions start after the second "---"
    // end_fm_idx points to the start of the second "---"
    // The second "---" has length 3, plus usually a newline
    let instructions_start = end_fm_idx + 4; // +4 for "\n---" length if we found via "\n---"
    
    // Check if we have enough content
    let instructions = if instructions_start < content.len() {
        content[instructions_start..].trim().to_string()
    } else {
        String::new()
    };

    let config: SkillConfig = serde_yaml::from_str(frontmatter)
        .with_context(|| format!("Failed to parse YAML frontmatter in {}", skill_file.display()))?;

    Ok(Skill {
        config,
        instructions,
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    // We only look at immediate subdirectories of the skills folder
    for entry in WalkDir::new(base_dir).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            match load_skill(entry.path()) {
                Ok(skill) => skills.push(skill),
                Err(_) => {
                    // Silently ignore directories that aren't skills or are malformed
                    // unless verbose logging is on (which we don't have easy access to here)
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}
```


## Assistant

<thinking>
**Refactoring Skills Definitions**

I'm currently working to consolidate the `Skill` struct definition. My initial thought was to import it, but then realized I'd inadvertently redefined it in `src/skills/manager.rs`, which is causing some confusion. I'm now going to remove the redundant definition to clear things up.


**Updating Skill Definition Source**

I'm now going to remove the duplicate `Skill` struct definition from `src/skills/manager.rs`, resolving the import issue. The plan is to use the definition from the `config` module, which should streamline the code. I am also working on how this affects function signatures, like the return type of `load_skill`, to ensure consistency. This should provide greater clarity.



</thinking>

**Tool Call: src\skills\manager.rs: use anyhow::{Context, Result, ... => use anyhow::{Context, Result, ...**
Status: Failed

Failed to edit, 0 occurrences found for old_string in H:\GitHub\grok-cli\src\skills\manager.rs. Ensure you're not escaping content incorrectly and check whitespace, indentation, and context. Use read_file tool to verify.

## Assistant

<thinking>
**Recalling Skill Loading**

I'm revisiting the details of how `load_skill` works. Specifically, I need to check `src/skills/manager.rs` to ensure my understanding is completely accurate. Some minor uncertainty has crept in, and a quick re-read should clear things up.



</thinking>

**Tool Call: src\skills\manager.rs**
Status: Completed

## Assistant

<thinking>
**Investigating Redefinition**

I've noted a potential conflict: the `Skill` struct is being redefined locally, even though `SkillConfig` is imported. This suggests a potential shadowing issue that warrants further investigation. I'm focusing on the `config.rs` file to see how this impacts the overall structure and if it leads to unintended behavior within the `manager`.


**Analyzing Simplification Strategy**

I'm now revising the `manager.rs` file. My initial concern was about redundant struct definitions and the potential for shadowing. I've decided to remove the local `Skill` definition and instead import it directly from `config.rs`. The `SkillConfig` import and usage within `load_skill` remains unaffected. The `PathBuf` issue has also been resolved in my analysis and will not require modification to other files. Rewriting the manager file ensures safety and a cleaner approach.



</thinking>

**Tool Call: Writing to src\skills\manager.rs**
Status: Completed

Diff: manager.rs
```
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::skills::config::{Skill, SkillConfig};

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(anyhow!("No SKILL.md found in {}", skill_dir.display()));
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content manually to be robust
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of the frontmatter
    // We look for the second "---" after the first one
    let end_fm_idx = content[3..]
        .find("\n---")
        .map(|i| i + 3) // Add back the offset
        .ok_or_else(|| anyhow!("Could not find end of frontmatter in SKILL.md"))?;

    let frontmatter = &content[3..end_fm_idx].trim();

    // The instructions start after the second "---"
    // end_fm_idx points to the start of the second "---"
    // The second "---" has length 3, plus usually a newline
    let instructions_start = end_fm_idx + 4; // +4 for "\n---" length if we found via "\n---"

    // Check if we have enough content
    let instructions = if instructions_start < content.len() {
        content[instructions_start..].trim().to_string()
    } else {
        String::new()
    };

    let config: SkillConfig = serde_yaml::from_str(frontmatter).with_context(|| {
        format!(
            "Failed to parse YAML frontmatter in {}",
            skill_file.display()
        )
    })?;

    Ok(Skill {
        config,
        instructions,
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    // We only look at immediate subdirectories of the skills folder
    for entry in WalkDir::new(base_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            if entry.path().join("SKILL.md").exists() {
                match load_skill(entry.path()) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        // Silently ignore or log warning if possible
                        // eprintln!("Warning: Failed to load skill at {}: {}", entry.path().display(), e);
                    }
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}
```


## Assistant

<thinking>
**Building the Skills Module**

I'm now focused on the foundational structure. The immediate task is to create the `src/skills/mod.rs` file. This is the first tangible step in establishing the skill framework. It's a key structural element.



</thinking>

**Tool Call: Writing to src\skills\mod.rs**
Status: Completed

Diff: mod.rs
```
pub mod config;
pub mod manager;

// Re-export common types
pub use config::{Skill, SkillConfig};
pub use manager::{load_skill, list_skills, find_skill, get_default_skills_dir};

```


## Assistant

<thinking>
**Checking Module Registration**

I'm currently focused on integrating the `skills` module. My next step is to register it within either `src/lib.rs` or `src/main.rs`. I'm leaning towards `src/lib.rs` for modularity, but I'll examine both files to confirm before proceeding with the registration.



</thinking>

**Tool Call: src\lib.rs**
Status: Completed

## Assistant

<thinking>
**Adding a module**

I've just added `pub mod skills;` to `src/lib.rs`. This should help me organize the codebase better, as I plan to create multiple skill definitions. My next step will be to define the skills module.



</thinking>

**Tool Call: src\lib.rs: pub mod acp;... => pub mod acp;...**
Status: Completed

Diff: lib.rs
```
//! Grok CLI Library
//!
//! This library provides the core functionality for the Grok CLI,
//! including API integration, configuration management, and display utilities.
//!
//! # Architecture Notes
//!
//! ## Library vs Binary Separation
//!
//! This crate contains both library and binary code. According to Rust best practices,
//! libraries should NOT contain:
//! - Terminal I/O operations (println!, eprintln!, print!, etc.)
//! - Progress bars (indicatif)
//! - Terminal UI (ratatui, crossterm)
//! - Direct runtime dependencies (#[tokio::main])
//! - Code that exits the process
//!
//! ## Current State
//!
//! The following modules currently violate library/binary separation:
//! - `cli::mod` - Contains I/O helper functions (marked deprecated)
//! - `cli::commands::*` - Command handlers print directly to stdout/stderr
//! - `display` - Some functions perform direct I/O
//!
//! ## Migration Path
//!
//! 1. ✅ Created `src/terminal/` module for binary-only I/O (not exposed in lib.rs)
//! 2. ⏳ TODO: Refactor command handlers to return Result<DisplayData> instead of printing
//! 3. ⏳ TODO: Move `cli::app` and command dispatch to binary crate
//! 4. ⏳ TODO: Make all `display` functions pure (return String, no I/O)
//!
//! For now, I/O functions are marked with `#[deprecated]` to indicate they should
//! be moved to the binary crate in a future refactor.

use clap::Subcommand;

pub mod acp;
pub mod cli;
pub mod config;
pub mod display;
pub mod grok_client_ext;
pub mod hooks;
pub mod mcp;
pub mod skills;
pub mod utils;

// Re-export grok_api types for use throughout the crate
pub use grok_api::{
    ChatResponse, Choice, Error as GrokApiError, FunctionCall, Message, ToolCall, Usage,
};

// Re-export the extended GrokClient
pub use grok_client_ext::GrokClient;

/// Helper function to extract text content from String
/// Kept for backwards compatibility with refactored code
pub fn extract_text_content(content: &str) -> String {
    content.to_string()
}

/// Helper function to convert Option<String> to String
/// Kept for backwards compatibility with refactored code
pub fn content_to_string(content: Option<&String>) -> String {
    content.cloned().unwrap_or_default()
}

/// Helper function to create text content
/// Kept for backwards compatibility with refactored code
pub fn text_content(text: impl Into<String>) -> String {
    text.into()
}

#[derive(Subcommand, Clone, Debug)]
pub enum CodeAction {
    /// Explain code functionality
    Explain {
        /// File path or code snippet
        input: String,
        /// Input is a file path (default: auto-detect)
        #[arg(short, long)]
        file: bool,
    },
    /// Review code for improvements
    Review {
        /// File path or code snippet
        input: String,
        /// Input is a file path (default: auto-detect)
        #[arg(short, long)]
        file: bool,
        /// Focus on specific aspects (security, performance, style, etc.)
        #[arg(long)]
        focus: Option<String>,
    },
    /// Generate code from description
    Generate {
        /// Description of what to generate
        description: Vec<String>,
        /// Programming language
        #[arg(short, long)]
        language: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Fix code issues
    Fix {
        /// File path containing code to fix
        file: String,
        /// Description of the issue to fix
        issue: Vec<String>,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum AcpAction {
    /// Start ACP server for Zed integration
    Server {
        /// Port to bind to (default: auto-assign)
        #[arg(short, long)]
        port: Option<u16>,
        /// Host to bind to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Start ACP in stdio mode (default for Zed)
    Stdio {
        /// Model to use (overrides default)
        #[arg(long)]
        model: Option<String>,
    },
    /// Test ACP connection
    Test {
        /// ACP server address
        #[arg(short, long)]
        address: String,
    },
    /// Show ACP capabilities
    Capabilities,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Initialize configuration with defaults
    Init {
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },
    /// Validate configuration
    Validate,
}

#[derive(Subcommand, Clone, Debug)]
pub enum SettingsAction {
    /// Show interactive settings browser
    Show,
    /// Edit settings interactively
    Edit,
    /// Reset settings to defaults
    Reset {
        /// Category to reset (optional, resets all if not specified)
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Export settings to file
    Export {
        /// Export file path
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Import settings from file
    Import {
        /// Import file path
        #[arg(short, long)]
        path: String,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum HistoryAction {
    /// List all chat sessions
    List,
    /// View a specific chat session
    View {
        /// Session ID to view
        session_id: String,
    },
    /// Search through chat sessions
    Search {
        /// Search query
        query: String,
    },
    /// Clear chat history
    Clear {
        /// Confirm deletion
        #[arg(long)]
        confirm: bool,
    },
}

// Re-export commonly used types and functions
pub use config::{Config, ConfigSource, RateLimitConfig};
pub use display::{
    ascii_art::{get_logo_for_width, print_grok_logo},
    banner::{BannerConfig, BannerType, print_banner, print_welcome_banner},
    get_terminal_size,
};

```


## Assistant

<thinking>
**Developing the CLI Command**

I'm currently working on building the CLI command. I'm focusing on the `SkillsCommand` enum and the `handle_skills_command` function within `src/cli /commands/skills.rs`. The goal is to establish a well-structured framework for future command implementations. I'm using `clap` and `anyhow` for parsing and error handling, respectively.


**Implementing Skills Commands**

I've started implementing the `SkillsCommand` enum and `handle_skills_command` function within `src/cli/commands/skills.rs`. I'm employing `clap` and `anyhow` for command parsing and error management. I'm focusing on the `List`, `Show`, and `New` subcommands to handle skill-related actions and using the `get_default_skills_dir` utility function.


**Designing Skill Command Logic**

I'm now in the process of implementing the core logic within the `SkillsCommand` enum variants. I'm focusing on the `List`, `Show`, and `New` subcommands. For `List`, I'm working on retrieving and displaying the available skills, including their names and descriptions using the `list_skills` helper. For `Show`, the plan is to display skill details by their name, leveraging `load_skill` to retrieve the configurations, and providing better instructions. Finally, for `New`, the focus is to create the template, which involves directory and file creation with proper handling of pre-existing skills.


**Writing Skills Module File**

I'm presently creating the `src/cli/commands/skills.rs` file. Within it, I'm defining the `SkillsCommand` enum with `List`, `Show`, and `New` subcommands and structuring the `handle_skills_command` function. I will be incorporating skill listing, showing skill details, and the creation of new skill templates, focusing on proper file and directory handling, and using `clap`, `anyhow`, and `colored` as I have been.



</thinking>

**Tool Call: Writing to src\cli\commands\skills.rs**
Status: Completed

Diff: skills.rs
```
use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;
use crate::skills::{list_skills, get_default_skills_dir};
use std::fs;

#[derive(Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    /// List available skills
    List,
    /// Show details of a specific skill
    Show {
        /// Name of the skill to show
        name: String,
    },
    /// Create a new skill template
    New {
        /// Name of the new skill
        name: String,
    },
}

pub async fn handle_skills_command(command: SkillsCommand) -> Result<()> {
    let skills_dir = get_default_skills_dir().unwrap_or_else(|| std::env::current_dir().unwrap().join(".grok/skills"));
    
    match command {
        SkillsCommand::List => {
            if !skills_dir.exists() {
                 println!("No skills directory found at {}. Use 'grok skills new <name>' to create one.", skills_dir.display());
                 return Ok(());
            }
            let skills = list_skills(&skills_dir)?;
            if skills.is_empty() {
                println!("No skills found in {}", skills_dir.display());
            } else {
                println!("{}", "Available Skills:".bright_cyan().bold());
                for skill in skills {
                    println!("  • {} - {}", skill.config.name.green().bold(), skill.config.description.dimmed());
                }
            }
        }
        SkillsCommand::Show { name } => {
            let skills = list_skills(&skills_dir)?;
            if let Some(skill) = skills.into_iter().find(|s| s.config.name == name) {
                 println!("{}", format!("Skill: {}", skill.config.name).green().bold());
                 println!("Description: {}", skill.config.description);
                 if let Some(license) = &skill.config.license {
                     println!("License: {}", license);
                 }
                 println!();
                 println!("{}", "Instructions:".bright_yellow());
                 println!("{}", skill.instructions);
            } else {
                println!("Skill '{}' not found.", name.red());
            }
        }
        SkillsCommand::New { name } => {
             let skill_path = skills_dir.join(&name);
             if skill_path.exists() {
                 println!("Skill '{}' already exists at {}", name, skill_path.display());
                 return Ok(());
             }
             fs::create_dir_all(&skill_path)?;
             let skill_md = format!(r#"---
name: {}
description: Description for {}
license: MIT
---

# Instructions for {}

Write your skill instructions here.
"#, name, name, name);
             fs::write(skill_path.join("SKILL.md"), skill_md)?;
             println!("Created new skill '{}' at {}", name.green(), skill_path.display());
        }
    }
    Ok(())
}

```


## Assistant

<thinking>
**Examining the Implementation**

I've identified a discrepancy. It appears the `handle_skills_command` function, though defined, isn't being utilized within either the `src/cli/commands/mod.rs` module or the `src/cli/app.rs` file. My next step is clear: I must now modify `src/cli/commands/mod.rs`.



</thinking>

**Tool Call: src\cli\commands\mod.rs**
Status: Completed

## Assistant

<thinking>
**Adding a module**

I've just added `pub mod skills;` to `src/cli/commands/ mod.rs`. This seems like a small step, but it lays the groundwork for modularizing the skill-related command logic. My next step will be to define the structure for the `skills` module.



</thinking>

**Tool Call: src\cli\commands\mod.rs: pub mod acp;... => pub mod acp;...**
Status: Completed

Diff: mod.rs
```
//! Command handlers for grok-cli
//!
//! This module contains all the command handler implementations for the various
//! CLI commands supported by grok-cli.
//!
//! Note: Individual command modules use deprecated I/O functions that will be
//! refactored in Phase 2. They have #![allow(deprecated)] to suppress warnings.

pub mod acp;
pub mod chat;
pub mod code;
pub mod config;
pub mod health;
pub mod history;
pub mod settings;
pub mod skills;

// Re-export all command handlers

```


## Assistant

<thinking>
**Reviewing Command Dispatch**

I'm currently focused on the `src/main.rs` file to implement command dispatch. My plan is to add the necessary logic there to route commands appropriately. I'm exploring the best approach to integrate the command-line interface with the core application logic.



</thinking>

**Tool Call: src\main.rs**
Status: Completed

## Assistant

<thinking>
**Inspecting the Run Function**

I'm now diving into the `src/cli/app.rs` file to scrutinize the `run()` function. The objective is to understand how the command-line application is initialized and executed. My focus is on the control flow within this function and how it orchestrates the overall application logic.



</thinking>

**Tool Call: src\cli\app.rs**
Status: Completed

## Assistant

<thinking>
**Reflecting on Implementation**

I'm currently working on adding the `Skills` variant to the `Commands` enum in `src/cli/app.rs`. My next step is to update the `run` function to process this new command. I'm focused on modifying the `src/cli/app.rs` file to integrate this functionality.


**Implementing the Skills Variant**

I've added the `Skills` variant to the `Commands` enum in `src/cli/app.rs` and integrated the relevant handling within the `run` function. Specifically, I've added a match arm to dispatch to the `handle_skills_command` function. My focus is now shifting to ensuring the correct context is maintained for the skills-related actions.



</thinking>

**Tool Call: src\cli\app.rs:     /// Health check and diagn... =>     /// Health check and diagn...**
Status: Completed

Diff: app.rs
```
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

use crate::config::{Config, ConfigSource};
use crate::display::banner::{BannerConfig, print_welcome_banner};
use crate::display::interactive::{InteractiveConfig, PromptStyle, start_interactive_mode};
use crate::utils::auth::{require_api_key, resolve_api_key};
use crate::utils::network::test_connectivity;

/// Grok CLI - Command-line interface for Grok AI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// API key for authentication
    #[arg(short, long, env = "GROK_API_KEY")]
    pub api_key: Option<String>,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Hide banner
    #[arg(long)]
    pub hide_banner: bool,

    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chat with Grok AI
    Chat {
        /// The message to send
        #[arg(required = true)]
        message: Vec<String>,

        /// Start an interactive chat session
        #[arg(short, long)]
        interactive: bool,

        /// System prompt to use
        #[arg(short, long)]
        system: Option<String>,

        /// Temperature for response generation (0.0 to 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f32,

        /// Maximum tokens in response
        #[arg(long, default_value = "4096")]
        max_tokens: u32,
    },

    /// Code-related operations
    Code {
        #[command(subcommand)]
        action: crate::CodeAction,
    },

    /// ACP (Agent Client Protocol) operations for Zed integration
    Acp {
        #[command(subcommand)]
        action: crate::AcpAction,
    },

    /// Interactive chat mode
    Interactive,

    /// Send a single query
    Query {
        /// The question or prompt to send
        #[arg(required = true)]
        prompt: Vec<String>,
    },

    /// Test network connectivity
    TestNetwork {
        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: crate::ConfigAction,
    },

    /// Settings management and configuration
    Settings {
        #[command(subcommand)]
        action: crate::SettingsAction,
    },

    /// Chat history management
    History {
        #[command(subcommand)]
        action: crate::HistoryAction,
    },

    /// Health check and diagnostics
    Health {
        /// Check API connectivity
        #[arg(long)]
        api: bool,

        /// Check configuration
        #[arg(long)]
        config: bool,

        /// Check all systems
        #[arg(long)]
        all: bool,
    },

    /// Manage agent skills
    Skills {
        #[command(subcommand)]
        action: crate::cli::commands::skills::SkillsCommand,
    },
}

/// Main application entry point
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = if let Some(config_path) = &cli.config {
        // Use explicit config path if provided
        let path_str = config_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid config path: contains non-UTF8 characters"))?;
        Config::load(Some(path_str)).await?
    } else {
        // Use hierarchical loading: project → system → defaults
        match Config::load_hierarchical().await {
            Ok(cfg) => {
                info!("✓ Configuration loaded successfully");
                cfg
            }
            Err(e) => {
                error!("Failed to load hierarchical configuration: {}", e);
                error!("Falling back to default configuration");
                Config {
                    config_source: Some(ConfigSource::Default),
                    ..Config::default()
                }
            }
        }
    };

    // Initialize telemetry
    crate::utils::telemetry::init(config.telemetry.enabled, config.telemetry.log_file.clone());

    // Resolve API key once
    let api_key = resolve_api_key(cli.api_key.clone(), &config);

    // Resolve model: CLI argument overrides config
    let model = cli.model.as_deref().unwrap_or(&config.default_model);

    // Show banner function
    let show_banner_fn = || {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: None,
        };
        print_welcome_banner(&banner_config);
    };

    match &cli.command {
        Some(Commands::Chat {
            message,
            interactive,
            system,
            temperature,
            max_tokens,
        }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: message.clone(),
                interactive: *interactive,
                system: system.clone(),
                temperature: *temperature,
                max_tokens: *max_tokens,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::Code { action }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::code::handle_code_action(
                action.clone(),
                &api_key,
                model,
                config.timeout_secs,
                config.max_retries,
                config.rate_limits,
            )
            .await?;
        }
        Some(Commands::Acp { action }) => {
            crate::cli::commands::acp::handle_acp_action(action.clone(), &config).await?;
        }
        Some(Commands::Interactive) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
        Some(Commands::Query { prompt }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let query = prompt.join(" ");

            if !cli.hide_banner {
                show_banner_fn();
            }

            info!("Sending query: {}", query);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: vec![query],
                interactive: false,
                system: None,
                temperature: 0.7,
                max_tokens: 4096,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::TestNetwork { timeout }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            let timeout_duration = std::time::Duration::from_secs(*timeout);
            match test_connectivity(timeout_duration).await {
                Ok(duration) => {
                    println!("✓ Network connectivity test passed in {:?}", duration);
                }
                Err(e) => {
                    error!("Network connectivity test failed: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Config { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::config::handle_config_action(action.clone(), &config).await?;
        }
        Some(Commands::Settings { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::settings::handle_settings_action(action.clone(), &config).await?;
        }
        Some(Commands::History { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::history::handle_history_action(action.clone()).await?;
        }
        Some(Commands::Health {
            api,
            config: check_config,
            all,
        }) => {
            let check_api = *api || *all;
            let check_cfg = *check_config || *all;
            crate::cli::commands::health::handle_health_check(
                check_api,
                check_cfg,
                api_key.as_deref(),
                &config,
                model,
                config.timeout_secs,
            )
            .await?;
        }
        None => {
            // Default to interactive mode
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
    }

    Ok(())
}

```


## Assistant

<thinking>
**Constructing The Run Function**

I'm now integrating the handler into the `run` function, a crucial step. This update involves refining the execution flow. I am focusing on ensuring that the handler interacts correctly within this primary function. This is critical for the overall operational integrity.



</thinking>

**Tool Call: src\cli\app.rs:             crate::cli::comman... =>             crate::cli::comman...**
Status: Failed

Failed to edit, 0 occurrences found for old_string in H:\GitHub\grok-cli\src\cli\app.rs. Ensure you're not escaping content incorrectly and check whitespace, indentation, and context. Use read_file tool to verify.

## Assistant

<thinking>
**Reviewing the Handler's Code**

I'm revisiting the `Health` command handler. It seems I might have misidentified the exact lines during my initial assessment. I'll need to re-examine the file to pinpoint the correct code block.



</thinking>

**Tool Call: src\cli\app.rs**
Status: Completed

Read lines 318-317 of 317 from src/cli/app.rs

## Assistant

<thinking>
**Considering the Bounds**

I've hit a snag. The file I'm working with has fewer lines than I initially anticipated. This throws off my offset calculations, and I need to adjust my starting point to stay within the file's boundaries. I'll read from line 300 to avoid any errors.



</thinking>

**Tool Call: src\cli\app.rs**
Status: Completed

Read lines 301-317 of 317 from src/cli/app.rs

## Assistant

<thinking>
**Identifying Insertion Point**

I've zeroed in on the exact location: it's right before the `None` value. My next step is to pinpoint the `Health` command block to ensure proper context. This should be a straightforward task of code analysis.



</thinking>

**Tool Call: Tool call not found**
Status: Failed

Tool call not found

## Assistant

<thinking>
**Addressing the oversight**

I've realized a critical omission in my initial assessment: the `file_path`. This oversight necessitates a revision of my approach. I'm now refocusing on incorporating this crucial element, which will likely alter the trajectory of my reasoning.



</thinking>

**Tool Call: src\cli\app.rs**
Status: Completed

Read lines 281-317 of 317 from src/cli/app.rs

## Assistant

<thinking>
**Analyzing Current Content**

\n\n

I've got the content pinned down now. Focusing on how the *check_api* and *check_cfg* booleans are being set. Thinking about how the `||` operator impacts the logic. It seems straightforward, but I need to make sure I grasp all the implications with respect to the `all` flag.


**Implementing Match Arms**

\n\n

I'm expanding the match statement now. The *Health* arm is good to go, now I'm adding the *Skills* arm. After this, I'll need to figure out what needs to be in the `None` arm. Getting this structure correct is key to handling all the command options.



</thinking>

**Tool Call: src\cli\app.rs:             .await?;... =>             .await?;...**
Status: Completed

Diff: app.rs
```
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

use crate::config::{Config, ConfigSource};
use crate::display::banner::{BannerConfig, print_welcome_banner};
use crate::display::interactive::{InteractiveConfig, PromptStyle, start_interactive_mode};
use crate::utils::auth::{require_api_key, resolve_api_key};
use crate::utils::network::test_connectivity;

/// Grok CLI - Command-line interface for Grok AI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// API key for authentication
    #[arg(short, long, env = "GROK_API_KEY")]
    pub api_key: Option<String>,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Hide banner
    #[arg(long)]
    pub hide_banner: bool,

    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chat with Grok AI
    Chat {
        /// The message to send
        #[arg(required = true)]
        message: Vec<String>,

        /// Start an interactive chat session
        #[arg(short, long)]
        interactive: bool,

        /// System prompt to use
        #[arg(short, long)]
        system: Option<String>,

        /// Temperature for response generation (0.0 to 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f32,

        /// Maximum tokens in response
        #[arg(long, default_value = "4096")]
        max_tokens: u32,
    },

    /// Code-related operations
    Code {
        #[command(subcommand)]
        action: crate::CodeAction,
    },

    /// ACP (Agent Client Protocol) operations for Zed integration
    Acp {
        #[command(subcommand)]
        action: crate::AcpAction,
    },

    /// Interactive chat mode
    Interactive,

    /// Send a single query
    Query {
        /// The question or prompt to send
        #[arg(required = true)]
        prompt: Vec<String>,
    },

    /// Test network connectivity
    TestNetwork {
        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: crate::ConfigAction,
    },

    /// Settings management and configuration
    Settings {
        #[command(subcommand)]
        action: crate::SettingsAction,
    },

    /// Chat history management
    History {
        #[command(subcommand)]
        action: crate::HistoryAction,
    },

    /// Health check and diagnostics
    Health {
        /// Check API connectivity
        #[arg(long)]
        api: bool,

        /// Check configuration
        #[arg(long)]
        config: bool,

        /// Check all systems
        #[arg(long)]
        all: bool,
    },

    /// Manage agent skills
    Skills {
        #[command(subcommand)]
        action: crate::cli::commands::skills::SkillsCommand,
    },
}

/// Main application entry point
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = if let Some(config_path) = &cli.config {
        // Use explicit config path if provided
        let path_str = config_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid config path: contains non-UTF8 characters"))?;
        Config::load(Some(path_str)).await?
    } else {
        // Use hierarchical loading: project → system → defaults
        match Config::load_hierarchical().await {
            Ok(cfg) => {
                info!("✓ Configuration loaded successfully");
                cfg
            }
            Err(e) => {
                error!("Failed to load hierarchical configuration: {}", e);
                error!("Falling back to default configuration");
                Config {
                    config_source: Some(ConfigSource::Default),
                    ..Config::default()
                }
            }
        }
    };

    // Initialize telemetry
    crate::utils::telemetry::init(config.telemetry.enabled, config.telemetry.log_file.clone());

    // Resolve API key once
    let api_key = resolve_api_key(cli.api_key.clone(), &config);

    // Resolve model: CLI argument overrides config
    let model = cli.model.as_deref().unwrap_or(&config.default_model);

    // Show banner function
    let show_banner_fn = || {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: None,
        };
        print_welcome_banner(&banner_config);
    };

    match &cli.command {
        Some(Commands::Chat {
            message,
            interactive,
            system,
            temperature,
            max_tokens,
        }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: message.clone(),
                interactive: *interactive,
                system: system.clone(),
                temperature: *temperature,
                max_tokens: *max_tokens,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::Code { action }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::code::handle_code_action(
                action.clone(),
                &api_key,
                model,
                config.timeout_secs,
                config.max_retries,
                config.rate_limits,
            )
            .await?;
        }
        Some(Commands::Acp { action }) => {
            crate::cli::commands::acp::handle_acp_action(action.clone(), &config).await?;
        }
        Some(Commands::Interactive) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
        Some(Commands::Query { prompt }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let query = prompt.join(" ");

            if !cli.hide_banner {
                show_banner_fn();
            }

            info!("Sending query: {}", query);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: vec![query],
                interactive: false,
                system: None,
                temperature: 0.7,
                max_tokens: 4096,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::TestNetwork { timeout }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            let timeout_duration = std::time::Duration::from_secs(*timeout);
            match test_connectivity(timeout_duration).await {
                Ok(duration) => {
                    println!("✓ Network connectivity test passed in {:?}", duration);
                }
                Err(e) => {
                    error!("Network connectivity test failed: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Config { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::config::handle_config_action(action.clone(), &config).await?;
        }
        Some(Commands::Settings { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::settings::handle_settings_action(action.clone(), &config).await?;
        }
        Some(Commands::History { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::history::handle_history_action(action.clone()).await?;
        }
        Some(Commands::Health {
            api,
            config: check_config,
            all,
        }) => {
            let check_api = *api || *all;
            let check_cfg = *check_config || *all;
            crate::cli::commands::health::handle_health_check(
                check_api,
                check_cfg,
                api_key.as_deref(),
                &config,
                model,
                config.timeout_secs,
            )
            .await?;
        }
        Some(Commands::Skills { action }) => {
            crate::cli::commands::skills::handle_skills_command(action.clone()).await?;
        }
        None => {
            // Default to interactive mode
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
    }

    Ok(())
}

```


## Assistant

<thinking>
**Analyzing Skill Integration**

I've successfully integrated the CLI commands. Now, the challenge is skill integration into the context. My current plan involves modifying the project context loading, specifically in `src/utils/context.rs`, likely creating a new function or adjusting the existing one. However, after thinking it through, project context and skill context are distinct. I'm leaning toward loading skills and adding their instructions to the system prompt in `src/display/interactive.rs`.


**Implementing Skill Loading**

I've decided to add `load_skills_context` to `src/skills/manager.rs`, which will return a formatted string containing instructions for all discovered skills. This function's output will then be appended to the system prompt within `src/display/interactive.rs`. I'm going with this approach to swiftly integrate all available skills. I will load all skills found in the `.grok/skills` directory, without any user interaction for now.


**Designing Skill Loading Logic**

I'm now implementing the `load_skills_context` function in `src/skills/manager.rs`, which will take the base directory, discover available skills, and formulate a comprehensive instruction set. The plan is to append the resulting string to the system prompt in `src/display/interactive.rs`. I'll also add a guard in `src/display/interactive.rs` to ensure the skill instructions are added to the system prompt if skills exist.



</thinking>

**Tool Call: src\skills\manager.rs:     } else {... =>     } else {...**
Status: Completed

Diff: manager.rs
```
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::skills::config::{Skill, SkillConfig};

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(anyhow!("No SKILL.md found in {}", skill_dir.display()));
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content manually to be robust
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of the frontmatter
    // We look for the second "---" after the first one
    let end_fm_idx = content[3..]
        .find("\n---")
        .map(|i| i + 3) // Add back the offset
        .ok_or_else(|| anyhow!("Could not find end of frontmatter in SKILL.md"))?;

    let frontmatter = &content[3..end_fm_idx].trim();

    // The instructions start after the second "---"
    // end_fm_idx points to the start of the second "---"
    // The second "---" has length 3, plus usually a newline
    let instructions_start = end_fm_idx + 4; // +4 for "\n---" length if we found via "\n---"

    // Check if we have enough content
    let instructions = if instructions_start < content.len() {
        content[instructions_start..].trim().to_string()
    } else {
        String::new()
    };

    let config: SkillConfig = serde_yaml::from_str(frontmatter).with_context(|| {
        format!(
            "Failed to parse YAML frontmatter in {}",
            skill_file.display()
        )
    })?;

    Ok(Skill {
        config,
        instructions,
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    // We only look at immediate subdirectories of the skills folder
    for entry in WalkDir::new(base_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            if entry.path().join("SKILL.md").exists() {
                match load_skill(entry.path()) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        // Silently ignore or log warning if possible
                        // eprintln!("Warning: Failed to load skill at {}: {}", entry.path().display(), e);
                    }
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}

/// Get formatted context string for all available skills
pub fn get_skills_context(base_dir: &Path) -> Result<String> {
    let skills = list_skills(base_dir)?;
    if skills.is_empty() {
        return Ok(String::new());
    }
    
    let mut context = String::from("\n\n## Available Skills\n\nThe following skills are available for use:\n\n");
    for skill in skills {
        context.push_str(&format!("### Skill: {}\n", skill.config.name));
        context.push_str(&format!("Description: {}\n", skill.config.description));
        context.push_str("\nInstructions:\n");
        context.push_str(&skill.instructions);
        context.push_str("\n\n---\n\n");
    }
    Ok(context)
}


```


## Assistant

<thinking>
**Analyzing Integration Strategy**

I'm currently focused on integrating the skills context into the interactive display. My current plan is to modify the `load_project_context_for_session` function to handle skill context loading as well. Alternatively, I'm considering appending this in `start_interactive_mode`. The latter seems simpler at first glance, since `start_interactive_mode` eventually calls `load_project_context`.


**Implementing Context Integration**

I've reviewed `src/display/interactive.rs` to finalize how to integrate the skill context. The direct approach of modifying `load_project_context_for_session` to include skill loading appears cleaner, though I'm still weighing the alternatives. I think I'll go with the append strategy within `start_interactive_mode` to simplify my modifications.



</thinking>

**Tool Call: src\display\interactive.rs**
Status: Completed

Read lines 91-190 of 1265 from src/display/interactive.rs

## Assistant

<thinking>
**Analyzing Skill Loading**

I'm currently focused on integrating the skills context into `start_interactive_mode`. Specifically, I'm working to ensure that the project context is loaded correctly when the interactive session begins. My aim is to use the `load_project_context_for_session` function, with the current directory as input, to prepare the environment.


**Modifying Context Loading**

I'm now revising the code to integrate the skills context within the `start_interactive_mode` function. I intend to use the `get_skills_context` function to load and incorporate skill definitions. I'll need to handle the potential absence of a skills directory gracefully and ensure context concatenation, if skills are found, within the project context. The code snippet above is my current focus.



</thinking>

**Tool Call: src\display\interactive.rs:     // Load project context if... =>     // Load project context if...**
Status: Completed

Diff: interactive.rs
```
//! Interactive mode for Grok CLI
//!
//! Provides a Gemini CLI-like interactive experience with persistent sessions,
//! input prompts, and real-time status display

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::Result;
use colored::*;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::{Duration, sleep};

use crate::GrokClient;
use crate::acp::security::SecurityPolicy;
use crate::acp::tools;
use crate::config::Config;
use crate::display::{
    BannerConfig, clear_current_line, print_directory_recommendation, print_grok_logo,
    print_welcome_banner,
};
use crate::utils::context::{
    format_context_for_prompt, get_all_context_file_paths, load_and_merge_project_context,
};
use crate::utils::session::{list_sessions, load_session, save_session};
use crate::utils::shell_permissions::{ApprovalMode, ShellPermissions};
use crate::{content_to_string, extract_text_content, text_content};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Interactive session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSession {
    pub session_id: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub conversation_history: Vec<ConversationItem>,
    pub current_directory: PathBuf,
    pub show_context_usage: bool,
    pub total_tokens_used: u32,
}

/// Conversation item in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItem {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: Option<u32>,
}

/// Interactive mode configuration
#[derive(Debug, Clone)]
pub struct InteractiveConfig {
    pub show_banner: bool,
    pub show_tips: bool,
    pub show_status: bool,
    pub auto_save_session: bool,
    pub prompt_style: PromptStyle,
    pub check_directory: bool,
}

/// Different prompt styles
#[derive(Debug, Clone, PartialEq)]
pub enum PromptStyle {
    Simple,
    Rich,
    Minimal,
}

impl Default for InteractiveConfig {
    fn default() -> Self {
        Self {
            show_banner: true,
            show_tips: true,
            show_status: true,
            auto_save_session: false,
            prompt_style: PromptStyle::Rich,
            check_directory: true,
        }
    }
}

impl InteractiveSession {
    /// Create a new interactive session
    pub fn new(model: String, system_prompt: Option<String>) -> Self {
        let session_id = generate_session_id();
        let current_directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            session_id,
            model,
            temperature: 0.7,
            max_tokens: 4096,
            system_prompt,
            conversation_history: Vec::new(),
            current_directory,
            show_context_usage: true,
            total_tokens_used: 0,
        }
    }

    /// Add a conversation item to the history
    pub fn add_conversation_item(&mut self, role: &str, content: &str, tokens_used: Option<u32>) {
        let item = ConversationItem {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            tokens_used,
        };

        if let Some(tokens) = tokens_used {
            self.total_tokens_used += tokens;
        }

        self.conversation_history.push(item);
    }

    /// Get context usage information
    pub fn get_context_info(&self) -> String {
        let conversation_count = self.conversation_history.len();
        let context_percentage = if self.total_tokens_used > 0 {
            let estimated_max = 8192; // Rough estimate for context window
            ((self.total_tokens_used as f32 / estimated_max as f32) * 100.0) as u8
        } else {
            0
        };

        format!(
            "{}% context left | {} messages",
            100 - context_percentage,
            conversation_count
        )
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("grok-{}", timestamp)
}

/// Start the interactive mode
pub async fn start_interactive_mode(
    api_key: &str,
    model: &str,
    config: &Config,
    interactive_config: InteractiveConfig,
) -> Result<()> {
    // Load project context if available
    let mut project_context = load_project_context_for_session(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );

    // Load skills context
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
             if !skills_context.is_empty() {
                 let ctx = project_context.get_or_insert_with(String::new);
                 ctx.push_str(&skills_context);
             }
        }
    }

    let mut session = InteractiveSession::new(model.to_string(), project_context);
    let client = GrokClient::new(api_key)?;

    // Display startup elements
    if interactive_config.show_banner {
        display_startup_screen(&interactive_config, &session, config).await?;
    }

    // Check if running in home directory
    if interactive_config.check_directory && is_home_directory(&session.current_directory) {
        let banner_config = BannerConfig::default();
        print_directory_recommendation(
            &session.current_directory.display().to_string(),
            &banner_config,
        );
    }

    // Main interactive loop
    loop {
        match run_interactive_loop(&mut session, &client, &interactive_config, config).await {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Error:".red(), e);
                continue;
            }
        }
    }

    // Goodbye message
    println!("{}", "\n👋 Thanks for using Grok CLI!".bright_cyan());

    if interactive_config.auto_save_session && !session.conversation_history.is_empty() {
        println!("{}", "Session saved for future reference.".dimmed());
    }

    Ok(())
}

/// Display the startup screen
async fn display_startup_screen(
    config: &InteractiveConfig,
    session: &InteractiveSession,
    app_config: &Config,
) -> Result<()> {
    let (width, _) = crate::display::get_terminal_size();

    // Clear screen and show logo with animation
    crate::display::clear_screen();

    if config.show_banner && !config.show_tips {
        print_grok_logo(width);
        sleep(Duration::from_millis(500)).await;
    }

    if config.show_tips {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: Some(width),
        };
        print_welcome_banner(&banner_config);
    }

    // Show current session info
    if config.show_status {
        print_session_info(session, app_config);
    }

    Ok(())
}

/// Print current session information
fn print_session_info(session: &InteractiveSession, config: &Config) {
    println!("{}", "Current session:".bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_yellow()
    );

    // Show config source
    if let Some(source) = &config.config_source {
        println!("  Configuration: {}", source.display().bright_magenta());
    }

    // Show context files info if loaded
    let context_paths = get_all_context_file_paths(&session.current_directory);
    if !context_paths.is_empty() {
        if context_paths.len() == 1 {
            println!(
                "  Context loaded: {}",
                context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .bright_green()
            );
        } else {
            println!(
                "  Context loaded: {} files",
                context_paths.len().to_string().bright_green()
            );
            for path in &context_paths {
                println!(
                    "    - {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .dimmed()
                );
            }
        }
    }

    if let Some(system) = &session.system_prompt {
        let preview = if system.len() > 60 {
            format!("{}...", &system[..60])
        } else {
            system.clone()
        };
        println!("  System prompt: {}", preview.bright_green());
    }
    println!();
}

/// Load project context for a new session
fn load_project_context_for_session(project_root: &PathBuf) -> Option<String> {
    match load_and_merge_project_context(project_root) {
        Ok(Some(context)) => {
            let formatted = format_context_for_prompt(&context);
            let context_paths = get_all_context_file_paths(project_root);

            if context_paths.is_empty() {
                // Shouldn't happen but handle gracefully
                return Some(formatted);
            }

            if context_paths.len() == 1 {
                let context_file_name = context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_os_string().into_string().ok())
                    .unwrap_or_else(|| "context file".to_string());

                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded project context from {}", context_file_name).dimmed()
                );
            } else {
                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded and merged {} context files", context_paths.len()).dimmed()
                );
                for path in &context_paths {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        println!("  {} {}", "•".dimmed(), name.dimmed());
                    }
                }
            }

            Some(formatted)
        }
        Ok(None) => {
            // No context file found - this is normal
            None
        }
        Err(e) => {
            eprintln!(
                "{} Failed to load project context: {}",
                "⚠".yellow(),
                e.to_string().dimmed()
            );
            None
        }
    }
}

use crate::display::components::input::{Suggestion, read_input_with_suggestions};

/// Main interactive loop
async fn run_interactive_loop(
    session: &mut InteractiveSession,
    client: &GrokClient,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<bool> {
    // Prepare prompt
    let prompt = match interactive_config.prompt_style {
        PromptStyle::Simple => format!("{} ", ">".bright_cyan()),
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            format!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            )
        }
        PromptStyle::Minimal => "» ".to_string(),
    };

    // Prepare suggestions
    let suggestions = vec![
        Suggestion {
            text: "/clear".to_string(),
            description: "Clear screen".to_string(),
        },
        Suggestion {
            text: "/help".to_string(),
            description: "Show help message".to_string(),
        },
        Suggestion {
            text: "/history".to_string(),
            description: "Show history".to_string(),
        },
        Suggestion {
            text: "/list".to_string(),
            description: "List saved sessions".to_string(),
        },
        Suggestion {
            text: "/load".to_string(),
            description: "Load a session".to_string(),
        },
        Suggestion {
            text: "/model".to_string(),
            description: "Change model".to_string(),
        },
        Suggestion {
            text: "/quit".to_string(),
            description: "Exit interactive mode".to_string(),
        },
        Suggestion {
            text: "/reset".to_string(),
            description: "Reset session".to_string(),
        },
        Suggestion {
            text: "/save".to_string(),
            description: "Save current session".to_string(),
        },
        Suggestion {
            text: "/settings".to_string(),
            description: "Open settings".to_string(),
        },
        Suggestion {
            text: "/status".to_string(),
            description: "Show status".to_string(),
        },
        Suggestion {
            text: "/system".to_string(),
            description: "Set system prompt".to_string(),
        },
        Suggestion {
            text: "/tools".to_string(),
            description: "List coding tools".to_string(),
        },
        Suggestion {
            text: "/version".to_string(),
            description: "Show version info".to_string(),
        },
        Suggestion {
            text: "/config".to_string(),
            description: "Show configuration info".to_string(),
        },
        Suggestion {
            text: "!ls".to_string(),
            description: "List files (shell command)".to_string(),
        },
        Suggestion {
            text: "!dir".to_string(),
            description: "List files on Windows (shell command)".to_string(),
        },
        Suggestion {
            text: "!git status".to_string(),
            description: "Check git status (shell command)".to_string(),
        },
        Suggestion {
            text: "!pwd".to_string(),
            description: "Print working directory (shell command)".to_string(),
        },
    ];

    // Read user input
    // Note: We're running blocking TUI code in an async context, which is generally bad,
    // but for a CLI it's acceptable as we're awaiting user input anyway.
    let input =
        tokio::task::spawn_blocking(move || read_input_with_suggestions(&prompt, &suggestions))
            .await??;

    let input = input.trim();

    // Handle empty input
    if input.is_empty() {
        return Ok(true);
    }

    // Handle shell commands (starting with !)
    if input.starts_with('!') {
        return handle_shell_command(input).await;
    }

    // Handle special commands
    if let Some(command_result) =
        handle_special_commands(input, session, interactive_config, app_config).await?
    {
        return Ok(command_result);
    }

    // Send to Grok API
    match send_to_grok(client, session, input).await {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("{} Failed to get response: {}", "Error:".red(), e);
            Ok(true)
        }
    }
}

/// Display the input prompt
fn display_prompt(session: &InteractiveSession, config: &InteractiveConfig) -> Result<()> {
    match config.prompt_style {
        PromptStyle::Simple => {
            print!("{} ", ">".bright_cyan());
        }
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            print!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            );
        }
        PromptStyle::Minimal => {
            print!("» ");
        }
    }

    io::stdout().flush()?;
    Ok(())
}

/// Read user input from stdin
fn read_user_input() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

/// Handle shell commands (those starting with !)
async fn handle_shell_command(input: &str) -> Result<bool> {
    let command = input.trim_start_matches('!').trim();

    if command.is_empty() {
        println!("{}", "Error: No command specified".red());
        return Ok(true);
    }

    // Create permissions manager (TODO: pass from session state)
    let mut permissions = ShellPermissions::new(ApprovalMode::Default);

    // Check if command should be executed
    match permissions.should_execute(command) {
        Ok(true) => {
            // Permission granted, execute command
            println!();
            println!("{} {}", "Executing:".bright_cyan(), command.bright_yellow());
            println!();

            // Determine shell based on OS
            #[cfg(target_os = "windows")]
            let shell = "cmd";
            #[cfg(target_os = "windows")]
            let shell_arg = "/C";

            #[cfg(not(target_os = "windows"))]
            let shell = "sh";
            #[cfg(not(target_os = "windows"))]
            let shell_arg = "-c";

            // Execute the command
            match std::process::Command::new(shell)
                .arg(shell_arg)
                .arg(command)
                .output()
            {
                Ok(output) => {
                    // Print stdout
                    if !output.stdout.is_empty() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        print!("{}", stdout);
                    }

                    // Print stderr in red
                    if !output.stderr.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprint!("{}", stderr.red());
                    }

                    // Show exit code if non-zero
                    if !output.status.success() {
                        println!();
                        println!(
                            "{} Command exited with code: {}",
                            "⚠".yellow(),
                            output.status.code().unwrap_or(-1)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to execute command: {}", "Error:".red(), e);
                }
            }

            println!();
        }
        Ok(false) => {
            // Permission denied
            println!();
            println!("{}", "Command execution cancelled".yellow());
            println!();
        }
        Err(e) => {
            eprintln!("{} Permission check failed: {}", "Error:".red(), e);
        }
    }

    Ok(true)
}

/// Handle special commands (those starting with /)
async fn handle_special_commands(
    input: &str,
    session: &mut InteractiveSession,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<Option<bool>> {
    if !input.starts_with('/') {
        return Ok(None);
    }

    let command = input.trim_start_matches('/').trim();
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(Some(true));
    }

    match parts[0] {
        "help" | "h" => {
            print_interactive_help();
            Ok(Some(true))
        }
        "quit" | "exit" | "q" => Ok(Some(false)),
        "clear" | "cls" => {
            crate::display::clear_screen();
            if interactive_config.show_banner {
                let (width, _) = crate::display::get_terminal_size();
                print_grok_logo(width);
            }
            Ok(Some(true))
        }
        "model" | "models" => {
            if parts.len() > 1 {
                session.model = parts[1].to_string();
                println!(
                    "{} Model changed to: {}",
                    "✓".bright_green(),
                    session.model.bright_cyan()
                );
            } else {
                println!(
                    "{} Current model: {}",
                    "ℹ".bright_blue(),
                    session.model.bright_cyan()
                );
            }
            Ok(Some(true))
        }
        "system" => {
            if parts.len() > 1 {
                let system_prompt = parts[1..].join(" ");
                session.system_prompt = Some(system_prompt.clone());
                println!(
                    "{} System prompt set: {}",
                    "✓".bright_green(),
                    system_prompt.bright_yellow()
                );
            } else {
                match &session.system_prompt {
                    Some(prompt) => println!(
                        "{} Current system prompt: {}",
                        "ℹ".bright_blue(),
                        prompt.bright_yellow()
                    ),
                    None => println!("{} No system prompt set", "ℹ".bright_blue()),
                }
            }
            Ok(Some(true))
        }
        "settings" => {
            crate::cli::commands::settings::handle_settings_action(
                crate::SettingsAction::Show,
                app_config,
            )
            .await?;
            Ok(Some(true))
        }
        "tools" => {
            print_available_tools();
            Ok(Some(true))
        }
        "history" => {
            print_conversation_history(session);
            Ok(Some(true))
        }
        "status" => {
            print_session_status(session);
            Ok(Some(true))
        }
        "version" => {
            println!(
                "{} Grok CLI v{}",
                "ℹ".bright_blue(),
                env!("CARGO_PKG_VERSION")
            );
            Ok(Some(true))
        }
        "reset" => {
            session.conversation_history.clear();
            session.total_tokens_used = 0;
            println!("{} Conversation history cleared", "✓".bright_green());
            Ok(Some(true))
        }
        "save" => {
            if parts.len() < 2 {
                println!("{} Usage: /save <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match save_session(session, name) {
                    Ok(path) => {
                        println!("{} Session saved to {}", "✓".bright_green(), path.display())
                    }
                    Err(e) => println!("{} Failed to save session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "load" => {
            if parts.len() < 2 {
                println!("{} Usage: /load <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match load_session(name) {
                    Ok(loaded_session) => {
                        *session = loaded_session;
                        println!("{} Session '{}' loaded", "✓".bright_green(), name);
                        // Note: Can't show config here as we don't have access to it in this scope
                        println!("  Model: {}", session.model.bright_cyan());
                        println!(
                            "  Directory: {}",
                            session
                                .current_directory
                                .display()
                                .to_string()
                                .bright_yellow()
                        );
                    }
                    Err(e) => println!("{} Failed to load session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "list" | "sessions" => {
            match list_sessions() {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        println!("{} No saved sessions found", "ℹ".bright_blue());
                    } else {
                        println!("{}", "Saved Sessions:".bright_cyan().bold());
                        for s in sessions {
                            println!("  • {}", s);
                        }
                    }
                }
                Err(e) => println!("{} Failed to list sessions: {}", "✗".bright_red(), e),
            }
            Ok(Some(true))
        }
        _ => {
            println!("{} Unknown command: /{}", "⚠".bright_yellow(), parts[0]);
            println!("Type /help for available commands");
            Ok(Some(true))
        }
    }
}

/// Print interactive mode help
fn print_interactive_help() {
    println!("{}", "Interactive mode commands:".bright_cyan().bold());
    println!();

    let commands = vec![
        ("/help, /h", "Show this help message"),
        ("/quit, /exit, /q", "Exit interactive mode"),
        ("/clear, /cls", "Clear screen and show logo"),
        ("/model [name]", "Show or change the current model"),
        ("/system [prompt]", "Show or set system prompt"),
        ("/tools", "List available coding tools"),
        (
            "!<command>",
            "Execute shell command locally (e.g., !dir, !ls -la)",
        ),
        ("/settings", "Open settings menu"),
        ("/history", "Show conversation history"),
        ("/status", "Show session status"),
        ("/version", "Show version info"),
        ("/config", "Show configuration info"),
        ("/reset", "Clear conversation history"),
        ("/save [name]", "Save current session"),
        ("/load [name]", "Load a saved session"),
        ("/list", "List saved sessions"),
    ];

    for (command, description) in commands {
        println!("  {:<20} {}", command.bright_white(), description);
    }
    println!();
    println!("{}", "Just type your message to chat with Grok!".dimmed());
    println!();
}

/// Print available coding tools
fn print_available_tools() {
    use colored::*;

    println!("{}", "Available Coding Tools:".bright_cyan().bold());
    println!();
    println!("{}", "These tools are available when using the ACP server or when Grok needs to perform file operations:".dimmed());
    println!();

    let tools = vec![
        (
            "read_file",
            "Read the content of a file",
            "read_file(path: string)",
        ),
        (
            "write_file",
            "Write content to a file",
            "write_file(path: string, content: string)",
        ),
        (
            "replace",
            "Replace text in a file",
            "replace(path: string, old_string: string, new_string: string)",
        ),
        (
            "list_directory",
            "List files and directories",
            "list_directory(path: string)",
        ),
        (
            "glob_search",
            "Find files matching a pattern",
            "glob_search(pattern: string)",
        ),
        (
            "search_file_content",
            "Search for text in files",
            "search_file_content(path: string, pattern: string)",
        ),
        (
            "run_shell_command",
            "Execute a shell command",
            "run_shell_command(command: string)",
        ),
        ("web_search", "Search the web", "web_search(query: string)"),
        (
            "web_fetch",
            "Fetch content from a URL",
            "web_fetch(url: string)",
        ),
        (
            "save_memory",
            "Save a fact to memory",
            "save_memory(fact: string)",
        ),
    ];

    println!("{}", "File Operations:".bright_yellow().bold());
    for (name, desc, sig) in &tools[0..3] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "File Search & Discovery:".bright_yellow().bold());
    for (name, desc, sig) in &tools[3..6] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Execution & Web:".bright_yellow().bold());
    for (name, desc, sig) in &tools[6..9] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Memory:".bright_yellow().bold());
    for (name, desc, sig) in &tools[9..10] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Note:".bright_cyan());
    println!(
        "  {}",
        "• Tools are automatically used by Grok when needed".dimmed()
    );
    println!(
        "  {}",
        "• For ACP server mode, use: grok acp stdio".dimmed()
    );
    println!(
        "  {}",
        "• All file operations respect security permissions".dimmed()
    );
    println!();
}

/// Print conversation history
fn print_conversation_history(session: &InteractiveSession) {
    if session.conversation_history.is_empty() {
        println!("{} No conversation history yet", "ℹ".bright_blue());
        return;
    }

    println!("{}", "Conversation History:".bright_cyan().bold());
    println!();

    for (i, item) in session.conversation_history.iter().enumerate() {
        let role_color = if item.role == "user" {
            Color::BrightGreen
        } else {
            Color::BrightBlue
        };

        let role_symbol = if item.role == "user" { "👤" } else { "🤖" };

        println!(
            "{} {} {}",
            format!("{}.", i + 1).dimmed(),
            role_symbol,
            item.role.color(role_color).bold()
        );

        // Show first 100 chars of content
        let content_preview = if item.content.len() > 100 {
            format!("{}...", &item.content[..97])
        } else {
            item.content.clone()
        };

        println!("   {}", content_preview);

        if let Some(tokens) = item.tokens_used {
            println!("   {} tokens used", tokens.to_string().dimmed());
        }
        println!();
    }
}

/// Print session status
fn print_session_status(session: &InteractiveSession) {
    println!("{}", "Session Status:".bright_cyan().bold());
    println!("  Session ID: {}", session.session_id.bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Temperature: {}",
        session.temperature.to_string().bright_yellow()
    );
    println!(
        "  Max tokens: {}",
        session.max_tokens.to_string().bright_yellow()
    );
    println!(
        "  Messages: {}",
        session
            .conversation_history
            .len()
            .to_string()
            .bright_green()
    );
    println!(
        "  Total tokens used: {}",
        session.total_tokens_used.to_string().bright_red()
    );
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_magenta()
    );

    if let Some(system) = &session.system_prompt {
        println!("  System prompt: {}", system.bright_green());
    }
    println!();
}

/// Send message to Grok and handle response
async fn send_to_grok(
    client: &GrokClient,
    session: &mut InteractiveSession,
    input: &str,
) -> Result<()> {
    // Add user message to history
    session.add_conversation_item("user", input, None);

    // Show thinking indicator
    print!("{} ", "Thinking...".bright_yellow());
    io::stdout().flush()?;

    // Prepare messages for API
    let mut messages = vec![];

    if let Some(system) = &session.system_prompt {
        messages.push(json!({
            "role": "system",
            "content": system
        }));
    }

    // Add conversation history (keep last 10 messages to avoid context overflow)
    let recent_history = session
        .conversation_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .collect::<Vec<_>>();

    for item in recent_history {
        messages.push(json!({
            "role": item.role,
            "content": item.content
        }));
    }

    // Get tool definitions for function calling
    let tools = tools::get_tool_definitions();

    // Set up security policy with current directory as trusted
    let mut security = SecurityPolicy::new();
    security.add_trusted_directory(&session.current_directory);

    // Send request using the existing client method with tools
    match client
        .chat_completion_with_history(
            &messages,
            session.temperature,
            session.max_tokens,
            &session.model,
            Some(tools),
        )
        .await
    {
        Ok(response_msg) => {
            clear_current_line();

            // Handle tool calls if present
            if let Some(tool_calls) = &response_msg.tool_calls {
                if !tool_calls.is_empty() {
                    println!("{}", "Grok is executing operations...".blue().bold());
                    println!();

                    for tool_call in tool_calls {
                        if let Err(e) = execute_tool_call_interactive(tool_call, &security) {
                            eprintln!("  {} Tool execution failed: {}", "✗".red(), e);
                        }
                    }

                    println!();
                    println!("{}", "All operations completed!".green().bold());
                    println!();

                    // Add assistant's response to history
                    let content = content_to_string(response_msg.content.as_ref());
                    let content = if content.is_empty() {
                        "Operations completed.".to_string()
                    } else {
                        content
                    };
                    session.add_conversation_item("assistant", &content, None);
                    return Ok(());
                }
            }

            let content = content_to_string(response_msg.content.as_ref());

            // Print Grok's response with nice formatting
            println!("{} {}", "🤖".bright_blue(), "Grok:".bright_blue().bold());
            println!();
            println!("{}", content);
            println!();

            // Add to conversation history
            session.add_conversation_item("assistant", &content, None);
        }
        Err(e) => {
            clear_current_line();
            return Err(e);
        }
    }

    Ok(())
}

/// Execute a tool call in interactive mode
fn execute_tool_call_interactive(
    tool_call: &crate::ToolCall,
    security: &SecurityPolicy,
) -> Result<()> {
    use anyhow::anyhow;

    let name = &tool_call.function.name;
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

    match name.as_str() {
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing content"))?;
            let result = tools::write_file(path, content, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = tools::read_file(path, security)?;
            println!(
                "  {} Read {} bytes from {}",
                "✓".green(),
                content.len(),
                path
            );
        }
        "replace" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let old = args["old_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing old_string"))?;
            let new = args["new_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing new_string"))?;
            let expected = args
                .get("expected_replacements")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let result = tools::replace(path, old, new, expected, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "list_directory" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let result = tools::list_directory(path, security)?;
            println!("  {} Directory contents of {}:", "✓".green(), path);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "glob_search" => {
            let pattern = args["pattern"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing pattern"))?;
            let result = tools::glob_search(pattern, security)?;
            println!("  {} Files matching '{}':", "✓".green(), pattern);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "save_memory" => {
            let fact = args["fact"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing fact"))?;
            let result = tools::save_memory(fact)?;
            println!("  {} {}", "✓".green(), result);
        }
        "run_shell_command" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing command"))?;
            println!("  {} Executing: {}", "⚙".cyan(), command);
            let result = tools::run_shell_command(command, security)?;
            println!("  {} Command output:", "✓".green());
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        _ => {
            println!("  {} Unsupported tool: {}", "⚠".yellow(), name);
        }
    }

    Ok(())
}

/// Check if current directory is the home directory
fn is_home_directory(current_dir: &PathBuf) -> bool {
    if let Some(home) = dirs::home_dir() {
        current_dir == &home
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = InteractiveSession::new("grok-4-1-fast-reasoning".to_string(), None);
        assert_eq!(session.model, "grok-4-1-fast-reasoning");
        assert!(session.conversation_history.is_empty());
        assert_eq!(session.total_tokens_used, 0);
    }

    #[test]
    fn test_add_conversation_item() {
        let mut session = InteractiveSession::new("grok-4-1-fast-reasoning".to_string(), None);
        session.add_conversation_item("user", "Hello", Some(10));

        assert_eq!(session.conversation_history.len(), 1);
        assert_eq!(session.total_tokens_used, 10);
        assert_eq!(session.conversation_history[0].content, "Hello");
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(id1.starts_with("grok-"));
        assert!(id2.starts_with("grok-"));
        assert_ne!(id1, id2); // Should be different due to timestamp
    }
}

```


## Assistant

<thinking>
**Verifying Path References**

I'm currently working on refactoring the crate paths within `src/display/interactive.rs`. I'm evaluating whether to use absolute paths (`crate::skills::...`) or relative paths with a `use` statement. I've already tested the absolute path approach and now need to double-check that the `use` statement approach also works as expected.


**Incorporating Skills Data**

I'm now incorporating skill loading information into the session display in `src/display/interactive.rs`. I've verified that `crate::skills` is accessible and have added the necessary `use` statement. My next step involves modifying the `print_session_info` function to show the loaded skills, which I've started by adding code that displays skill names, with the number of skills, if any, also displayed.



</thinking>

**Tool Call: src\display\interactive.rs**
Status: Completed

Read lines 251-350 of 1275 from src/display/interactive.rs

## Assistant

<thinking>
**Analyzing Skill Integration**

I've recently decided to incorporate the skill loading details within the `print_session_info` function. My focus is now on efficiently displaying context file information and ensuring the loaded skills are accurately represented. Currently, I'm working to integrate the skill loading data.


**Incorporating Functionality Details**

My priority has shifted toward integrating the skill loading data within the `print_session_info` function. I've been focusing on adding the skill loading info in `print_session_info`, specifically within the existing block, to ensure context files are displayed correctly and skills are loaded, reflected in the output.



</thinking>

**Tool Call: src\display\interactive.rs:             }... =>             }...**
Status: Completed

Diff: interactive.rs
```
//! Interactive mode for Grok CLI
//!
//! Provides a Gemini CLI-like interactive experience with persistent sessions,
//! input prompts, and real-time status display

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::Result;
use colored::*;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::{Duration, sleep};

use crate::GrokClient;
use crate::acp::security::SecurityPolicy;
use crate::acp::tools;
use crate::config::Config;
use crate::display::{
    BannerConfig, clear_current_line, print_directory_recommendation, print_grok_logo,
    print_welcome_banner,
};
use crate::utils::context::{
    format_context_for_prompt, get_all_context_file_paths, load_and_merge_project_context,
};
use crate::utils::session::{list_sessions, load_session, save_session};
use crate::utils::shell_permissions::{ApprovalMode, ShellPermissions};
use crate::{content_to_string, extract_text_content, text_content};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Interactive session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSession {
    pub session_id: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub conversation_history: Vec<ConversationItem>,
    pub current_directory: PathBuf,
    pub show_context_usage: bool,
    pub total_tokens_used: u32,
}

/// Conversation item in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItem {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: Option<u32>,
}

/// Interactive mode configuration
#[derive(Debug, Clone)]
pub struct InteractiveConfig {
    pub show_banner: bool,
    pub show_tips: bool,
    pub show_status: bool,
    pub auto_save_session: bool,
    pub prompt_style: PromptStyle,
    pub check_directory: bool,
}

/// Different prompt styles
#[derive(Debug, Clone, PartialEq)]
pub enum PromptStyle {
    Simple,
    Rich,
    Minimal,
}

impl Default for InteractiveConfig {
    fn default() -> Self {
        Self {
            show_banner: true,
            show_tips: true,
            show_status: true,
            auto_save_session: false,
            prompt_style: PromptStyle::Rich,
            check_directory: true,
        }
    }
}

impl InteractiveSession {
    /// Create a new interactive session
    pub fn new(model: String, system_prompt: Option<String>) -> Self {
        let session_id = generate_session_id();
        let current_directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            session_id,
            model,
            temperature: 0.7,
            max_tokens: 4096,
            system_prompt,
            conversation_history: Vec::new(),
            current_directory,
            show_context_usage: true,
            total_tokens_used: 0,
        }
    }

    /// Add a conversation item to the history
    pub fn add_conversation_item(&mut self, role: &str, content: &str, tokens_used: Option<u32>) {
        let item = ConversationItem {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            tokens_used,
        };

        if let Some(tokens) = tokens_used {
            self.total_tokens_used += tokens;
        }

        self.conversation_history.push(item);
    }

    /// Get context usage information
    pub fn get_context_info(&self) -> String {
        let conversation_count = self.conversation_history.len();
        let context_percentage = if self.total_tokens_used > 0 {
            let estimated_max = 8192; // Rough estimate for context window
            ((self.total_tokens_used as f32 / estimated_max as f32) * 100.0) as u8
        } else {
            0
        };

        format!(
            "{}% context left | {} messages",
            100 - context_percentage,
            conversation_count
        )
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("grok-{}", timestamp)
}

/// Start the interactive mode
pub async fn start_interactive_mode(
    api_key: &str,
    model: &str,
    config: &Config,
    interactive_config: InteractiveConfig,
) -> Result<()> {
    // Load project context if available
    let mut project_context = load_project_context_for_session(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );

    // Load skills context
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
            if !skills_context.is_empty() {
                let ctx = project_context.get_or_insert_with(String::new);
                ctx.push_str(&skills_context);
            }
        }
    }

    let mut session = InteractiveSession::new(model.to_string(), project_context);
    let client = GrokClient::new(api_key)?;

    // Display startup elements
    if interactive_config.show_banner {
        display_startup_screen(&interactive_config, &session, config).await?;
    }

    // Check if running in home directory
    if interactive_config.check_directory && is_home_directory(&session.current_directory) {
        let banner_config = BannerConfig::default();
        print_directory_recommendation(
            &session.current_directory.display().to_string(),
            &banner_config,
        );
    }

    // Main interactive loop
    loop {
        match run_interactive_loop(&mut session, &client, &interactive_config, config).await {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Error:".red(), e);
                continue;
            }
        }
    }

    // Goodbye message
    println!("{}", "\n👋 Thanks for using Grok CLI!".bright_cyan());

    if interactive_config.auto_save_session && !session.conversation_history.is_empty() {
        println!("{}", "Session saved for future reference.".dimmed());
    }

    Ok(())
}

/// Display the startup screen
async fn display_startup_screen(
    config: &InteractiveConfig,
    session: &InteractiveSession,
    app_config: &Config,
) -> Result<()> {
    let (width, _) = crate::display::get_terminal_size();

    // Clear screen and show logo with animation
    crate::display::clear_screen();

    if config.show_banner && !config.show_tips {
        print_grok_logo(width);
        sleep(Duration::from_millis(500)).await;
    }

    if config.show_tips {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: Some(width),
        };
        print_welcome_banner(&banner_config);
    }

    // Show current session info
    if config.show_status {
        print_session_info(session, app_config);
    }

    Ok(())
}

/// Print current session information
fn print_session_info(session: &InteractiveSession, config: &Config) {
    println!("{}", "Current session:".bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_yellow()
    );

    // Show config source
    if let Some(source) = &config.config_source {
        println!("  Configuration: {}", source.display().bright_magenta());
    }

    // Show context files info if loaded
    let context_paths = get_all_context_file_paths(&session.current_directory);
    if !context_paths.is_empty() {
        if context_paths.len() == 1 {
            println!(
                "  Context loaded: {}",
                context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .bright_green()
            );
        } else {
            println!(
                "  Context loaded: {} files",
                context_paths.len().to_string().bright_green()
            );
            for path in &context_paths {
                println!(
                    "    - {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .dimmed()
                );
            }
        }
    }

    // Show loaded skills
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        if let Ok(skills) = crate::skills::list_skills(&skills_dir) {
            if !skills.is_empty() {
                println!(
                    "  Skills: {}",
                    format!("{} loaded", skills.len()).bright_green()
                );
                for skill in skills {
                    println!("    - {}", skill.config.name.dimmed());
                }
            }
        }
    }

    if let Some(system) = &session.system_prompt {
        let preview = if system.len() > 60 {
            format!("{}...", &system[..60])
        } else {
            system.clone()
        };
        println!("  System prompt: {}", preview.bright_green());
    }
    println!();
}

/// Load project context for a new session
fn load_project_context_for_session(project_root: &PathBuf) -> Option<String> {
    match load_and_merge_project_context(project_root) {
        Ok(Some(context)) => {
            let formatted = format_context_for_prompt(&context);
            let context_paths = get_all_context_file_paths(project_root);

            if context_paths.is_empty() {
                // Shouldn't happen but handle gracefully
                return Some(formatted);
            }

            if context_paths.len() == 1 {
                let context_file_name = context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_os_string().into_string().ok())
                    .unwrap_or_else(|| "context file".to_string());

                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded project context from {}", context_file_name).dimmed()
                );
            } else {
                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded and merged {} context files", context_paths.len()).dimmed()
                );
                for path in &context_paths {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        println!("  {} {}", "•".dimmed(), name.dimmed());
                    }
                }
            }

            Some(formatted)
        }
        Ok(None) => {
            // No context file found - this is normal
            None
        }
        Err(e) => {
            eprintln!(
                "{} Failed to load project context: {}",
                "⚠".yellow(),
                e.to_string().dimmed()
            );
            None
        }
    }
}

use crate::display::components::input::{Suggestion, read_input_with_suggestions};

/// Main interactive loop
async fn run_interactive_loop(
    session: &mut InteractiveSession,
    client: &GrokClient,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<bool> {
    // Prepare prompt
    let prompt = match interactive_config.prompt_style {
        PromptStyle::Simple => format!("{} ", ">".bright_cyan()),
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            format!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            )
        }
        PromptStyle::Minimal => "» ".to_string(),
    };

    // Prepare suggestions
    let suggestions = vec![
        Suggestion {
            text: "/clear".to_string(),
            description: "Clear screen".to_string(),
        },
        Suggestion {
            text: "/help".to_string(),
            description: "Show help message".to_string(),
        },
        Suggestion {
            text: "/history".to_string(),
            description: "Show history".to_string(),
        },
        Suggestion {
            text: "/list".to_string(),
            description: "List saved sessions".to_string(),
        },
        Suggestion {
            text: "/load".to_string(),
            description: "Load a session".to_string(),
        },
        Suggestion {
            text: "/model".to_string(),
            description: "Change model".to_string(),
        },
        Suggestion {
            text: "/quit".to_string(),
            description: "Exit interactive mode".to_string(),
        },
        Suggestion {
            text: "/reset".to_string(),
            description: "Reset session".to_string(),
        },
        Suggestion {
            text: "/save".to_string(),
            description: "Save current session".to_string(),
        },
        Suggestion {
            text: "/settings".to_string(),
            description: "Open settings".to_string(),
        },
        Suggestion {
            text: "/status".to_string(),
            description: "Show status".to_string(),
        },
        Suggestion {
            text: "/system".to_string(),
            description: "Set system prompt".to_string(),
        },
        Suggestion {
            text: "/tools".to_string(),
            description: "List coding tools".to_string(),
        },
        Suggestion {
            text: "/version".to_string(),
            description: "Show version info".to_string(),
        },
        Suggestion {
            text: "/config".to_string(),
            description: "Show configuration info".to_string(),
        },
        Suggestion {
            text: "!ls".to_string(),
            description: "List files (shell command)".to_string(),
        },
        Suggestion {
            text: "!dir".to_string(),
            description: "List files on Windows (shell command)".to_string(),
        },
        Suggestion {
            text: "!git status".to_string(),
            description: "Check git status (shell command)".to_string(),
        },
        Suggestion {
            text: "!pwd".to_string(),
            description: "Print working directory (shell command)".to_string(),
        },
    ];

    // Read user input
    // Note: We're running blocking TUI code in an async context, which is generally bad,
    // but for a CLI it's acceptable as we're awaiting user input anyway.
    let input =
        tokio::task::spawn_blocking(move || read_input_with_suggestions(&prompt, &suggestions))
            .await??;

    let input = input.trim();

    // Handle empty input
    if input.is_empty() {
        return Ok(true);
    }

    // Handle shell commands (starting with !)
    if input.starts_with('!') {
        return handle_shell_command(input).await;
    }

    // Handle special commands
    if let Some(command_result) =
        handle_special_commands(input, session, interactive_config, app_config).await?
    {
        return Ok(command_result);
    }

    // Send to Grok API
    match send_to_grok(client, session, input).await {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("{} Failed to get response: {}", "Error:".red(), e);
            Ok(true)
        }
    }
}

/// Display the input prompt
fn display_prompt(session: &InteractiveSession, config: &InteractiveConfig) -> Result<()> {
    match config.prompt_style {
        PromptStyle::Simple => {
            print!("{} ", ">".bright_cyan());
        }
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            print!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            );
        }
        PromptStyle::Minimal => {
            print!("» ");
        }
    }

    io::stdout().flush()?;
    Ok(())
}

/// Read user input from stdin
fn read_user_input() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

/// Handle shell commands (those starting with !)
async fn handle_shell_command(input: &str) -> Result<bool> {
    let command = input.trim_start_matches('!').trim();

    if command.is_empty() {
        println!("{}", "Error: No command specified".red());
        return Ok(true);
    }

    // Create permissions manager (TODO: pass from session state)
    let mut permissions = ShellPermissions::new(ApprovalMode::Default);

    // Check if command should be executed
    match permissions.should_execute(command) {
        Ok(true) => {
            // Permission granted, execute command
            println!();
            println!("{} {}", "Executing:".bright_cyan(), command.bright_yellow());
            println!();

            // Determine shell based on OS
            #[cfg(target_os = "windows")]
            let shell = "cmd";
            #[cfg(target_os = "windows")]
            let shell_arg = "/C";

            #[cfg(not(target_os = "windows"))]
            let shell = "sh";
            #[cfg(not(target_os = "windows"))]
            let shell_arg = "-c";

            // Execute the command
            match std::process::Command::new(shell)
                .arg(shell_arg)
                .arg(command)
                .output()
            {
                Ok(output) => {
                    // Print stdout
                    if !output.stdout.is_empty() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        print!("{}", stdout);
                    }

                    // Print stderr in red
                    if !output.stderr.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprint!("{}", stderr.red());
                    }

                    // Show exit code if non-zero
                    if !output.status.success() {
                        println!();
                        println!(
                            "{} Command exited with code: {}",
                            "⚠".yellow(),
                            output.status.code().unwrap_or(-1)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to execute command: {}", "Error:".red(), e);
                }
            }

            println!();
        }
        Ok(false) => {
            // Permission denied
            println!();
            println!("{}", "Command execution cancelled".yellow());
            println!();
        }
        Err(e) => {
            eprintln!("{} Permission check failed: {}", "Error:".red(), e);
        }
    }

    Ok(true)
}

/// Handle special commands (those starting with /)
async fn handle_special_commands(
    input: &str,
    session: &mut InteractiveSession,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<Option<bool>> {
    if !input.starts_with('/') {
        return Ok(None);
    }

    let command = input.trim_start_matches('/').trim();
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(Some(true));
    }

    match parts[0] {
        "help" | "h" => {
            print_interactive_help();
            Ok(Some(true))
        }
        "quit" | "exit" | "q" => Ok(Some(false)),
        "clear" | "cls" => {
            crate::display::clear_screen();
            if interactive_config.show_banner {
                let (width, _) = crate::display::get_terminal_size();
                print_grok_logo(width);
            }
            Ok(Some(true))
        }
        "model" | "models" => {
            if parts.len() > 1 {
                session.model = parts[1].to_string();
                println!(
                    "{} Model changed to: {}",
                    "✓".bright_green(),
                    session.model.bright_cyan()
                );
            } else {
                println!(
                    "{} Current model: {}",
                    "ℹ".bright_blue(),
                    session.model.bright_cyan()
                );
            }
            Ok(Some(true))
        }
        "system" => {
            if parts.len() > 1 {
                let system_prompt = parts[1..].join(" ");
                session.system_prompt = Some(system_prompt.clone());
                println!(
                    "{} System prompt set: {}",
                    "✓".bright_green(),
                    system_prompt.bright_yellow()
                );
            } else {
                match &session.system_prompt {
                    Some(prompt) => println!(
                        "{} Current system prompt: {}",
                        "ℹ".bright_blue(),
                        prompt.bright_yellow()
                    ),
                    None => println!("{} No system prompt set", "ℹ".bright_blue()),
                }
            }
            Ok(Some(true))
        }
        "settings" => {
            crate::cli::commands::settings::handle_settings_action(
                crate::SettingsAction::Show,
                app_config,
            )
            .await?;
            Ok(Some(true))
        }
        "tools" => {
            print_available_tools();
            Ok(Some(true))
        }
        "history" => {
            print_conversation_history(session);
            Ok(Some(true))
        }
        "status" => {
            print_session_status(session);
            Ok(Some(true))
        }
        "version" => {
            println!(
                "{} Grok CLI v{}",
                "ℹ".bright_blue(),
                env!("CARGO_PKG_VERSION")
            );
            Ok(Some(true))
        }
        "reset" => {
            session.conversation_history.clear();
            session.total_tokens_used = 0;
            println!("{} Conversation history cleared", "✓".bright_green());
            Ok(Some(true))
        }
        "save" => {
            if parts.len() < 2 {
                println!("{} Usage: /save <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match save_session(session, name) {
                    Ok(path) => {
                        println!("{} Session saved to {}", "✓".bright_green(), path.display())
                    }
                    Err(e) => println!("{} Failed to save session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "load" => {
            if parts.len() < 2 {
                println!("{} Usage: /load <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match load_session(name) {
                    Ok(loaded_session) => {
                        *session = loaded_session;
                        println!("{} Session '{}' loaded", "✓".bright_green(), name);
                        // Note: Can't show config here as we don't have access to it in this scope
                        println!("  Model: {}", session.model.bright_cyan());
                        println!(
                            "  Directory: {}",
                            session
                                .current_directory
                                .display()
                                .to_string()
                                .bright_yellow()
                        );
                    }
                    Err(e) => println!("{} Failed to load session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "list" | "sessions" => {
            match list_sessions() {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        println!("{} No saved sessions found", "ℹ".bright_blue());
                    } else {
                        println!("{}", "Saved Sessions:".bright_cyan().bold());
                        for s in sessions {
                            println!("  • {}", s);
                        }
                    }
                }
                Err(e) => println!("{} Failed to list sessions: {}", "✗".bright_red(), e),
            }
            Ok(Some(true))
        }
        _ => {
            println!("{} Unknown command: /{}", "⚠".bright_yellow(), parts[0]);
            println!("Type /help for available commands");
            Ok(Some(true))
        }
    }
}

/// Print interactive mode help
fn print_interactive_help() {
    println!("{}", "Interactive mode commands:".bright_cyan().bold());
    println!();

    let commands = vec![
        ("/help, /h", "Show this help message"),
        ("/quit, /exit, /q", "Exit interactive mode"),
        ("/clear, /cls", "Clear screen and show logo"),
        ("/model [name]", "Show or change the current model"),
        ("/system [prompt]", "Show or set system prompt"),
        ("/tools", "List available coding tools"),
        (
            "!<command>",
            "Execute shell command locally (e.g., !dir, !ls -la)",
        ),
        ("/settings", "Open settings menu"),
        ("/history", "Show conversation history"),
        ("/status", "Show session status"),
        ("/version", "Show version info"),
        ("/config", "Show configuration info"),
        ("/reset", "Clear conversation history"),
        ("/save [name]", "Save current session"),
        ("/load [name]", "Load a saved session"),
        ("/list", "List saved sessions"),
    ];

    for (command, description) in commands {
        println!("  {:<20} {}", command.bright_white(), description);
    }
    println!();
    println!("{}", "Just type your message to chat with Grok!".dimmed());
    println!();
}

/// Print available coding tools
fn print_available_tools() {
    use colored::*;

    println!("{}", "Available Coding Tools:".bright_cyan().bold());
    println!();
    println!("{}", "These tools are available when using the ACP server or when Grok needs to perform file operations:".dimmed());
    println!();

    let tools = vec![
        (
            "read_file",
            "Read the content of a file",
            "read_file(path: string)",
        ),
        (
            "write_file",
            "Write content to a file",
            "write_file(path: string, content: string)",
        ),
        (
            "replace",
            "Replace text in a file",
            "replace(path: string, old_string: string, new_string: string)",
        ),
        (
            "list_directory",
            "List files and directories",
            "list_directory(path: string)",
        ),
        (
            "glob_search",
            "Find files matching a pattern",
            "glob_search(pattern: string)",
        ),
        (
            "search_file_content",
            "Search for text in files",
            "search_file_content(path: string, pattern: string)",
        ),
        (
            "run_shell_command",
            "Execute a shell command",
            "run_shell_command(command: string)",
        ),
        ("web_search", "Search the web", "web_search(query: string)"),
        (
            "web_fetch",
            "Fetch content from a URL",
            "web_fetch(url: string)",
        ),
        (
            "save_memory",
            "Save a fact to memory",
            "save_memory(fact: string)",
        ),
    ];

    println!("{}", "File Operations:".bright_yellow().bold());
    for (name, desc, sig) in &tools[0..3] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "File Search & Discovery:".bright_yellow().bold());
    for (name, desc, sig) in &tools[3..6] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Execution & Web:".bright_yellow().bold());
    for (name, desc, sig) in &tools[6..9] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Memory:".bright_yellow().bold());
    for (name, desc, sig) in &tools[9..10] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Note:".bright_cyan());
    println!(
        "  {}",
        "• Tools are automatically used by Grok when needed".dimmed()
    );
    println!(
        "  {}",
        "• For ACP server mode, use: grok acp stdio".dimmed()
    );
    println!(
        "  {}",
        "• All file operations respect security permissions".dimmed()
    );
    println!();
}

/// Print conversation history
fn print_conversation_history(session: &InteractiveSession) {
    if session.conversation_history.is_empty() {
        println!("{} No conversation history yet", "ℹ".bright_blue());
        return;
    }

    println!("{}", "Conversation History:".bright_cyan().bold());
    println!();

    for (i, item) in session.conversation_history.iter().enumerate() {
        let role_color = if item.role == "user" {
            Color::BrightGreen
        } else {
            Color::BrightBlue
        };

        let role_symbol = if item.role == "user" { "👤" } else { "🤖" };

        println!(
            "{} {} {}",
            format!("{}.", i + 1).dimmed(),
            role_symbol,
            item.role.color(role_color).bold()
        );

        // Show first 100 chars of content
        let content_preview = if item.content.len() > 100 {
            format!("{}...", &item.content[..97])
        } else {
            item.content.clone()
        };

        println!("   {}", content_preview);

        if let Some(tokens) = item.tokens_used {
            println!("   {} tokens used", tokens.to_string().dimmed());
        }
        println!();
    }
}

/// Print session status
fn print_session_status(session: &InteractiveSession) {
    println!("{}", "Session Status:".bright_cyan().bold());
    println!("  Session ID: {}", session.session_id.bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Temperature: {}",
        session.temperature.to_string().bright_yellow()
    );
    println!(
        "  Max tokens: {}",
        session.max_tokens.to_string().bright_yellow()
    );
    println!(
        "  Messages: {}",
        session
            .conversation_history
            .len()
            .to_string()
            .bright_green()
    );
    println!(
        "  Total tokens used: {}",
        session.total_tokens_used.to_string().bright_red()
    );
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_magenta()
    );

    if let Some(system) = &session.system_prompt {
        println!("  System prompt: {}", system.bright_green());
    }
    println!();
}

/// Send message to Grok and handle response
async fn send_to_grok(
    client: &GrokClient,
    session: &mut InteractiveSession,
    input: &str,
) -> Result<()> {
    // Add user message to history
    session.add_conversation_item("user", input, None);

    // Show thinking indicator
    print!("{} ", "Thinking...".bright_yellow());
    io::stdout().flush()?;

    // Prepare messages for API
    let mut messages = vec![];

    if let Some(system) = &session.system_prompt {
        messages.push(json!({
            "role": "system",
            "content": system
        }));
    }

    // Add conversation history (keep last 10 messages to avoid context overflow)
    let recent_history = session
        .conversation_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .collect::<Vec<_>>();

    for item in recent_history {
        messages.push(json!({
            "role": item.role,
            "content": item.content
        }));
    }

    // Get tool definitions for function calling
    let tools = tools::get_tool_definitions();

    // Set up security policy with current directory as trusted
    let mut security = SecurityPolicy::new();
    security.add_trusted_directory(&session.current_directory);

    // Send request using the existing client method with tools
    match client
        .chat_completion_with_history(
            &messages,
            session.temperature,
            session.max_tokens,
            &session.model,
            Some(tools),
        )
        .await
    {
        Ok(response_msg) => {
            clear_current_line();

            // Handle tool calls if present
            if let Some(tool_calls) = &response_msg.tool_calls {
                if !tool_calls.is_empty() {
                    println!("{}", "Grok is executing operations...".blue().bold());
                    println!();

                    for tool_call in tool_calls {
                        if let Err(e) = execute_tool_call_interactive(tool_call, &security) {
                            eprintln!("  {} Tool execution failed: {}", "✗".red(), e);
                        }
                    }

                    println!();
                    println!("{}", "All operations completed!".green().bold());
                    println!();

                    // Add assistant's response to history
                    let content = content_to_string(response_msg.content.as_ref());
                    let content = if content.is_empty() {
                        "Operations completed.".to_string()
                    } else {
                        content
                    };
                    session.add_conversation_item("assistant", &content, None);
                    return Ok(());
                }
            }

            let content = content_to_string(response_msg.content.as_ref());

            // Print Grok's response with nice formatting
            println!("{} {}", "🤖".bright_blue(), "Grok:".bright_blue().bold());
            println!();
            println!("{}", content);
            println!();

            // Add to conversation history
            session.add_conversation_item("assistant", &content, None);
        }
        Err(e) => {
            clear_current_line();
            return Err(e);
        }
    }

    Ok(())
}

/// Execute a tool call in interactive mode
fn execute_tool_call_interactive(
    tool_call: &crate::ToolCall,
    security: &SecurityPolicy,
) -> Result<()> {
    use anyhow::anyhow;

    let name = &tool_call.function.name;
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

    match name.as_str() {
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing content"))?;
            let result = tools::write_file(path, content, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = tools::read_file(path, security)?;
            println!(
                "  {} Read {} bytes from {}",
                "✓".green(),
                content.len(),
                path
            );
        }
        "replace" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let old = args["old_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing old_string"))?;
            let new = args["new_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing new_string"))?;
            let expected = args
                .get("expected_replacements")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let result = tools::replace(path, old, new, expected, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "list_directory" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let result = tools::list_directory(path, security)?;
            println!("  {} Directory contents of {}:", "✓".green(), path);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "glob_search" => {
            let pattern = args["pattern"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing pattern"))?;
            let result = tools::glob_search(pattern, security)?;
            println!("  {} Files matching '{}':", "✓".green(), pattern);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "save_memory" => {
            let fact = args["fact"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing fact"))?;
            let result = tools::save_memory(fact)?;
            println!("  {} {}", "✓".green(), result);
        }
        "run_shell_command" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing command"))?;
            println!("  {} Executing: {}", "⚙".cyan(), command);
            let result = tools::run_shell_command(command, security)?;
            println!("  {} Command output:", "✓".green());
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        _ => {
            println!("  {} Unsupported tool: {}", "⚠".yellow(), name);
        }
    }

    Ok(())
}

/// Check if current directory is the home directory
fn is_home_directory(current_dir: &PathBuf) -> bool {
    if let Some(home) = dirs::home_dir() {
        current_dir == &home
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = InteractiveSession::new("grok-4-1-fast-reasoning".to_string(), None);
        assert_eq!(session.model, "grok-4-1-fast-reasoning");
        assert!(session.conversation_history.is_empty());
        assert_eq!(session.total_tokens_used, 0);
    }

    #[test]
    fn test_add_conversation_item() {
        let mut session = InteractiveSession::new("grok-4-1-fast-reasoning".to_string(), None);
        session.add_conversation_item("user", "Hello", Some(10));

        assert_eq!(session.conversation_history.len(), 1);
        assert_eq!(session.total_tokens_used, 10);
        assert_eq!(session.conversation_history[0].content, "Hello");
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(id1.starts_with("grok-"));
        assert!(id2.starts_with("grok-"));
        assert_ne!(id1, id2); // Should be different due to timestamp
    }
}

```


## Assistant

<thinking>
**Developing Skill Management**

I've recently finalized the core skill management system, including the `src/skills/` module. The `grok skills` CLI command is operational, allowing me to list, display, and initiate skill creation. I've also incorporated skill loading into the `interactive` mode, ensuring skills automatically integrate into the system prompt.


**Confirming Implementation Details**

I just verified the `src/skills/` module and the `grok skills` CLI command by building the project. Additionally, the loaded skills now display correctly within the session info during interactive mode, as planned.



</thinking>

**Tool Call: cargo build --quiet [current working directory H:\GitHub\grok-cli]**
Status: Completed

error[E0425]: cannot find function `get_skills_context` in module `crate::skills`
   --> src\display\interactive.rs:167:52
    |
167 | ...::skills::get_skills_context(&skills_dir) {
    |              ^^^^^^^^^^^^^^^^^^ not found in `crate::skills`
    |
help: consider importing this function
    |
 11 + use crate::skills::manager::get_skills_context;
    |
help: if you import `get_skills_context`, refer to it directly
    |
167 -         if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
167 +         if let Ok(skills_context) = get_skills_context(&skills_dir) {
    |

error[E0282]: type annotations needed
   --> src\display\interactive.rs:168:17
    |
168 |             if !skills_context.is_empty() {
    |                 ^^^^^^^^^^^^^^ cannot infer type

Some errors have detailed explanations: E0282, E0425.
For more information about an error, try `rustc --explain E0282`.
error: could not compile `grok-cli` (lib) due to 2 previous errors

## Assistant

<thinking>
**Inspecting Module's Export**

Okay, I've just realized I messed up and need to fix the export in `src/skills/mod.rs`. Looks like `get_skills_context` was overlooked. Time to take a closer look at the module's structure to confirm this, and then address the omission.



</thinking>

**Tool Call: src\skills\mod.rs**
Status: Completed

## Assistant

<thinking>
**Listing Modifications**

I've just added `get_skills_context` to the export list. It should be available now.



</thinking>

**Tool Call: src\skills\mod.rs: // Re-export common types... => // Re-export common types...**
Status: Completed

Diff: mod.rs
```
pub mod config;
pub mod manager;

// Re-export common types
pub use config::{Skill, SkillConfig};
pub use manager::{find_skill, get_default_skills_dir, get_skills_context, list_skills, load_skill};

```


## Assistant

<thinking>
**Reconciling Function Names**

I've been meticulously comparing the function names. It seems I made a mistake, and need to change `find_skill` to match the original, and ensure the other functions are correct. Now I'm carefully reviewing the file for accurate naming.


**Confirming the Changes**

I've just confirmed the function name discrepancies. It appears my edits were correct after all, incorporating `get_skills_context` without issue, and keeping `find_skill` as is. This aligns with the original code. I'm now proceeding to rebuild, confident that the naming conventions are now aligned.



</thinking>

**Tool Call: cargo build --quiet [current working directory H:\GitHub\grok-cli]**
Status: Completed

## Assistant

<thinking>
**Evaluating Test Results**

Okay, so I got it to build. Now I'm shifting to testing. First, a skills list check to ensure a clean slate. Next, I'm adding a "test-skill". Then I'll check the list again, and finally, I'll check it in interactive mode, making sure it loads up properly. Let's see how this unfolds.


**Beginning Test Sequence**

The build was a success! Now, I'm commencing the testing phase. Starting with an empty skills list using `grok skills list`, as a baseline. After this verification, I'll introduce the `test-skill` and then re-evaluate the skills list. Lastly, I will dive into interactive mode for final confirmation.



</thinking>

**Tool Call: ./target/debug/grok skills list [current working directory H:\GitHub\grok-cli]**
Status: Completed

2026-02-06T03:23:47.982199Z  INFO grok_cli::utils::chat_logger: Chat logger initialized: "C:\\Users\\johnm\\.grok\\logs\\chat_sessions"
2026-02-06T03:23:47.984583Z  INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
2026-02-06T03:23:47.984942Z  INFO grok_cli::cli::app: ✓ Configuration loaded successfully
No skills directory found at C:\Users\johnm\.grok\skills. Use 'grok skills new <name>' to create one.

## Assistant

<thinking>
**Creating a Skill**

Okay, I'm now focusing on the creation of a new skill. I have a firm grasp of the fundamental concepts. My attention is now fully dedicated to defining the skill's specific capabilities and functionalities. I am also currently pondering over the most effective way to implement this skill.



</thinking>

**Tool Call: ./target/debug/grok skills new my-first-skill [current working directory H:\GitHub\grok-cli]**
Status: Completed

2026-02-06T03:24:23.948160Z  INFO grok_cli::utils::chat_logger: Chat logger initialized: "C:\\Users\\johnm\\.grok\\logs\\chat_sessions"
2026-02-06T03:24:23.950141Z  INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
2026-02-06T03:24:23.950343Z  INFO grok_cli::cli::app: ✓ Configuration loaded successfully
Created new skill 'my-first-skill' at C:\Users\johnm\.grok\skills\my-first-skill

## Assistant

<thinking>
**Revisiting the Core Ideas**

I'm currently wrestling with the core concepts again, ensuring they remain robust. Refining the initial framework is key. I'm scrutinizing the relationships between the elements to make sure that they align and work well with each other. This stage is about strengthening the foundational aspects before I move forward.



</thinking>

**Tool Call: ./target/debug/grok skills list [current working directory H:\GitHub\grok-cli]**
Status: Completed

2026-02-06T03:24:59.378130Z  INFO grok_cli::utils::chat_logger: Chat logger initialized: "C:\\Users\\johnm\\.grok\\logs\\chat_sessions"
2026-02-06T03:24:59.379994Z  INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
2026-02-06T03:24:59.380174Z  INFO grok_cli::cli::app: ✓ Configuration loaded successfully
Available Skills:
  • my-first-skill - Description for my-first-skill

## Assistant

<thinking>
**Assessing Interactive Mode**

I'm currently trying to verify the skill's interactive functionality. I'm running `grok interactive` to observe the banner and status output, hoping to confirm it loads correctly. My plan is to exit immediately after it runs if the output looks promising.


**Evaluating Load and Exit**

I've decided to test the `grok interactive` function by piping `/quit` to it. I want to see if the skill loads successfully and exits without looping.



</thinking>

**Tool Call: echo "/quit" | ./target/debug/grok interactive [current working directory H:\GitHub\grok-cli]**
Status: Completed

  ░██████╗░██████╗░░█████╗░██╗░░██╗
  ██╔════╝░██╔══██╗██╔══██╗██║░██╔╝
  ██║░░██╗░██████╔╝██║░░██║█████═╝░
  ██║░░╚██╗██╔══██╗██║░░██║██╔═██╗░
  ╚██████╔╝██║░░██║╚█████╔╝██║░╚██╗
  ░╚═════╝░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝
                                     v0.1.3

┌──────────────────────────────────────────────────────────────────────────┐    
│ Tips for getting started:                                                │    
│ 1. Ask questions, edit files, or run commands.                           │    
│ 2. Be specific for the best results.                                     │    
│ 3. /help for more information.                                           │    
└──────────────────────────────────────────────────────────────────────────┘    

Current session:
  Model: grok-code-fast-1
  Directory: H:\GitHub\grok-cli
  Configuration: project (H:\GitHub\grok-cli\.grok\.env)
  Context loaded: 5 files
    - rules
    - CONTEXT.md
    - context.md
    - CONTEXT.md
    - memory.md
  Skills: 1 loaded
    - my-first-skill
  System prompt:

## Project Context

The following context has been loaded ...

╰──────────────────────────────────────────────────────────────────────╯        

## Assistant

<thinking>
**Validating the Skills Loading**

I've confirmed the skill loading mechanism is operational within the interactive mode. I've successfully implemented the `src/skills/` module and the `grok skills` command. This should enable access and control of the skills.


**Finalizing the Skill Module**

I've verified the skill loading process and the `grok skills` command. The skill context loading within interactive mode is fully functional, and I'm satisfied with the integration. I've now completed the requested tasks, including the skill module, CLI, and interactive mode integration. To keep things tidy, I'll remove the example skill I created. The entire system is now operational.



</thinking>

**Tool Call: rm -r C:\Users\johnm\.grok\skills\my-first-skill [current working directory H:\GitHub\grok-cli]**
Status: Completed

## Assistant

<thinking>
**Concluding and Checking Thoroughly**

I've completed the initial phase and now it's time to refine. My memory banks are screaming about Clippy, so I'll initiate that check immediately. Further, I recall needing to document the `SkillConfig` and `Skill` structures, which I will add soon. After the Clippy check, I'll move on to documentation.



</thinking>

**Tool Call: cargo clippy -- -D warnings [current working directory H:\GitHub\grok-cli]**
Status: Completed

    Checking tokio v1.49.0
    Checking tokio-util v0.7.18
    Checking tower v0.5.3
    Checking tokio-native-tls v0.3.1
    Checking tower-http v0.6.8
    Checking h2 v0.4.13
    Checking hyper v1.8.1
    Checking hyper-util v0.1.20
    Checking hyper-tls v0.6.0
    Checking reqwest v0.12.28
    Checking reqwest v0.13.1
    Checking grok_api v0.1.0
    Checking grok-cli v0.1.3 (H:\GitHub\grok-cli)
error: you should consider adding a `Default` implementation for `SecurityPolicy`
  --> src\acp\security.rs:12:5
   |
12 | /     pub fn new() -> Self {
13 | |         let working_directory = std::env::current_dir().unwrap_or_els...
14 | |         Self {
15 | |             trusted_directories: Vec::new(),
...  |
18 | |     }
   | |_____^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#new_without_default
   = note: `-D clippy::new-without-default` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clippy::new_without_default)]`
help: try adding this
   |
11 + impl Default for SecurityPolicy {
12 +     fn default() -> Self {
13 +         Self::new()
14 +     }
15 + }
   |

error: you should consider adding a `Default` implementation for `SecurityManager`
   --> src\acp\security.rs:99:5
    |
 99 | /     pub fn new() -> Self {
100 | |         Self {
101 | |             policy: Arc::new(Mutex::new(SecurityPolicy::new())),        
102 | |         }
103 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#new_without_default
help: try adding this
    |
 98 + impl Default for SecurityManager {
 99 +     fn default() -> Self {
100 +         Self::new()
101 +     }
102 + }
    |

error: this `if` statement can be collapsed
   --> src\cli\commands\chat.rs:104:13
    |
104 | /             if let Some(tool_calls) = &response.tool_calls {
105 | |                 if !tool_calls.is_empty() {
106 | |                     print_info("Executing requested operations...");    
107 | |                     let mut security = SecurityPolicy::new();
...   |
116 | |             }
    | |_____________^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
    = note: `-D clippy::collapsible-if` implied by `-D warnings`
    = help: to override `-D warnings` add `#[allow(clippy::collapsible_if)]`    
help: collapse nested if block
    |
104 ~             if let Some(tool_calls) = &response.tool_calls
105 ~                 && !tool_calls.is_empty() {
106 |                     print_info("Executing requested operations...");      
...
114 |                     return Ok(());
115 ~                 }
    |

error: this `if` statement can be collapsed
   --> src\cli\commands\chat.rs:384:17
    |
384 | / ...   if let Some(tool_calls) = &response_msg.tool_calls {
385 | | ...       if !tool_calls.is_empty() {
386 | | ...           println!("{}", "Grok is executing operations...".blue(... 
...   |
403 | | ...   }
    | |_______^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
384 ~                 if let Some(tool_calls) = &response_msg.tool_calls        
385 ~                     && !tool_calls.is_empty() {
386 |                         println!("{}", "Grok is executing operations...".blue().dimmed());
...
401 |                         continue;
402 ~                     }
    |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1288:9
     |
1288 | /         if let Ok(temp) = std::env::var("GROK_TEMPERATURE") {
1289 | |             if let Ok(temp_val) = temp.parse::<f32>() {
1290 | |                 self.default_temperature = temp_val;
1291 | |             }
1292 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1288 ~         if let Ok(temp) = std::env::var("GROK_TEMPERATURE")
1289 ~             && let Ok(temp_val) = temp.parse::<f32>() {
1290 |                 self.default_temperature = temp_val;
1291 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1294:9
     |
1294 | /         if let Ok(tokens) = std::env::var("GROK_MAX_TOKENS") {
1295 | |             if let Ok(tokens_val) = tokens.parse::<u32>() {
1296 | |                 self.default_max_tokens = tokens_val;
1297 | |             }
1298 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1294 ~         if let Ok(tokens) = std::env::var("GROK_MAX_TOKENS")
1295 ~             && let Ok(tokens_val) = tokens.parse::<u32>() {
1296 |                 self.default_max_tokens = tokens_val;
1297 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1301:9
     |
1301 | /         if let Ok(timeout) = std::env::var("GROK_TIMEOUT") {
1302 | |             if let Ok(timeout_val) = timeout.parse::<u64>() {
1303 | |                 self.timeout_secs = timeout_val;
1304 | |             }
1305 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1301 ~         if let Ok(timeout) = std::env::var("GROK_TIMEOUT")
1302 ~             && let Ok(timeout_val) = timeout.parse::<u64>() {
1303 |                 self.timeout_secs = timeout_val;
1304 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1307:9
     |
1307 | /         if let Ok(retries) = std::env::var("GROK_MAX_RETRIES") {       
1308 | |             if let Ok(retries_val) = retries.parse::<u32>() {
1309 | |                 self.max_retries = retries_val;
1310 | |             }
1311 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1307 ~         if let Ok(retries) = std::env::var("GROK_MAX_RETRIES")
1308 ~             && let Ok(retries_val) = retries.parse::<u32>() {
1309 |                 self.max_retries = retries_val;
1310 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1317:9
     |
1317 | /         if let Ok(delay) = std::env::var("GROK_BASE_RETRY_DELAY") {    
1318 | |             if let Ok(delay_val) = delay.parse::<u64>() {
1319 | |                 self.network.base_retry_delay = delay_val;
1320 | |             }
1321 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1317 ~         if let Ok(delay) = std::env::var("GROK_BASE_RETRY_DELAY")        
1318 ~             && let Ok(delay_val) = delay.parse::<u64>() {
1319 |                 self.network.base_retry_delay = delay_val;
1320 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1323:9
     |
1323 | /         if let Ok(delay) = std::env::var("GROK_MAX_RETRY_DELAY") {     
1324 | |             if let Ok(delay_val) = delay.parse::<u64>() {
1325 | |                 self.network.max_retry_delay = delay_val;
1326 | |             }
1327 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1323 ~         if let Ok(delay) = std::env::var("GROK_MAX_RETRY_DELAY")
1324 ~             && let Ok(delay_val) = delay.parse::<u64>() {
1325 |                 self.network.max_retry_delay = delay_val;
1326 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1333:9
     |
1333 | /         if let Ok(timeout) = std::env::var("GROK_CONNECT_TIMEOUT") {   
1334 | |             if let Ok(timeout_val) = timeout.parse::<u64>() {
1335 | |                 self.network.connect_timeout = timeout_val;
1336 | |             }
1337 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1333 ~         if let Ok(timeout) = std::env::var("GROK_CONNECT_TIMEOUT")       
1334 ~             && let Ok(timeout_val) = timeout.parse::<u64>() {
1335 |                 self.network.connect_timeout = timeout_val;
1336 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1339:9
     |
1339 | /         if let Ok(timeout) = std::env::var("GROK_READ_TIMEOUT") {      
1340 | |             if let Ok(timeout_val) = timeout.parse::<u64>() {
1341 | |                 self.network.read_timeout = timeout_val;
1342 | |             }
1343 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1339 ~         if let Ok(timeout) = std::env::var("GROK_READ_TIMEOUT")
1340 ~             && let Ok(timeout_val) = timeout.parse::<u64>() {
1341 |                 self.network.read_timeout = timeout_val;
1342 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1362:9
     |
1362 | /         if let Ok(width) = std::env::var("GROK_TERMINAL_WIDTH") {      
1363 | |             if let Ok(width_val) = width.parse::<usize>() {
1364 | |                 self.ui.terminal_width = width_val;
1365 | |             }
1366 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1362 ~         if let Ok(width) = std::env::var("GROK_TERMINAL_WIDTH")
1363 ~             && let Ok(width_val) = width.parse::<usize>() {
1364 |                 self.ui.terminal_width = width_val;
1365 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1386:9
     |
1386 | /         if let Ok(size) = std::env::var("GROK_MAX_FILE_SIZE_MB") {     
1387 | |             if let Ok(size_val) = size.parse::<u64>() {
1388 | |                 self.logging.max_file_size_mb = size_val;
1389 | |             }
1390 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1386 ~         if let Ok(size) = std::env::var("GROK_MAX_FILE_SIZE_MB")
1387 ~             && let Ok(size_val) = size.parse::<u64>() {
1388 |                 self.logging.max_file_size_mb = size_val;
1389 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1392:9
     |
1392 | /         if let Ok(count) = std::env::var("GROK_ROTATION_COUNT") {      
1393 | |             if let Ok(count_val) = count.parse::<u32>() {
1394 | |                 self.logging.rotation_count = count_val;
1395 | |             }
1396 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1392 ~         if let Ok(count) = std::env::var("GROK_ROTATION_COUNT")
1393 ~             && let Ok(count_val) = count.parse::<u32>() {
1394 |                 self.logging.rotation_count = count_val;
1395 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1407:9
     |
1407 | /         if let Ok(port) = std::env::var("GROK_ACP_PORT") {
1408 | |             if let Ok(port_val) = port.parse::<u16>() {
1409 | |                 self.acp.default_port = Some(port_val);
1410 | |             }
1411 | |         }
     | |_________^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1407 ~         if let Ok(port) = std::env::var("GROK_ACP_PORT")
1408 ~             && let Ok(port_val) = port.parse::<u16>() {
1409 |                 self.acp.default_port = Some(port_val);
1410 ~             }
     |

error: this `if` statement can be collapsed
    --> src\config\mod.rs:1485:9
     |
1485 | / ...   if let Some(port) = self.acp.default_port {
1486 | | ...       if port < 1024 {
1487 | | ...           warn!(
1488 | | ...               "ACP port {} is below 1024, may require elevated ... 
...    |
1492 | | ...   }
     | |_______^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1485 ~         if let Some(port) = self.acp.default_port
1486 ~             && port < 1024 {
1487 |                 warn!(
 ...
1490 |                 );
1491 ~             }
     |

error: this `if` statement can be collapsed
   --> src\display\interactive.rs:166:5
    |
166 | /     if let Some(skills_dir) = crate::skills::get_default_skills_dir() { 
167 | |         if let Ok(skills_context) = crate::skills::get_skills_contex... 
168 | |             if !skills_context.is_empty() {
169 | |                 let ctx = project_context.get_or_insert_with(String:... 
...   |
173 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
166 ~     if let Some(skills_dir) = crate::skills::get_default_skills_dir()     
167 ~         && let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
168 |             if !skills_context.is_empty() {
...
171 |             }
172 ~         }
    |

error: this `if` statement can be collapsed
   --> src\display\interactive.rs:167:9
    |
167 | / ...   if let Ok(skills_context) = crate::skills::get_skills_context(... 
168 | | ...       if !skills_context.is_empty() {
169 | | ...           let ctx = project_context.get_or_insert_with(String::n... 
170 | | ...           ctx.push_str(&skills_context);
171 | | ...       }
172 | | ...   }
    | |_______^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
167 ~         if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir)
168 ~             && !skills_context.is_empty() {
169 |                 let ctx = project_context.get_or_insert_with(String::new);
170 |                 ctx.push_str(&skills_context);
171 ~             }
    |

error: this `if` statement can be collapsed
   --> src\display\interactive.rs:299:5
    |
299 | /     if let Some(skills_dir) = crate::skills::get_default_skills_dir() { 
300 | |         if let Ok(skills) = crate::skills::list_skills(&skills_dir) {   
301 | |             if !skills.is_empty() {
302 | |                 println!(
...   |
311 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
299 ~     if let Some(skills_dir) = crate::skills::get_default_skills_dir()     
300 ~         && let Ok(skills) = crate::skills::list_skills(&skills_dir) {     
301 |             if !skills.is_empty() {
...
309 |             }
310 ~         }
    |

error: this `if` statement can be collapsed
   --> src\display\interactive.rs:300:9
    |
300 | /         if let Ok(skills) = crate::skills::list_skills(&skills_dir) {   
301 | |             if !skills.is_empty() {
302 | |                 println!(
303 | |                     "  Skills: {}",
...   |
310 | |         }
    | |_________^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
300 ~         if let Ok(skills) = crate::skills::list_skills(&skills_dir)       
301 ~             && !skills.is_empty() {
302 |                 println!(
...
308 |                 }
309 ~             }
    |

error: this `if` statement can be collapsed
    --> src\display\interactive.rs:1106:13
     |
1106 | / ...   if let Some(tool_calls) = &response_msg.tool_calls {
1107 | | ...       if !tool_calls.is_empty() {
1108 | | ...           println!("{}", "Grok is executing operations...".blue... 
1109 | | ...           println!();
...    |
1131 | | ...   }
     | |_______^
     |
     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
     |
1106 ~             if let Some(tool_calls) = &response_msg.tool_calls
1107 ~                 && !tool_calls.is_empty() {
1108 |                     println!("{}", "Grok is executing operations...".blue().bold());
 ...
1129 |                     return Ok(());
1130 ~                 }
     |

error: this `if` statement can be collapsed
   --> src\mcp\client.rs:151:9
    |
151 | / ...   if let Some(result) = response.get("result") {
152 | | ...       if let Some(tools_val) = result.get("tools") {
153 | | ...           let tools: Vec<Tool> = serde_json::from_value(tools_va... 
154 | | ...           return Ok(tools);
155 | | ...       }
156 | | ...   }
    | |_______^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
151 ~         if let Some(result) = response.get("result")
152 ~             && let Some(tools_val) = result.get("tools") {
153 |                 let tools: Vec<Tool> = serde_json::from_value(tools_val.clone())?;
154 |                 return Ok(tools);
155 ~             }
    |

error: this `if` statement can be collapsed
  --> src\skills\manager.rs:80:9
   |
80 | /         if entry.file_type().is_dir() {
81 | |             if entry.path().join("SKILL.md").exists() {
82 | |                 match load_skill(entry.path()) {
83 | |                     Ok(skill) => skills.push(skill),
...  |
90 | |         }
   | |_________^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
   |
80 ~         if entry.file_type().is_dir()
81 ~             && entry.path().join("SKILL.md").exists() {
82 |                 match load_skill(entry.path()) {
...
88 |                 }
89 ~             }
   |

error: this `if` statement can be collapsed
   --> src\utils\context.rs:97:5
    |
 97 | /     if let Some(global_dir) = get_global_context_dir() {
 98 | |         if global_dir.exists() && global_dir.is_dir() {
 99 | |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
100 | |                 let file_path = global_dir.join(file_name);
...   |
119 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
 97 ~     if let Some(global_dir) = get_global_context_dir()
 98 ~         && global_dir.exists() && global_dir.is_dir() {
 99 |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
...
117 |             }
118 ~         }
    |

error: this `if` statement can be collapsed
   --> src\utils\context.rs:177:5
    |
177 | /     if let Some(global_dir) = get_global_context_dir() {
178 | |         if global_dir.exists() && global_dir.is_dir() {
179 | |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
180 | |                 let file_path = global_dir.join(file_name);
...   |
204 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
177 ~     if let Some(global_dir) = get_global_context_dir()
178 ~         && global_dir.exists() && global_dir.is_dir() {
179 |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
...
202 |             }
203 ~         }
    |

error: this `if` statement can be collapsed
   --> src\utils\context.rs:231:5
    |
231 | /     if let Some(global_dir) = get_global_context_dir() {
232 | |         if global_dir.exists() && global_dir.is_dir() {
233 | |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
234 | |                 let file_path = global_dir.join(file_name);
...   |
240 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
231 ~     if let Some(global_dir) = get_global_context_dir()
232 ~         && global_dir.exists() && global_dir.is_dir() {
233 |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
...
238 |             }
239 ~         }
    |

error: this `if` statement can be collapsed
   --> src\utils\context.rs:262:5
    |
262 | /     if let Some(global_dir) = get_global_context_dir() {
263 | |         if global_dir.exists() && global_dir.is_dir() {
264 | |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
265 | |                 let file_path = global_dir.join(file_name);
...   |
271 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
262 ~     if let Some(global_dir) = get_global_context_dir()
263 ~         && global_dir.exists() && global_dir.is_dir() {
264 |             for file_name in GLOBAL_CONTEXT_FILE_NAMES {
...
269 |             }
270 ~         }
    |

error: this `if` statement can be collapsed
   --> src\utils\network.rs:144:5
    |
144 | /     if let Ok(addrs) = tokio::net::lookup_host("starlink.com:80").aw... 
145 | |         if addrs.count() > 0 {
146 | |             info!("Starlink domain resolution successful - possible ... 
147 | |             return true;
148 | |         }
149 | |     }
    | |_____^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
    |
144 ~     if let Ok(addrs) = tokio::net::lookup_host("starlink.com:80").await   
145 ~         && addrs.count() > 0 {
146 |             info!("Starlink domain resolution successful - possible Starlink connection");
147 |             return true;
148 ~         }
    |

error: this `if` statement can be collapsed
  --> src\utils\shell_permissions.rs:74:9
   |
74 | / ...   if let Some(path) = &permissions.policy_path {
75 | | ...       if let Ok(policy) = Self::load_policy(path) {
76 | | ...           permissions.persistent_allowlist = policy.allowed_comma... 
77 | | ...       }
78 | | ...   }
   | |_______^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
help: collapse nested if block
   |
74 ~         if let Some(path) = &permissions.policy_path
75 ~             && let Ok(policy) = Self::load_policy(path) {
76 |                 permissions.persistent_allowlist = policy.allowed_commands;
77 ~             }
   |

error: could not compile `grok-cli` (lib) due to 30 previous errors

## Assistant

<thinking>
**Examining Clippy Errors**

I'm focused on the clippy errors now. They seem typical for a codebase needing attention, mostly around collapsible `if` statements. I've pinpointed an error in `src/skills/manager.rs`, specifically. The goal is to clean up these easily fixed issues.


**Prioritizing Clippy Fixes**

I'm deep into the `clippy` fixes. A few `collapsible_if` issues jumped out, as I predicted. I'm taking care of the one in `src/skills/manager.rs`, as mentioned earlier. Also, I'm tackling a similar problem in `src/display/interactive.rs` to keep things tidy. I will continue to focus on the key errors, and defer the rest.



</thinking>

**Tool Call: src\skills\manager.rs:     // Look for directories co... =>     // Look for directories co...**
Status: Failed

Failed to edit, 0 occurrences found for old_string in H:\GitHub\grok-cli\src\skills\manager.rs. Ensure you're not escaping content incorrectly and check whitespace, indentation, and context. Use read_file tool to verify.

## Assistant

<thinking>
**Checking for Errors**

I've reviewed the instructions again, and I'm focusing on the formatting of the output, specifically indentation and newlines. I'm double-checking the spacing to ensure everything is correct before generating a summary.



</thinking>

**Tool Call: src\skills\manager.rs**
Status: Completed

Read lines 124-123 of 123 from src/skills/manager.rs

## Assistant

<thinking>
**Reviewing File Contents**

I'm currently focused on the file's conclusion. My attention is drawn to specific lines, around the 80th mark. I'm taking a close look at the details in that particular segment.



</thinking>

**Tool Call: src\skills\manager.rs**
Status: Completed

Read lines 61-90 of 123 from src/skills/manager.rs

## Assistant

<thinking>
**Assessing Content Comprehension**

I've successfully identified the core function: processing file system entries to extract and load skill data. My immediate focus is on how the code handles directory entries. I am taking note of the conditional that checks for the existence of "SKILL.md" within a directory.


**Evaluating Code Execution Path**

My attention is now on the `entry.path().join("SKILL.md").exists()` conditional. I am re-evaluating the path resolution strategy to ensure robustness against different file system layouts. I anticipate issues potentially arising from incorrect handling of relative paths during the file lookup, so I'm investigating how those paths are constructed and resolved.



</thinking>

**Tool Call: src\skills\manager.rs:         if entry.file_type().i... =>         if entry.file_type().i...**
Status: Completed

Diff: manager.rs
```
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::skills::config::{Skill, SkillConfig};

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(anyhow!("No SKILL.md found in {}", skill_dir.display()));
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content manually to be robust
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of the frontmatter
    // We look for the second "---" after the first one
    let end_fm_idx = content[3..]
        .find("\n---")
        .map(|i| i + 3) // Add back the offset
        .ok_or_else(|| anyhow!("Could not find end of frontmatter in SKILL.md"))?;

    let frontmatter = &content[3..end_fm_idx].trim();

    // The instructions start after the second "---"
    // end_fm_idx points to the start of the second "---"
    // The second "---" has length 3, plus usually a newline
    let instructions_start = end_fm_idx + 4; // +4 for "\n---" length if we found via "\n---"

    // Check if we have enough content
    let instructions = if instructions_start < content.len() {
        content[instructions_start..].trim().to_string()
    } else {
        String::new()
    };

    let config: SkillConfig = serde_yaml::from_str(frontmatter).with_context(|| {
        format!(
            "Failed to parse YAML frontmatter in {}",
            skill_file.display()
        )
    })?;

    Ok(Skill {
        config,
        instructions,
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    // We only look at immediate subdirectories of the skills folder
    for entry in WalkDir::new(base_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() && entry.path().join("SKILL.md").exists() {
            match load_skill(entry.path()) {
                Ok(skill) => skills.push(skill),
                Err(_e) => {
                    // Silently ignore or log warning if possible
                    // eprintln!("Warning: Failed to load skill at {}: {}", entry.path().display(), e);
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}

/// Get formatted context string for all available skills
pub fn get_skills_context(base_dir: &Path) -> Result<String> {
    let skills = list_skills(base_dir)?;
    if skills.is_empty() {
        return Ok(String::new());
    }

    let mut context =
        String::from("\n\n## Available Skills\n\nThe following skills are available for use:\n\n");
    for skill in skills {
        context.push_str(&format!("### Skill: {}\n", skill.config.name));
        context.push_str(&format!("Description: {}\n", skill.config.description));
        context.push_str("\nInstructions:\n");
        context.push_str(&skill.instructions);
        context.push_str("\n\n---\n\n");
    }
    Ok(context)
}

```


## User

go onn
