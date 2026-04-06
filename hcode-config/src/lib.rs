//! HCode Config - Configuration loading compatible with OpenCode format.

pub mod agent;
pub mod config;
pub mod provider;
pub mod substitution;

pub use config::*;
pub use substitution::{substitute_config, substitute_string, SubstitutionError};

use std::env;
use std::path::{Path, PathBuf};

/// Configuration loading error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[source] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[source] serde_json::Error),

    #[error("Substitution error: {0}")]
    SubstitutionError(#[from] SubstitutionError),

    #[error("Config file not found")]
    NotFound,

    #[error("Invalid baseURL: {0}")]
    InvalidBaseUrl(String),
}

/// Get the platform-specific config directory.
///
/// On Windows: %LOCALAPPDATA%\hcode
/// On macOS/Linux: ~/.config/hcode
fn get_config_dir() -> Option<PathBuf> {
    // Check for override via environment variable
    if let Ok(dir) = env::var("HCODE_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use %LOCALAPPDATA%\hcode
        env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("hcode"))
            .ok()
            .or_else(|| {
                // Fallback to APPDATA if LOCALAPPDATA is not set
                env::var("APPDATA")
                    .map(|p| PathBuf::from(p).join("hcode"))
                    .ok()
            })
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, use ~/.config/hcode
        dirs::home_dir().map(|h| h.join(".config").join("hcode"))
    }
}

/// Load configuration from file.
///
/// Search order (matching OpenCode):
/// 1. HCODE_CONFIG environment variable (explicit path)
/// 2. ./hcode.jsonc or ./hcode.json (current directory)
/// 3. ./.hcode/config.json (current directory)
/// 4. ./.opencode/config.json (current directory, for compatibility)
/// 5. Global config directory:
///    - Windows: %LOCALAPPDATA%\hcode\config.json
///    - macOS/Linux: ~/.config/hcode/config.json
/// 6. Default empty config
pub fn load_config() -> Result<Config, ConfigError> {
    // Check HCODE_CONFIG env var first (explicit path)
    if let Ok(path) = env::var("HCODE_CONFIG") {
        let path = PathBuf::from(path);
        if path.exists() {
            return load_config_from_path(&path);
        }
    }

    // Check current directory
    let cwd = env::current_dir().map_err(ConfigError::ReadError)?;

    // ./hcode.jsonc (JSON with comments)
    let project_config_jsonc = cwd.join("hcode.jsonc");
    if project_config_jsonc.exists() {
        return load_config_from_path(&project_config_jsonc);
    }

    // ./hcode.json
    let project_config = cwd.join("hcode.json");
    if project_config.exists() {
        return load_config_from_path(&project_config);
    }

    // ./.hcode/config.json
    let hcode_dir_config = cwd.join(".hcode").join("config.json");
    if hcode_dir_config.exists() {
        return load_config_from_path(&hcode_dir_config);
    }

    // ./.opencode/config.json (for OpenCode compatibility)
    let opencode_dir_config = cwd.join(".opencode").join("config.json");
    if opencode_dir_config.exists() {
        return load_config_from_path(&opencode_dir_config);
    }

    // Check global config directory
    if let Some(config_dir) = get_config_dir() {
        // Try config.json first
        let global_config = config_dir.join("config.json");
        if global_config.exists() {
            return load_config_from_path(&global_config);
        }

        // Try hcode.json
        let hcode_json = config_dir.join("hcode.json");
        if hcode_json.exists() {
            return load_config_from_path(&hcode_json);
        }

        // Try hcode.jsonc
        let hcode_jsonc = config_dir.join("hcode.jsonc");
        if hcode_jsonc.exists() {
            return load_config_from_path(&hcode_jsonc);
        }
    }

    // Return default empty config
    Ok(Config::default())
}

