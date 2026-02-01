// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod k8s;
mod config;
mod cluster_manager;
mod import;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Init directories
    let _ = config::init_directories();

    // Initialize cluster manager
    let db_path = config::get_app_config_dir().join("clusters.db");
    let cluster_manager = cluster_manager::ClusterManager::new(db_path)
        .expect("Failed to initialize cluster manager");
    let cluster_manager_state = cluster_manager::ClusterManagerState(std::sync::Mutex::new(cluster_manager));

    tauri::Builder::default()
        .plugin(tauri_plugin_websocket::init())
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(cluster_manager_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            // Legacy k8s commands (deprecated, kept for backwards compatibility)
            k8s::list_contexts,
            k8s::list_namespaces,
            k8s::list_pods,
            k8s::delete_pod,
            k8s::get_pod_events,
            k8s::stream_container_logs,
            k8s::start_pod_watch,
            // NEW: Cluster-based k8s commands
            k8s::cluster_list_namespaces,
            k8s::cluster_list_pods,
            k8s::cluster_delete_pod,
            k8s::cluster_get_pod_events,
            k8s::cluster_stream_container_logs,
            k8s::cluster_start_pod_watch,
            // Cluster management commands
            cluster_manager::db_list_clusters,
            cluster_manager::db_get_cluster,
            cluster_manager::db_migrate_legacy_configs,
            cluster_manager::db_update_cluster,
            cluster_manager::db_update_last_accessed,
            cluster_manager::db_delete_cluster,
            // Import commands
            import::import_discover_file,
            import::import_discover_folder,
            import::import_add_cluster,
            // Legacy config
            config::import_kubeconfig
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
