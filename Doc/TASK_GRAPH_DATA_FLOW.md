# Task Graph Engine Data Flow

## Overview

The Task Graph Engine enables Grok-CLI to execute complex multi-step workflows with proper dependency resolution and error handling. This document outlines the data flow and architecture.

## Architecture

```
User Request
    ↓
LLM Analysis
    ↓
Task Graph Generation (JSON)
    ↓
execute_task_graph Tool
    ↓
TaskGraph::execute()
    ↓
Topological Sort
    ↓
Sequential Tool Execution
    ↓
Result Aggregation
```

## Components

### 1. Task Graph Schema
- **TaskNode**: Individual execution step with ID, action, and dependencies
- **ToolCall**: Tool name and JSON arguments
- **TaskGraph**: Collection of nodes with execution logic

### 2. Execution Flow
1. Parse JSON graph into TaskGraph struct
2. Perform topological sort to detect cycles and order execution
3. Execute tools in dependency order
4. Aggregate results and handle errors

### 3. Integration Points
- **Tool Registry**: execute_task_graph registered as tool
- **CPU Router**: Tool calls flow through existing infrastructure
- **Security**: All tool executions respect existing security policies

## Data Structures

```rust
#[derive(Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub action: ToolCall,
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct TaskGraph {
    pub nodes: HashMap<String, TaskNode>,
}
```

## Error Handling

- **Cycle Detection**: Topological sort fails on circular dependencies
- **Tool Failures**: Individual tool errors propagated up
- **Invalid JSON**: Parse errors with detailed messages
- **Security**: All operations subject to existing access controls

## Example Workflow

```json
{
  "nodes": {
    "read": {
      "id": "read",
      "action": {
        "tool_name": "read_file",
        "arguments": {"path": "input.txt"}
      },
      "dependencies": []
    },
    "process": {
      "id": "process",
      "action": {
        "tool_name": "replace",
        "arguments": {
          "path": "input.txt",
          "old_string": "old",
          "new_string": "new"
        }
      },
      "dependencies": ["read"]
    },
    "save": {
      "id": "save",
      "action": {
        "tool_name": "write_file",
        "arguments": {
          "path": "output.txt",
          "content": "processed content"
        }
      },
      "dependencies": ["process"]
    }
  }
}
```

Execution order: read → process → save