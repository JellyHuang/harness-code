//! Configuration variable substitution.
//!
//! Supports OpenCode-style variable substitution:
//! - `{env:VAR_NAME}` - Substitute with environment variable
//! - `{file:path}` - Substitute with file contents

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// Substitution error.
#[derive(Debug, thiserror::Error)]
pub enum SubstitutionError {
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Failed to read file: {0}")]
    FileReadError(String, #[source] std::io::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Substitute variables in a string.
///
/// Supports:
/// - `{env:VAR_NAME}` - Replace with environment variable value
/// - `{file:path}` - Replace with file contents
pub fn substitute_string(
    value: &str,
    config_dir: Option<&Path>,
) -> Result<String, SubstitutionError> {
    // Pattern for {env:VAR_NAME} or {file:path}
    let env_pattern = Regex::new(r"\{env:([^}]+)\}").unwrap();
    let file_pattern = Regex::new(r"\{file:([^}]+)\}").unwrap();

    let mut result = value.to_string();

    // Substitute environment variables
    result = env_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            env::var(var_name).unwrap_or_default()
        })
        .into_owned();

    // Substitute file contents
    result = file_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let path_str = &caps[1];
            match resolve_file_path(path_str, config_dir) {
                Ok(path) => match std::fs::read_to_string(&path) {
                    Ok(content) => content.trim().to_string(),
                    Err(e) => {
                        eprintln!("Warning: Failed to read file {}: {}", path.display(), e);
                        String::new()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    String::new()
                }
            }
        })
        .into_owned();

    Ok(result)
}

/// Resolve a file path, handling ~ and relative paths.
fn resolve_file_path(
    path_str: &str,
    config_dir: Option<&Path>,
) -> Result<PathBuf, SubstitutionError> {
    let path = if path_str.starts_with('~') {
        // Expand ~ to home directory
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| {
                SubstitutionError::InvalidPath("Cannot determine home directory".into())
            })?;
        PathBuf::from(path_str.replacen('~', &home, 1))
    } else if Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else if let Some(dir) = config_dir {
        // Relative to config file directory
        dir.join(path_str)
    } else {
        // Relative to current directory
        PathBuf::from(path_str)
    };

    Ok(path)
}

/// Substitute all string fields in a ProviderOptions.
pub fn substitute_provider_options(
    options: &mut super::ProviderOptions,
    config_dir: Option<&Path>,
) -> Result<(), SubstitutionError> {
    if let Some(ref api_key) = options.api_key {
        options.api_key = Some(substitute_string(api_key, config_dir)?);
    }
    if let Some(ref base_url) = options.base_url {
        options.base_url = Some(substitute_string(base_url, config_dir)?);
    }
    // Substitute headers
    let mut new_headers = HashMap::new();
    for (key, value) in &options.headers {
        let new_value = substitute_string(value, config_dir)?;
        new_headers.insert(key.clone(), new_value);
    }
    options.headers = new_headers;
    Ok(())
}

/// Substitute all string fields in a ModelConfig.
pub fn substitute_model_config(
    model: &mut super::ModelConfig,
    config_dir: Option<&Path>,
) -> Result<(), SubstitutionError> {
    if let Some(ref name) = model.name {
        model.name = Some(substitute_string(name, config_dir)?);
    }
    if let Some(ref id) = model.id {
        model.id = Some(substitute_string(id, config_dir)?);
    }
    Ok(())
}

/// Substitute all string fields in a ProviderConfig.
pub fn substitute_provider_config(
    provider: &mut super::ProviderConfig,
    config_dir: Option<&Path>,
) -> Result<(), SubstitutionError> {
    substitute_provider_options(&mut provider.options, config_dir)?;

    if let Some(ref mut models) = provider.models {
        for model in models.values_mut() {
            substitute_model_config(model, config_dir)?;
        }
    }
    Ok(())
}

/// Substitute all string fields in an AgentConfig.
pub fn substitute_agent_config(
    agent: &mut super::AgentConfig,
    config_dir: Option<&Path>,
) -> Result<(), SubstitutionError> {
    if let Some(ref model) = agent.model {
        agent.model = Some(substitute_string(model, config_dir)?);
    }
    if let Some(ref prompt) = agent.prompt {
        agent.prompt = Some(substitute_string(prompt, config_dir)?);
    }
    if let Some(ref description) = agent.description {
        agent.description = Some(substitute_string(description, config_dir)?);
    }
    Ok(())
}

/// Substitute all variables in a Config.
pub fn substitute_config(
    config: &mut super::Config,
    config_dir: Option<&Path>,
) -> Result<(), SubstitutionError> {
    // Substitute model strings
    if let Some(ref model) = config.model {
        config.model = Some(substitute_string(model, config_dir)?);
    }
    if let Some(ref small_model) = config.small_model {
        config.small_model = Some(substitute_string(small_model, config_dir)?);
    }
    if let Some(ref data_dir) = config.data_dir {
        config.data_dir = Some(substitute_string(data_dir, config_dir)?);
    }

    // Substitute provider configs
    for provider in config.provider.values_mut() {
        substitute_provider_config(provider, config_dir)?;
    }

    // Substitute agent configs
    for agent in config.agents.values_mut() {
        substitute_agent_config(agent, config_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_substitution() {
        env::set_var("TEST_VAR", "test_value");
        let result = substitute_string("prefix-{env:TEST_VAR}-suffix", None).unwrap();
        assert_eq!(result, "prefix-test_value-suffix");
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_missing_env_var() {
        let result = substitute_string("{env:NONEXISTENT_VAR}", None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_multiple_substitutions() {
        env::set_var("VAR1", "a");
        env::set_var("VAR2", "b");
        let result = substitute_string("{env:VAR1}-{env:VAR2}", None).unwrap();
        assert_eq!(result, "a-b");
        env::remove_var("VAR1");
        env::remove_var("VAR2");
    }

    #[test]
    fn test_no_substitution() {
        let result = substitute_string("plain text", None).unwrap();
        assert_eq!(result, "plain text");
    }
}
