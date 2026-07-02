use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryConfig {
    pub name: String,
    pub path: String,
    pub encrypt: bool,
    #[serde(default = "default_one_way_sync")]
    pub one_way_sync: bool,
}

fn default_one_way_sync() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub target_dir: String,
    pub global_exclude: Vec<String>,
    pub repositories: Vec<RepositoryConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_dir: String::new(),
            global_exclude: vec![
                ".DS_Store".to_string(),
                "node_modules/".to_string(),
                "target/".to_string(),
                "bin/".to_string(),
                "obj/".to_string(),
                ".idea/".to_string(),
                ".vscode/".to_string(),
                "DerivedData/".to_string(),
                "*.xcodeproj/xcuserdata/".to_string(),
                "*.xcworkspace/xcuserdata/".to_string(),
                "*.xcodeproj/project.xcworkspace/xcuserdata/".to_string(),
                "ephemeral/".to_string(),
                "Pods/".to_string(),
                "gradle-wrapper.jar".to_string(),
                "GeneratedPluginRegistrant.*".to_string(),
                "Generated.xcconfig".to_string(),
                "generated_plugin*".to_string(),
                "__pycache__/".to_string(),
                "venv/".to_string(),
                ".venv/".to_string(),
                "env/".to_string(),
                ".gradle/".to_string(),
                ".dart_tool/".to_string(),
                "vendor/".to_string(),
                "*.log".to_string(),
            ],
            repositories: Vec::new(),
        }
    }
}

pub fn get_app_dir() -> Result<PathBuf, String> {
    let home = if let Ok(home) = std::env::var("HOME") {
        // macOS / Linux
        PathBuf::from(home)
    } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
        // Windows
        PathBuf::from(user_profile)
    } else {
        return Err("Could not determine user home directory".to_string());
    };

    Ok(home.join(".loar"))
}

pub fn get_config_path() -> Result<PathBuf, String> {
    let app_dir = get_app_dir()?;
    Ok(app_dir.join("loar.toml"))
}

pub fn load_config() -> Result<AppConfig, String> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: AppConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let app_dir = get_app_dir()?;
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_path = app_dir.join("loar.toml");
    let content = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}
