use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    #[allow(dead_code)]
    pub kubeconfig_paths: Vec<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            kubeconfig_paths: vec![],
        }
    }
}

pub fn get_app_config_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Could not find home directory");
    path.push(".rustylens");
    path
}

pub fn get_kubeconfigs_dir() -> PathBuf {
    let mut path = get_app_config_dir();
    path.push("kubeconfigs");
    path
}

pub fn init_directories() -> std::io::Result<()> {
    let app_dir = get_app_config_dir();
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }

    let kube_dir = get_kubeconfigs_dir();
    if !kube_dir.exists() {
        fs::create_dir_all(&kube_dir)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn import_kubeconfig(path: String) -> Result<String, String> {
    let source = PathBuf::from(path);
    if !source.exists() {
        return Err("Source file does not exist".to_string());
    }

    let file_name = source
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();

    // Add timestamp to prevent overwrites or just unique ID
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let dest_name = format!("{}_{}", timestamp, file_name);
    let dest_path = get_kubeconfigs_dir().join(&dest_name);

    fs::copy(&source, &dest_path).map_err(|e| format!("Failed to copy config: {}", e))?;

    Ok(dest_name)
}
