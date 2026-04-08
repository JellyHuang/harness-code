//! Built-in agent definitions.

use super::schema::AgentDefinition;

/// Get all built-in agent definitions.
pub fn get_builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            name: "researcher".to_string(),
            description: "Search and analyze codebase to find information".to_string(),
            system_prompt: Some(
                "You are a research agent. Your job is to search and analyze the codebase \
                 to find specific information. Be thorough and systematic. \
                 Use grep, glob, and read tools to explore. Summarize your findings clearly."
                    .to_string(),
            ),
            tools: Some(vec![
                "bash".to_string(),
                "read".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "webfetch".to_string(),
            ]),
            model: None,
            disallowed_tools: Some(vec!["write".to_string(), "edit".to_string()]),
        },
        AgentDefinition {
            name: "coder".to_string(),
            description: "Write and edit code files".to_string(),
            system_prompt: Some(
                "You are a coding agent. Your job is to write and modify code files. \
                 Follow existing code patterns and conventions. Write clean, well-documented code. \
                 Test your changes when possible."
                    .to_string(),
            ),
            tools: Some(vec![
                "bash".to_string(),
                "read".to_string(),
                "write".to_string(),
                "edit".to_string(),
                "glob".to_string(),
                "grep".to_string(),
            ]),
            model: None,
            disallowed_tools: None,
        },
        AgentDefinition {
            name: "reviewer".to_string(),
            description: "Review code for issues and improvements".to_string(),
            system_prompt: Some(
                "You are a code review agent. Your job is to review code for potential issues, \
                 bugs, security vulnerabilities, and improvement opportunities. \
                 Be thorough but constructive. Provide specific, actionable feedback."
                    .to_string(),
            ),
            tools: Some(vec![
                "read".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "bash".to_string(),
            ]),
            model: None,
            disallowed_tools: Some(vec!["write".to_string(), "edit".to_string()]),
        },
        AgentDefinition {
            name: "tester".to_string(),
            description: "Write and run tests for code".to_string(),
            system_prompt: Some(
                "You are a testing agent. Your job is to write and execute tests. \
                 Ensure code works correctly and handle edge cases. \
                 Report test results clearly with pass/fail status."
                    .to_string(),
            ),
            tools: Some(vec![
                "bash".to_string(),
                "read".to_string(),
                "write".to_string(),
                "edit".to_string(),
                "glob".to_string(),
                "grep".to_string(),
            ]),
            model: None,
            disallowed_tools: None,
        },
        AgentDefinition {
            name: "planner".to_string(),
            description: "Plan and break down complex tasks".to_string(),
            system_prompt: Some(
                "You are a planning agent. Your job is to analyze complex tasks and break them \
                 into smaller, manageable steps. Create clear, actionable plans with dependencies. \
                 Think through potential issues and edge cases."
                    .to_string(),
            ),
            tools: Some(vec![
                "read".to_string(),
                "glob".to_string(),
                "grep".to_string(),
            ]),
            model: None,
            disallowed_tools: Some(vec!["write".to_string(), "edit".to_string(), "bash".to_string()]),
        },
    ]
}

/// Get a built-in agent by name.
pub fn get_builtin_agent(name: &str) -> Option<AgentDefinition> {
    get_builtin_agents().into_iter().find(|a| a.name == name)
}

/// Check if an agent name is a built-in agent.
pub fn is_builtin_agent(name: &str) -> bool {
    get_builtin_agents().iter().any(|a| a.name == name)
}