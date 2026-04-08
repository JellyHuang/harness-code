//! Plugin types.

use serde::{Deserialize, Serialize};

/// Plugin manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    /// Plugin name.
    pub name: String,
    
    /// Plugin version.
    pub version: String,
    
    /// Plugin description.
    #[serde(default)]
    pub description: Option<String>,
    
    /// Plugin author.
    #[serde(default)]
    pub author: Option<String>,
    
    /// Entry point (WASM or native).
    #[serde(default)]
    pub main: Option<String>,
    
    /// Tools provided by the plugin.
    #[serde(default)]
    pub tools: Vec<PluginTool>,
    
    /// Hooks provided by the plugin.
    #[serde(default)]
    pub hooks: Vec<PluginHook>,
    
    /// Commands provided by the plugin.
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

/// Tool definition from plugin.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginTool {
    /// Tool name.
    pub name: String,
    
    /// Tool description.
    pub description: String,
    
    /// Tool input schema.
    pub input_schema: serde_json::Value,
}

/// Hook definition from plugin.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginHook {
    /// Event to hook.
    pub event: String,
    
    /// Handler command.
    pub handler: String,
}

/// Command definition from plugin.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginCommand {
    /// Command name.
    pub name: String,
    
    /// Command description.
    pub description: String,
    
    /// Command handler.
    pub handler: String,
}

/// Loaded plugin.
#[derive(Debug)]
pub struct LoadedPlugin {
    /// Plugin manifest.
    pub manifest: PluginManifest,
    
    /// Plugin path.
    pub path: std::path::PathBuf,
}