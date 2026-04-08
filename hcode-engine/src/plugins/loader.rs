//! Plugin loader.

use super::types::{LoadedPlugin, PluginManifest};
use std::path::{Path, PathBuf};
use std::fs;

/// Plugin error.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Failed to load plugin: {0}")]
    LoadError(String),
    
    #[error("Invalid plugin format: {0}")]
    InvalidFormat(String),
}

/// Plugin loader.
pub struct PluginLoader {
    /// Directories to search for plugins.
    plugin_dirs: Vec<PathBuf>,
}

impl PluginLoader {
    /// Create a new plugin loader.
    pub fn new(plugin_dirs: Vec<PathBuf>) -> Self {
        Self { plugin_dirs }
    }

    /// Create with default directories.
    pub fn with_defaults() -> Self {
        let dirs = vec![
            PathBuf::from(".hcode/plugins"),
            PathBuf::from(".opencode/plugins"),
            dirs::config_dir()
                .map(|p| p.join("hcode").join("plugins"))
                .unwrap_or_default(),
        ];
        
        Self::new(dirs)
    }

    /// Load all plugins.
    pub fn load_all(&self) -> Result<Vec<LoadedPlugin>, PluginError> {
        let mut plugins = Vec::new();
        
        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }
            
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    if path.is_dir() {
                        if let Ok(plugin) = self.load_plugin(&path) {
                            plugins.push(plugin);
                        }
                    }
                }
            }
        }
        
        Ok(plugins)
    }

    /// Load a single plugin.
    pub fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin, PluginError> {
        // Look for manifest.json or plugin.yaml
        let manifest_path = path.join("manifest.json")
            .exists()
            .then(|| path.join("manifest.json"))
            .or_else(|| {
                path.join("plugin.yaml").exists()
                    .then(|| path.join("plugin.yaml"))
            })
            .or_else(|| {
                path.join("plugin.yml").exists()
                    .then(|| path.join("plugin.yml"))
            });
        
        let manifest_path = manifest_path
            .ok_or_else(|| PluginError::NotFound("No manifest found".to_string()))?;
        
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| PluginError::LoadError(e.to_string()))?;
        
        let manifest: PluginManifest = if manifest_path.extension().map(|e| e == "json").unwrap_or(false) {
            serde_json::from_str(&content)
                .map_err(|e| PluginError::InvalidFormat(e.to_string()))?
        } else {
            serde_yaml::from_str(&content)
                .map_err(|e| PluginError::InvalidFormat(e.to_string()))?
        };
        
        Ok(LoadedPlugin {
            manifest,
            path: path.to_path_buf(),
        })
    }
}