/// Strip JSONC comments and trailing commas from content.
///
/// This is a simple implementation that handles:
/// - Single-line comments (// ...)
/// - Multi-line comments (/* ... */)
/// - Trailing commas before ] and }
fn strip_jsonc(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    let mut in_string = false;

    while i < chars.len() {
        let c = chars[i];

        // Handle string literals (don't strip comments inside strings)
        if c == '"' && (i == 0 || chars[i - 1] != '\\') {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }

        if in_string {
            result.push(c);
            i += 1;
            continue;
        }

        // Check for single-line comment
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            // Skip until end of line
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Check for multi-line comment
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // Skip */
            continue;
        }

        result.push(c);
        i += 1;
    }

    // Remove trailing commas before ] and }
    let mut final_result = String::with_capacity(result.len());
    let result_chars: Vec<char> = result.chars().collect();
    let mut j = 0;

    while j < result_chars.len() {
        if result_chars[j] == ',' {
            // Look ahead for ] or }
            let mut k = j + 1;
            while k < result_chars.len() && result_chars[k].is_whitespace() {
                k += 1;
            }
            if k < result_chars.len() && (result_chars[k] == ']' || result_chars[k] == '}') {
                // Skip the comma
                j += 1;
                continue;
            }
        }
        final_result.push(result_chars[j]);
        j += 1;
    }

    final_result
}

/// Load configuration from a specific path.
pub fn load_config_from_path(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(ConfigError::ReadError)?;

    // Check if it's a JSONC file and strip comments if needed
    let json_content = if path.extension().map(|e| e == "jsonc").unwrap_or(false) {
        strip_jsonc(&content)
    } else {
        content
    };

    // Parse JSON
    let mut config: Config =
        serde_json::from_str(&json_content).map_err(ConfigError::ParseError)?;

    // Get config directory for relative path resolution
    let config_dir = path.parent().map(|p| p.to_path_buf());

    // Apply variable substitution
    substitute_config(&mut config, config_dir.as_deref())?;

    // Validate
    validate_config(&config)?;

    Ok(config)
}

/// Validate configuration.
fn validate_config(config: &Config) -> Result<(), ConfigError> {
    // Validate baseURLs
    for (provider_id, provider) in &config.provider {
        if let Some(ref base_url) = provider.options.base_url {
            // Basic URL validation
            if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                return Err(ConfigError::InvalidBaseUrl(format!(
                    "Provider '{}' has invalid baseURL: {}",
                    provider_id, base_url
                )));
            }
        }
    }
    Ok(())
}

/// Save configuration to a file.
pub fn save_config(config: &Config, path: &Path) -> Result<(), ConfigError> {
    let content = serde_json::to_string_pretty(config).map_err(|e| ConfigError::ParseError(e))?;

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(ConfigError::ReadError)?;
    }

    std::fs::write(path, content).map_err(ConfigError::ReadError)?;
    Ok(())
}

/// Get the default config path.
///
/// On Windows: %LOCALAPPDATA%\hcode\config.json
/// On macOS/Linux: ~/.config/hcode/config.json
pub fn default_config_path() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_default_config() {
        // Should return default config when no file exists
        let config = load_config().unwrap();
        assert!(config.model.is_none());
    }

    #[test]
    fn test_load_config_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let json = r#"{
            "model": "anthropic/claude-sonnet-4",
            "provider": {
                "anthropic": {
                    "options": {
                        "apiKey": "test-key"
                    }
                }
            }
        }"#;
        temp_file.write_all(json.as_bytes()).unwrap();

        let config = load_config_from_path(temp_file.path()).unwrap();
        assert_eq!(config.model, Some("anthropic/claude-sonnet-4".to_string()));
        assert!(config.provider.contains_key("anthropic"));
    }

    #[test]
    fn test_env_substitution_in_config() {
        std::env::set_var("TEST_API_KEY", "secret-key-123");

        let mut temp_file = NamedTempFile::new().unwrap();
        let json = r#"{
            "provider": {
                "anthropic": {
                    "options": {
                        "apiKey": "{env:TEST_API_KEY}"
                    }
                }
            }
        }"#;
        temp_file.write_all(json.as_bytes()).unwrap();

        let config = load_config_from_path(temp_file.path()).unwrap();
        assert_eq!(
            config.provider.get("anthropic").unwrap().options.api_key,
            Some("secret-key-123".to_string())
        );

        std::env::remove_var("TEST_API_KEY");
    }
}
