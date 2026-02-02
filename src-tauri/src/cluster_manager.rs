use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: String,
    pub name: String,
    pub context_name: String,
    pub config_path: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub tags: String, // JSON-encoded array
    pub created_at: i64,
    pub last_accessed: i64,
}

pub struct ClusterManager {
    conn: Mutex<Connection>,
}

impl ClusterManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

        // Create clusters table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clusters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                context_name TEXT NOT NULL,
                config_path TEXT NOT NULL,
                icon TEXT,
                description TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create clusters table: {}", e))?;

        Ok(ClusterManager {
            conn: Mutex::new(conn),
        })
    }

    pub fn add_cluster(
        &self,
        name: String,
        context_name: String,
        config_path: PathBuf,
        icon: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<Cluster, String> {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let tags_json =
            serde_json::to_string(&tags).map_err(|e| format!("Failed to serialize tags: {}", e))?;

        let config_path_str = config_path.to_string_lossy().to_string();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clusters (id, name, context_name, config_path, icon, description, tags, created_at, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &id,
                &name,
                &context_name,
                &config_path_str,
                &icon,
                &description,
                &tags_json,
                now,
                now,
            ],
        )
        .map_err(|e| format!("Failed to insert cluster: {}", e))?;

        Ok(Cluster {
            id,
            name,
            context_name,
            config_path: config_path_str,
            icon,
            description,
            tags: tags_json,
            created_at: now,
            last_accessed: now,
        })
    }

    pub fn list_clusters(&self) -> Result<Vec<Cluster>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, context_name, config_path, icon, description, tags, created_at, last_accessed FROM clusters ORDER BY last_accessed DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let clusters = stmt
            .query_map([], |row| {
                Ok(Cluster {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    context_name: row.get(2)?,
                    config_path: row.get(3)?,
                    icon: row.get(4)?,
                    description: row.get(5)?,
                    tags: row.get(6)?,
                    created_at: row.get(7)?,
                    last_accessed: row.get(8)?,
                })
            })
            .map_err(|e| format!("Failed to query clusters: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect clusters: {}", e))?;

        Ok(clusters)
    }

    pub fn get_cluster(&self, id: &str) -> Result<Option<Cluster>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, context_name, config_path, icon, description, tags, created_at, last_accessed FROM clusters WHERE id = ?1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let cluster = stmt
            .query_row([id], |row| {
                Ok(Cluster {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    context_name: row.get(2)?,
                    config_path: row.get(3)?,
                    icon: row.get(4)?,
                    description: row.get(5)?,
                    tags: row.get(6)?,
                    created_at: row.get(7)?,
                    last_accessed: row.get(8)?,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to query cluster: {}", e))?;

        Ok(cluster)
    }

    pub fn update_cluster(
        &self,
        id: &str,
        name: Option<String>,
        icon: Option<Option<String>>,
        description: Option<Option<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        // Build dynamic UPDATE query based on provided fields
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(name_val) = name {
            updates.push("name = ?");
            params.push(Box::new(name_val));
        }

        if let Some(icon_val) = icon {
            updates.push("icon = ?");
            params.push(Box::new(icon_val));
        }

        if let Some(desc_val) = description {
            updates.push("description = ?");
            params.push(Box::new(desc_val));
        }

        if let Some(tags_val) = tags {
            let tags_json = serde_json::to_string(&tags_val)
                .map_err(|e| format!("Failed to serialize tags: {}", e))?;
            updates.push("tags = ?");
            params.push(Box::new(tags_json));
        }

        if updates.is_empty() {
            return Ok(());
        }

        let query = format!("UPDATE clusters SET {} WHERE id = ?", updates.join(", "));
        params.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();

        conn.execute(&query, param_refs.as_slice())
            .map_err(|e| format!("Failed to update cluster: {}", e))?;

        Ok(())
    }

    pub fn update_last_accessed(&self, id: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clusters SET last_accessed = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| format!("Failed to update last_accessed: {}", e))?;

        Ok(())
    }

    pub fn delete_cluster(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clusters WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete cluster: {}", e))?;

        Ok(())
    }
}

// Tauri commands
use tauri::State;

pub struct ClusterManagerState(pub Arc<Mutex<ClusterManager>>);

#[tauri::command]
pub fn db_list_clusters(state: State<ClusterManagerState>) -> Result<Vec<Cluster>, String> {
    let manager = state.0.lock().unwrap();
    manager.list_clusters()
}

#[tauri::command]
pub fn db_get_cluster(
    id: String,
    state: State<ClusterManagerState>,
) -> Result<Option<Cluster>, String> {
    let manager = state.0.lock().unwrap();
    manager.get_cluster(&id)
}

#[tauri::command]
pub fn db_update_cluster(
    id: String,
    name: Option<String>,
    icon: Option<Option<String>>,
    description: Option<Option<String>>,
    tags: Option<Vec<String>>,
    state: State<ClusterManagerState>,
) -> Result<(), String> {
    let manager = state.0.lock().unwrap();
    manager.update_cluster(&id, name, icon, description, tags)
}

#[tauri::command]
pub fn db_update_last_accessed(
    id: String,
    state: State<ClusterManagerState>,
) -> Result<(), String> {
    let manager = state.0.lock().unwrap();
    manager.update_last_accessed(&id)
}

#[tauri::command]
pub fn db_delete_cluster(id: String, state: State<ClusterManagerState>) -> Result<(), String> {
    let manager = state.0.lock().unwrap();

    // Get cluster to find config file path
    if let Some(cluster) = manager.get_cluster(&id)? {
        // Delete the config file
        let config_path = PathBuf::from(&cluster.config_path);
        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .map_err(|e| format!("Failed to delete config file: {}", e))?;
        }
    }

    // Delete from database
    manager.delete_cluster(&id)
}

#[tauri::command]
pub fn db_migrate_legacy_configs(state: State<ClusterManagerState>) -> Result<Vec<String>, String> {
    use crate::import::{discover_contexts_in_folder, extract_context};

    let manager = state.0.lock().unwrap();
    let kubeconfigs_dir = crate::config::get_kubeconfigs_dir();

    if !kubeconfigs_dir.exists() {
        return Ok(vec![]); // No legacy configs to migrate
    }

    // Discover all contexts from the kubeconfigs directory
    let discovered = discover_contexts_in_folder(&kubeconfigs_dir)
        .map_err(|e| format!("Failed to discover legacy configs: {}", e))?;

    let mut migrated = Vec::new();
    let conn = manager.conn.lock().unwrap();

    for ctx in discovered {
        // Check if this context already exists in the database
        let existing = conn
            .query_row(
                "SELECT COUNT(*) FROM clusters WHERE context_name = ?1",
                [&ctx.context_name],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        if existing > 0 {
            // Already migrated, skip
            continue;
        }

        // Import this context
        let id = uuid::Uuid::new_v4().to_string();

        // Extract this context to a new file
        let config_path =
            match extract_context(&PathBuf::from(&ctx.source_file), &ctx.context_name, &id) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Failed to extract context {}: {}", ctx.context_name, e);
                    continue;
                }
            };

        // Add to database
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO clusters (id, name, context_name, config_path, created_at, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &id,
                &ctx.context_name, // Use context name as display name initially
                &ctx.context_name,
                config_path.to_string_lossy().to_string(),
                now,
                now,
            ),
        )
        .map_err(|e| format!("Failed to insert cluster: {}", e))?;

        migrated.push(ctx.context_name);
    }

    Ok(migrated)
}
