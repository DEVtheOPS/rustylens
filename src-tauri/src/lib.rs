// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod cluster_manager;
mod config;
mod image_utils;
mod import;
mod k8s;

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
    let cluster_manager_state = cluster_manager::ClusterManagerState(std::sync::Arc::new(
        std::sync::Mutex::new(cluster_manager),
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(cluster_manager_state)
        .manage(k8s::WatcherState::default())
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
            k8s::cluster_get_metrics,
            k8s::cluster_get_events,
            // Workload commands
            k8s::cluster_list_deployments,
            k8s::cluster_delete_deployment,
            k8s::cluster_list_statefulsets,
            k8s::cluster_delete_statefulset,
            k8s::cluster_list_daemonsets,
            k8s::cluster_delete_daemonset,
            k8s::cluster_list_replicasets,
            k8s::cluster_delete_replicaset,
            k8s::cluster_list_jobs,
            k8s::cluster_delete_job,
            k8s::cluster_list_cronjobs,
            k8s::cluster_delete_cronjob,
            // Config & Network & Storage
            k8s::cluster_list_config_maps,
            k8s::cluster_delete_config_map,
            k8s::cluster_list_secrets,
            k8s::cluster_delete_secret,
            k8s::cluster_list_resource_quotas,
            k8s::cluster_delete_resource_quota,
            k8s::cluster_list_limit_ranges,
            k8s::cluster_delete_limit_range,
            k8s::cluster_list_hpa,
            k8s::cluster_delete_hpa,
            k8s::cluster_list_pdb,
            k8s::cluster_delete_pdb,
            k8s::cluster_list_services,
            k8s::cluster_delete_service,
            k8s::cluster_list_endpoints,
            k8s::cluster_delete_endpoint,
            k8s::cluster_list_ingresses,
            k8s::cluster_delete_ingress,
            k8s::cluster_list_network_policies,
            k8s::cluster_delete_network_policy,
            k8s::cluster_list_pvc,
            k8s::cluster_delete_pvc,
            k8s::cluster_list_pv,
            k8s::cluster_delete_pv,
            k8s::cluster_list_storage_classes,
            k8s::cluster_delete_storage_class,
            k8s::cluster_list_service_accounts,
            k8s::cluster_delete_service_account,
            k8s::cluster_list_roles,
            k8s::cluster_delete_role,
            k8s::cluster_list_cluster_roles,
            k8s::cluster_delete_cluster_role,
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
            // Image processing
            image_utils::process_icon_file,
            // Legacy config
            config::import_kubeconfig
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
