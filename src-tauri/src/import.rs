use crate::cluster_manager::ClusterManagerState;
use kube::config::Kubeconfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;
use std::io::Write;

const MAX_DISCOVERY_DEPTH: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredContext {
    pub context_name: String,
    pub cluster_name: String,
    pub user_name: String,
    pub namespace: Option<String>,
    pub source_file: String,
}

/// Discover all contexts in a single kubeconfig file
pub fn discover_contexts_in_file(path: &Path) -> Result<Vec<DiscoveredContext>, String> {
    let kubeconfig =
        Kubeconfig::read_from(path).map_err(|e| format!("Failed to read kubeconfig: {}", e))?;

    let mut contexts = Vec::new();
    let source_file = path.to_string_lossy().to_string();

    for named_context in kubeconfig.contexts.iter() {
        if let Some(context) = &named_context.context {
            contexts.push(DiscoveredContext {
                context_name: named_context.name.clone(),
                cluster_name: context.cluster.clone(),
                user_name: context.user.clone().unwrap_or_default(),
                namespace: context.namespace.clone(),
                source_file: source_file.clone(),
            });
        }
    }

    Ok(contexts)
}

/// Discover all contexts in all kubeconfig files within a folder (recursively)
pub fn discover_contexts_in_folder(path: &Path) -> Result<Vec<DiscoveredContext>, String> {
    let mut all_contexts = Vec::new();

    if !path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    fn visit_dirs(
        dir: &Path,
        depth: usize,
        contexts: &mut Vec<DiscoveredContext>,
    ) -> Result<(), String> {
        if !dir.is_dir() {
            return Ok(());
        }
        if depth > MAX_DISCOVERY_DEPTH {
            return Ok(());
        }

        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|e| format!("Failed to read metadata: {}", e))?;

            // Skip symlinks to avoid traversal outside scope and cycle-based recursion attacks.
            if metadata.file_type().is_symlink() {
                continue;
            }

            if path.is_dir() {
                // Skip hidden directories and common non-config directories
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "node_modules" {
                        continue;
                    }
                }
                visit_dirs(&path, depth + 1, contexts)?;
            } else if path.is_file() {
                // Try to parse as kubeconfig (skip if it fails)
                if let Ok(file_contexts) = discover_contexts_in_file(&path) {
                    contexts.extend(file_contexts);
                }
            }
        }

        Ok(())
    }

    visit_dirs(path, 0, &mut all_contexts)?;
    Ok(all_contexts)
}

/// Extract a single context from a kubeconfig file and create a new single-context config
pub fn extract_context(
    source_path: &Path,
    context_name: &str,
    cluster_id: &str,
) -> Result<PathBuf, String> {
    let kubeconfig = Kubeconfig::read_from(source_path)
        .map_err(|e| format!("Failed to read kubeconfig: {}", e))?;

    // Find the context
    let context = kubeconfig
        .contexts
        .iter()
        .find(|c| c.name == context_name)
        .ok_or_else(|| format!("Context '{}' not found", context_name))?;

    let ctx = context
        .context
        .as_ref()
        .ok_or_else(|| "Context has no context field".to_string())?;

    // Find the associated cluster and user
    let cluster = kubeconfig
        .clusters
        .iter()
        .find(|c| c.name == ctx.cluster)
        .ok_or_else(|| format!("Cluster '{}' not found", ctx.cluster))?;

    let user_name = ctx
        .user
        .as_ref()
        .ok_or_else(|| "Context has no user".to_string())?;
    let user = kubeconfig
        .auth_infos
        .iter()
        .find(|u| &u.name == user_name)
        .ok_or_else(|| format!("User '{}' not found", user_name))?;

    // Create new isolated kubeconfig
    let new_config = Kubeconfig {
        clusters: vec![cluster.clone()],
        auth_infos: vec![user.clone()],
        contexts: vec![context.clone()],
        current_context: Some(context_name.to_string()),
        ..Default::default()
    };

    // Save to ~/.kore/kubeconfigs/<cluster_id>.yaml
    let kubeconfigs_dir = crate::config::get_kubeconfigs_dir();
    let config_path = kubeconfigs_dir.join(format!("{}.yaml", cluster_id));

    // Serialize and write the kubeconfig
    let yaml_content = serde_yaml::to_string(&new_config)
        .map_err(|e| format!("Failed to serialize kubeconfig: {}", e))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&config_path)
        .map_err(|e| format!("Failed to open kubeconfig for writing: {}", e))?;
    file.write_all(yaml_content.as_bytes())
        .map_err(|e| format!("Failed to write kubeconfig: {}", e))?;
    crate::config::set_owner_only_file_permissions(&config_path)
        .map_err(|e| format!("Failed to set secure permissions: {}", e))?;

    Ok(config_path)
}

