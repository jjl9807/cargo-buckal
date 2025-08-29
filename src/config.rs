use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_buck2_binary")]
    pub buck2_binary: String,
}

fn default_buck2_binary() -> String {
    "buck2".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buck2_binary: default_buck2_binary(),
        }
    }
}

impl Config {
    /// Load configuration from ~/.config/buckal/config.toml
    pub fn load() -> Self {
        let config_path = Self::config_path();
        
        if !config_path.exists() {
            return Self::default();
        }
        
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str::<Config>(&content) {
                    Ok(config) => config,
                    Err(_) => {
                        eprintln!("Warning: Failed to parse config file at {}, using defaults", config_path.display());
                        Self::default()
                    }
                }
            }
            Err(_) => {
                eprintln!("Warning: Failed to read config file at {}, using defaults", config_path.display());
                Self::default()
            }
        }
    }
    
    /// Get the configuration file path
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("buckal").join("config.toml")
    }
    
}