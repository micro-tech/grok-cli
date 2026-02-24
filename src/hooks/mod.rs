pub mod loader;

use anyhow::Result;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub tool_name: String,
    pub args: Value,
}

pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    // Return Ok(true) to continue, Ok(false) to abort (silently or with log), Err to fail hard
    fn before_tool(&self, _context: &ToolContext) -> Result<bool> {
        Ok(true)
    }

    fn after_tool(&self, _context: &ToolContext, _result: &str) -> Result<()> {
        Ok(())
    }
}

pub struct HookManager {
    hooks: Vec<Box<dyn Hook>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    pub fn list_hooks(&self) -> Vec<&str> {
        self.hooks.iter().map(|h| h.name()).collect()
    }

    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    pub fn execute_before_tool(&self, tool_name: &str, args: &Value) -> Result<bool> {
        let context = ToolContext {
            tool_name: tool_name.to_string(),
            args: args.clone(),
        };

        for hook in &self.hooks {
            if !hook.before_tool(&context)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn execute_after_tool(&self, tool_name: &str, args: &Value, result: &str) -> Result<()> {
        let context = ToolContext {
            tool_name: tool_name.to_string(),
            args: args.clone(),
        };

        for hook in &self.hooks {
            hook.after_tool(&context, result)?;
        }
        Ok(())
    }
}

pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn register_hooks(&self, hook_manager: &mut HookManager) -> Result<()>;
}

pub struct ExtensionManager {
    extensions: Vec<Box<dyn Extension>>,
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register(&mut self, extension: Box<dyn Extension>) {
        self.extensions.push(extension);
    }

    pub fn register_all_hooks(&self, hook_manager: &mut HookManager) -> Result<()> {
        for ext in &self.extensions {
            ext.register_hooks(hook_manager)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestHook {
        name: String,
        before_called: Arc<Mutex<bool>>,
        after_called: Arc<Mutex<bool>>,
    }

    impl Hook for TestHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn before_tool(&self, _context: &ToolContext) -> Result<bool> {
            let mut called = self.before_called.lock().unwrap();
            *called = true;
            Ok(true)
        }

        fn after_tool(&self, _context: &ToolContext, _result: &str) -> Result<()> {
            let mut called = self.after_called.lock().unwrap();
            *called = true;
            Ok(())
        }
    }

    #[test]
    fn test_hooks_execution() {
        let before = Arc::new(Mutex::new(false));
        let after = Arc::new(Mutex::new(false));

        let hook = TestHook {
            name: "test".to_string(),
            before_called: before.clone(),
            after_called: after.clone(),
        };

        let mut manager = HookManager::new();
        manager.register(Box::new(hook));

        let args = serde_json::json!({});

        assert!(manager.execute_before_tool("test_tool", &args).unwrap());
        assert!(*before.lock().unwrap());

        manager
            .execute_after_tool("test_tool", &args, "result")
            .unwrap();
        assert!(*after.lock().unwrap());
    }
}