// Tauri Commands

#[tauri::command]
pub fn import_discover_file(path: String) -> Result<Vec<DiscoveredContext>, String> {
    let path = PathBuf::from(path);
    discover_contexts_in_file(&path)
}

#[tauri::command]
pub fn import_discover_folder(path: String) -> Result<Vec<DiscoveredContext>, String> {
    let path = PathBuf::from(path);
    discover_contexts_in_folder(&path)
}

#[tauri::command]
pub async fn import_add_cluster(
    name: String,
    context_name: String,
    source_file: String,
    icon: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    state: State<'_, ClusterManagerState>,
) -> Result<String, String> {
    // Generate cluster ID
    let cluster_id = uuid::Uuid::new_v4().to_string();

    // Extract context to isolated config file
    let source_path = PathBuf::from(source_file);
    let config_path = extract_context(&source_path, &context_name, &cluster_id)?;

    // Add to database
    let manager = state
        .0
        .lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;
    let cluster = manager.add_cluster(name, context_name, config_path, icon, description, tags)?;

    Ok(cluster.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_kubeconfig(
        dir: &Path,
        filename: &str,
        contexts: &[(&str, &str, &str)],
    ) -> PathBuf {
        let config_path = dir.join(filename);
        let mut file = fs::File::create(&config_path).unwrap();

        writeln!(file, "apiVersion: v1").unwrap();
        writeln!(file, "kind: Config").unwrap();
        writeln!(file, "current-context: {}", contexts[0].0).unwrap();
        writeln!(file, "clusters:").unwrap();

        for (_, cluster_name, _) in contexts {
            writeln!(file, "- name: {}", cluster_name).unwrap();
            writeln!(file, "  cluster:").unwrap();
            writeln!(file, "    server: https://example.com").unwrap();
        }

        writeln!(file, "users:").unwrap();
        for (_, _, user_name) in contexts {
            writeln!(file, "- name: {}", user_name).unwrap();
            writeln!(file, "  user:").unwrap();
            writeln!(file, "    token: test-token").unwrap();
        }

        writeln!(file, "contexts:").unwrap();
        for (context_name, cluster_name, user_name) in contexts {
            writeln!(file, "- name: {}", context_name).unwrap();
            writeln!(file, "  context:").unwrap();
            writeln!(file, "    cluster: {}", cluster_name).unwrap();
            writeln!(file, "    user: {}", user_name).unwrap();
        }

        config_path
    }

    #[test]
    fn test_discover_contexts_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_kubeconfig(
            temp_dir.path(),
            "config",
            &[
                ("prod-context", "prod-cluster", "prod-user"),
                ("dev-context", "dev-cluster", "dev-user"),
            ],
        );

        let contexts = discover_contexts_in_file(&config_path).unwrap();

        assert_eq!(contexts.len(), 2);
        assert_eq!(contexts[0].context_name, "prod-context");
        assert_eq!(contexts[0].cluster_name, "prod-cluster");
        assert_eq!(contexts[1].context_name, "dev-context");
        assert_eq!(contexts[1].cluster_name, "dev-cluster");
    }

    #[test]
    fn test_discover_contexts_in_folder() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple kubeconfig files
        create_test_kubeconfig(temp_dir.path(), "config1", &[("ctx1", "cluster1", "user1")]);

        create_test_kubeconfig(temp_dir.path(), "config2", &[("ctx2", "cluster2", "user2")]);

        // Create subdirectory with another config
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        create_test_kubeconfig(&subdir, "config3", &[("ctx3", "cluster3", "user3")]);

        let contexts = discover_contexts_in_folder(temp_dir.path()).unwrap();

        assert_eq!(contexts.len(), 3);
        assert!(contexts.iter().any(|c| c.context_name == "ctx1"));
        assert!(contexts.iter().any(|c| c.context_name == "ctx2"));
        assert!(contexts.iter().any(|c| c.context_name == "ctx3"));
    }

    #[test]
    fn test_extract_context() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_kubeconfig(
            temp_dir.path(),
            "config",
            &[
                ("prod-context", "prod-cluster", "prod-user"),
                ("dev-context", "dev-cluster", "dev-user"),
            ],
        );

        // Mock the kubeconfigs directory
        let kubeconfigs_dir = temp_dir.path().join("kubeconfigs");
        fs::create_dir(&kubeconfigs_dir).unwrap();

        // Note: In actual test we'd need to mock get_kubeconfigs_dir()
        // For now, this tests the parsing logic
        let result = discover_contexts_in_file(&config_path);
        assert!(result.is_ok());

        let contexts = result.unwrap();
        assert!(contexts.iter().any(|c| c.context_name == "prod-context"));
    }

    #[test]
    fn test_invalid_kubeconfig() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_path = temp_dir.path().join("invalid.yaml");

        let mut file = fs::File::create(&invalid_path).unwrap();
        writeln!(file, "invalid: yaml: content:").unwrap();

        let result = discover_contexts_in_file(&invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_file() {
        let result = discover_contexts_in_file(Path::new("/nonexistent/path/config"));
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_folder() {
        let temp_dir = TempDir::new().unwrap();
        let contexts = discover_contexts_in_folder(temp_dir.path()).unwrap();
        assert_eq!(contexts.len(), 0);
    }

    #[test]
    fn test_discover_contexts_respects_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let mut current = temp_dir.path().to_path_buf();

        for i in 0..MAX_DISCOVERY_DEPTH {
            current = current.join(format!("level-{}", i));
            fs::create_dir(&current).unwrap();
        }

        create_test_kubeconfig(&current, "within-limit.yaml", &[("ctx-within", "c1", "u1")]);

        let too_deep = current.join("too-deep");
        fs::create_dir(&too_deep).unwrap();
        create_test_kubeconfig(&too_deep, "too-deep.yaml", &[("ctx-too-deep", "c2", "u2")]);

        let contexts = discover_contexts_in_folder(temp_dir.path()).unwrap();
        assert!(contexts.iter().any(|c| c.context_name == "ctx-within"));
        assert!(!contexts.iter().any(|c| c.context_name == "ctx-too-deep"));
    }

    #[cfg(unix)]
    #[test]
    fn test_discover_contexts_skips_symlink_dirs() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let external_target_root = TempDir::new().unwrap();
        let real_dir = temp_dir.path().join("real");
        fs::create_dir(&real_dir).unwrap();
        create_test_kubeconfig(&real_dir, "real.yaml", &[("ctx-real", "c1", "u1")]);

        let linked_target = external_target_root.path().join("linked-target");
        fs::create_dir(&linked_target).unwrap();
        create_test_kubeconfig(
            &linked_target,
            "linked.yaml",
            &[("ctx-via-symlink", "c2", "u2")],
        );

        let link_path = real_dir.join("link");
        symlink(&linked_target, &link_path).unwrap();

        let contexts = discover_contexts_in_folder(temp_dir.path()).unwrap();
        assert!(contexts.iter().any(|c| c.context_name == "ctx-real"));
        assert!(!contexts.iter().any(|c| c.context_name == "ctx-via-symlink"));
    }
}
