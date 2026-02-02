// ... imports ...
use crate::cluster_manager::ClusterManagerState;
use crate::config;
use futures::StreamExt;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Endpoints, Event, LimitRange, Node, PersistentVolume, PersistentVolumeClaim, Pod,
    ResourceQuota, Secret, Service, ServiceAccount,
};
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use k8s_openapi::api::rbac::v1::{ClusterRole, Role};
use k8s_openapi::api::storage::v1::StorageClass;
use kube::config::Kubeconfig;
use kube::runtime::watcher;
use kube::{Api, Client, Config};
use std::path::PathBuf;
use tauri::{Emitter, State, Window};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::JoinHandle;

pub struct WatcherState(pub Arc<Mutex<HashMap<String, JoinHandle<()>>>>);

impl Default for WatcherState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

// Helper to find which file contains the context
fn find_kubeconfig_path_for_context(context_name: &str) -> Option<PathBuf> {
    // 1. Standard locations
    let mut paths = vec![];
    if let Ok(p) = std::env::var("KUBECONFIG") {
        paths.push(PathBuf::from(p));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".kube").join("config"));
    }

    // 2. Custom app config directory
    let app_kube_dir = config::get_kubeconfigs_dir();
    if app_kube_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(app_kube_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    paths.push(path);
                }
            }
        }
    }

    // Check each file
    for path in paths {
        if path.exists() {
            if let Ok(config) = Kubeconfig::read_from(&path) {
                for ctx in config.contexts {
                    if ctx.name == context_name {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

// Helper to create client
async fn create_client_for_context(context_name: &str) -> Result<Client, String> {
    let config_path = find_kubeconfig_path_for_context(context_name).ok_or_else(|| {
        format!(
            "Context '{}' not found in any kubeconfig file",
            context_name
        )
    })?;

    let kubeconfig = Kubeconfig::read_from(&config_path)
        .map_err(|e| format!("Failed to read kubeconfig {:?}: {}", config_path, e))?;

    let options = kube::config::KubeConfigOptions {
        context: Some(context_name.to_string()),
        ..Default::default()
    };

    let config = Config::from_custom_kubeconfig(kubeconfig, &options)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;

    Client::try_from(config).map_err(|e| format!("Failed to create client: {}", e))
}

// NEW: Helper to create client from cluster ID
async fn create_client_for_cluster(cluster_id: &str, state: &State<'_, ClusterManagerState>) -> Result<Client, String> {
    let manager = state.0.clone();
    let cluster_id = cluster_id.to_string();

    // 1. Blocking I/O (DB + File Read)
    let kubeconfig = tauri::async_runtime::spawn_blocking(move || {
        // Get config path
        let config_path = {
            let manager = manager.lock().unwrap();
            let cluster = manager.get_cluster(&cluster_id)?
                .ok_or_else(|| format!("Cluster '{}' not found", cluster_id))?;
            PathBuf::from(&cluster.config_path)
        };
        
        if !config_path.exists() {
            return Err(format!("Config file not found: {:?}", config_path));
        }

        let kubeconfig = Kubeconfig::read_from(&config_path)
            .map_err(|e| format!("Failed to read kubeconfig {:?}: {}", config_path, e))?;

        Ok(kubeconfig)
    }).await.map_err(|e| e.to_string())??;

    // 2. Async Config Loading
    // The extracted config should have only one context, use current_context
    let context_name = kubeconfig.current_context.as_ref()
        .ok_or_else(|| "No current context in kubeconfig".to_string())?;

    let options = kube::config::KubeConfigOptions {
        context: Some(context_name.clone()),
        ..Default::default()
    };

    let config = Config::from_custom_kubeconfig(kubeconfig, &options)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;

    Client::try_from(config).map_err(|e| format!("Failed to create client: {}", e))
}

#[tauri::command]
pub async fn list_contexts() -> Result<Vec<String>, String> {
    let mut paths = vec![];
    if let Ok(p) = std::env::var("KUBECONFIG") {
        paths.push(PathBuf::from(p));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".kube").join("config"));
    }

    let app_kube_dir = config::get_kubeconfigs_dir();
    if app_kube_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(app_kube_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    paths.push(path);
                }
            }
        }
    }

    let mut contexts = Vec::new();
    for path in paths {
        if path.exists() {
            if let Ok(config) = Kubeconfig::read_from(&path) {
                for ctx in config.contexts {
                    contexts.push(ctx.name);
                }
            }
        }
    }

    if contexts.is_empty() {
        return Ok(vec![]);
    }

    contexts.sort();
    contexts.dedup();

    Ok(contexts)
}

#[tauri::command]
pub async fn list_namespaces(context_name: String) -> Result<Vec<String>, String> {
    use k8s_openapi::api::core::v1::Namespace;
    use kube::api::ListParams;

    let client = create_client_for_context(&context_name).await?;
    let ns_api: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();

    let list = ns_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list namespaces: {}", e))?;

    let names = list
        .items
        .into_iter()
        .filter_map(|ns| ns.metadata.name)
        .collect();

    Ok(names)
}

fn probe_to_info(probe_type: &str, probe: &k8s_openapi::api::core::v1::Probe) -> ProbeInfo {
    let (handler_type, details) = if let Some(http) = probe.http_get.as_ref() {
        let path = http.path.clone().unwrap_or_else(|| "/".to_string());
        let port = match &http.port {
            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(n) => n.to_string(),
            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::String(s) => s.clone(),
        };
        let scheme = http.scheme.clone().unwrap_or_else(|| "HTTP".to_string());
        (
            "httpGet".to_string(),
            format!("{}://{}:{}{}", scheme, "localhost", port, path),
        )
    } else if let Some(tcp) = probe.tcp_socket.as_ref() {
        let port = match &tcp.port {
            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(n) => n.to_string(),
            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::String(s) => s.clone(),
        };
        ("tcpSocket".to_string(), format!(":{}", port))
    } else if let Some(exec) = probe.exec.as_ref() {
        let command = exec
            .command
            .as_ref()
            .map(|c| c.join(" "))
            .unwrap_or_default();
        ("exec".to_string(), command)
    } else {
        ("unknown".to_string(), "".to_string())
    };

    ProbeInfo {
        probe_type: probe_type.to_string(),
        handler_type,
        details,
        initial_delay_seconds: probe.initial_delay_seconds.unwrap_or(0),
        period_seconds: probe.period_seconds.unwrap_or(10),
        timeout_seconds: probe.timeout_seconds.unwrap_or(1),
        success_threshold: probe.success_threshold.unwrap_or(1),
        failure_threshold: probe.failure_threshold.unwrap_or(3),
    }
}

fn map_pod_to_summary(p: Pod) -> PodSummary {
    let status = p
        .status
        .as_ref()
        .map(|s| s.phase.clone().unwrap_or_default())
        .unwrap_or_default();
    let name = p.metadata.name.clone().unwrap_or_default();
    let namespace = p.metadata.namespace.clone().unwrap_or_default();
    let age = p
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|t| {
            // k8s-openapi 0.27 uses `jiff` by default or `chrono` if configured, but t.0 returns the inner type
            // Convert timestamp string to chrono DateTime to be safe across versions or just parse it
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&t.0.to_string()) {
                let duration = chrono::Utc::now().signed_duration_since(ts);
                let days = duration.num_days();
                if days > 0 {
                    format!("{}d", days)
                } else {
                    let hours = duration.num_hours();
                    if hours > 0 {
                        format!("{}h", hours)
                    } else {
                        let minutes = duration.num_minutes();
                        if minutes > 0 {
                            format!("{}m", minutes)
                        } else {
                            format!("{}s", duration.num_seconds())
                        }
                    }
                }
            } else {
                "-".to_string()
            }
        })
        .unwrap_or_default();

    let creation_timestamp = p
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|t| t.0.to_string());

    let node = p
        .spec
        .as_ref()
        .and_then(|s| s.node_name.clone())
        .unwrap_or_default();

    let container_statuses = p
        .status
        .as_ref()
        .and_then(|s| s.container_statuses.as_ref());
    let containers = container_statuses.map(|s| s.len()).unwrap_or(0);
    let restarts: i32 = container_statuses
        .map(|s| s.iter().map(|cs| cs.restart_count).sum())
        .unwrap_or(0);

    let qos = p
        .status
        .as_ref()
        .and_then(|s| s.qos_class.clone())
        .unwrap_or_default();

    let controlled_by = p
        .metadata
        .owner_references
        .as_ref()
        .and_then(|refs| refs.first())
        .map(|r| format!("{}/{}", r.kind, r.name))
        .unwrap_or_else(|| "-".to_string());

    // Labels and annotations
    let labels = p.metadata.labels.clone().unwrap_or_default();
    let annotations = p.metadata.annotations.clone().unwrap_or_default();

    // Network info
    let pod_ip = p
        .status
        .as_ref()
        .and_then(|s| s.pod_ip.clone())
        .unwrap_or_else(|| "-".to_string());
    let host_ip = p
        .status
        .as_ref()
        .and_then(|s| s.host_ip.clone())
        .unwrap_or_else(|| "-".to_string());

    // Service account
    let service_account = p
        .spec
        .as_ref()
        .and_then(|s| s.service_account_name.clone())
        .unwrap_or_else(|| "default".to_string());

    // Priority class
    let priority_class = p
        .spec
        .as_ref()
        .and_then(|s| s.priority_class_name.clone())
        .unwrap_or_else(|| "-".to_string());

    // Container details
    let mut container_details = Vec::new();
    if let Some(spec) = p.spec.as_ref() {
        for container in &spec.containers {
            let container_status = container_statuses
                .and_then(|statuses| statuses.iter().find(|s| s.name == container.name))
                .cloned();

            let ready = container_status.as_ref().map(|s| s.ready).unwrap_or(false);
            let restart_count = container_status
                .as_ref()
                .map(|s| s.restart_count)
                .unwrap_or(0);

            let state = if let Some(cs) = container_status.as_ref() {
                if cs.state.as_ref().and_then(|s| s.running.as_ref()).is_some() {
                    "Running".to_string()
                } else if cs.state.as_ref().and_then(|s| s.waiting.as_ref()).is_some() {
                    let reason = cs
                        .state
                        .as_ref()
                        .and_then(|s| s.waiting.as_ref())
                        .and_then(|w| w.reason.clone())
                        .unwrap_or_else(|| "Waiting".to_string());
                    format!("Waiting: {}", reason)
                } else if cs
                    .state
                    .as_ref()
                    .and_then(|s| s.terminated.as_ref())
                    .is_some()
                {
                    let reason = cs
                        .state
                        .as_ref()
                        .and_then(|s| s.terminated.as_ref())
                        .and_then(|t| t.reason.clone())
                        .unwrap_or_else(|| "Terminated".to_string());
                    format!("Terminated: {}", reason)
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            };

            let resources = container.resources.as_ref();
            let cpu_request = resources
                .and_then(|r| r.requests.as_ref())
                .and_then(|req| req.get("cpu"))
                .map(|q| q.0.clone());
            let cpu_limit = resources
                .and_then(|r| r.limits.as_ref())
                .and_then(|lim| lim.get("cpu"))
                .map(|q| q.0.clone());
            let memory_request = resources
                .and_then(|r| r.requests.as_ref())
                .and_then(|req| req.get("memory"))
                .map(|q| q.0.clone());
            let memory_limit = resources
                .and_then(|r| r.limits.as_ref())
                .and_then(|lim| lim.get("memory"))
                .map(|q| q.0.clone());

            // Ports
            let ports = container
                .ports
                .as_ref()
                .map(|ports| {
                    ports
                        .iter()
                        .map(|p| ContainerPort {
                            name: p.name.clone(),
                            container_port: p.container_port,
                            host_port: p.host_port,
                            protocol: p.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Environment variables
            let env = container
                .env
                .as_ref()
                .map(|envs| {
                    envs.iter()
                        .map(|e| {
                            let value_from = if e.value_from.is_some() {
                                Some("(from ConfigMap/Secret)".to_string())
                            } else {
                                None
                            };
                            EnvVar {
                                name: e.name.clone(),
                                value: e.value.clone(),
                                value_from,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Volume mounts
            let volume_mounts = container
                .volume_mounts
                .as_ref()
                .map(|mounts| {
                    mounts
                        .iter()
                        .map(|m| VolumeMount {
                            name: m.name.clone(),
                            mount_path: m.mount_path.clone(),
                            sub_path: m.sub_path.clone(),
                            read_only: m.read_only.unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Probes
            let mut probes = Vec::new();
            if let Some(liveness) = container.liveness_probe.as_ref() {
                probes.push(probe_to_info("liveness", liveness));
            }
            if let Some(readiness) = container.readiness_probe.as_ref() {
                probes.push(probe_to_info("readiness", readiness));
            }
            if let Some(startup) = container.startup_probe.as_ref() {
                probes.push(probe_to_info("startup", startup));
            }

            let image_pull_policy = container
                .image_pull_policy
                .clone()
                .unwrap_or_else(|| "IfNotPresent".to_string());

            container_details.push(ContainerInfo {
                name: container.name.clone(),
                image: container.image.clone().unwrap_or_default(),
                image_pull_policy,
                ready,
                restart_count,
                state,
                cpu_request,
                cpu_limit,
                memory_request,
                memory_limit,
                ports,
                env,
                volume_mounts,
                probes,
            });
        }
    }

    // Volumes
    let mut volumes = Vec::new();
    if let Some(spec) = p.spec.as_ref() {
        if let Some(vols) = spec.volumes.as_ref() {
            for vol in vols {
                let volume_type = if vol.config_map.is_some() {
                    "ConfigMap".to_string()
                } else if vol.secret.is_some() {
                    "Secret".to_string()
                } else if vol.empty_dir.is_some() {
                    "EmptyDir".to_string()
                } else if vol.host_path.is_some() {
                    "HostPath".to_string()
                } else if vol.persistent_volume_claim.is_some() {
                    "PersistentVolumeClaim".to_string()
                } else if vol.projected.is_some() {
                    "Projected".to_string()
                } else if vol.downward_api.is_some() {
                    "DownwardAPI".to_string()
                } else {
                    "Other".to_string()
                };

                volumes.push(VolumeInfo {
                    name: vol.name.clone(),
                    volume_type,
                });
            }
        }
    }

    // Conditions
    let mut conditions = Vec::new();
    if let Some(status) = p.status.as_ref() {
        if let Some(conds) = status.conditions.as_ref() {
            for cond in conds {
                conditions.push(PodCondition {
                    condition_type: cond.type_.clone(),
                    status: cond.status.clone(),
                    reason: cond.reason.clone(),
                    message: cond.message.clone(),
                    last_transition_time: cond
                        .last_transition_time
                        .as_ref()
                        .map(|t| t.0.to_string()),
                });
            }
        }
    }

    PodSummary {
        name,
        namespace,
        status,
        age,
        creation_timestamp,
        containers,
        restarts,
        node,
        qos,
        controlled_by,
        labels,
        annotations,
        pod_ip,
        host_ip,
        service_account,
        priority_class,
        container_details,
        volumes,
        conditions,
    }
}

#[tauri::command]
pub async fn list_pods(context_name: String, namespace: String) -> Result<Vec<PodSummary>, String> {
    use kube::api::ListParams;

    let client = create_client_for_context(&context_name).await?;

    let pods: Api<Pod> = if namespace == "all" {
        Api::all(client)
    } else {
        Api::namespaced(client, &namespace)
    };

    let lp = ListParams::default();

    let pod_list = pods
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list pods: {}", e))?;

    let summaries = pod_list.items.into_iter().map(map_pod_to_summary).collect();

    Ok(summaries)
}

#[tauri::command]
pub async fn delete_pod(
    context_name: String,
    namespace: String,
    pod_name: String,
) -> Result<(), String> {
    use kube::api::DeleteParams;

    let client = create_client_for_context(&context_name).await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    pods.delete(&pod_name, &DeleteParams::default())
        .await
        .map_err(|e| format!("Failed to delete pod: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_pod_events(
    context_name: String,
    namespace: String,
    pod_name: String,
) -> Result<Vec<PodEventInfo>, String> {
    use k8s_openapi::api::core::v1::Event;
    use kube::api::ListParams;

    let client = create_client_for_context(&context_name).await?;
    let events: Api<Event> = Api::namespaced(client, &namespace);

    let lp = ListParams::default().fields(&format!("involvedObject.name={}", pod_name));

    let event_list = events
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    let mut event_infos: Vec<PodEventInfo> = event_list
        .items
        .into_iter()
        .map(|e| {
            let source = e
                .source
                .as_ref()
                .and_then(|s| s.component.clone())
                .unwrap_or_else(|| "unknown".to_string());

            PodEventInfo {
                event_type: e.type_.unwrap_or_else(|| "Normal".to_string()),
                reason: e.reason.unwrap_or_default(),
                message: e.message.unwrap_or_default(),
                count: e.count.unwrap_or(1),
                first_timestamp: e.first_timestamp.as_ref().map(|t| t.0.to_string()),
                last_timestamp: e.last_timestamp.as_ref().map(|t| t.0.to_string()),
                source,
            }
        })
        .collect();

    // Sort by last_timestamp descending (most recent first)
    event_infos.sort_by(|a, b| b.last_timestamp.as_ref().cmp(&a.last_timestamp.as_ref()));

    Ok(event_infos)
}

#[tauri::command]
pub async fn stream_container_logs(
    window: Window,
    context_name: String,
    namespace: String,
    pod_name: String,
    container_name: String,
    stream_id: String,
) -> Result<(), String> {
    use futures::{AsyncBufReadExt, TryStreamExt};
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::LogParams;

    let client = create_client_for_context(&context_name).await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let log_params = LogParams {
        follow: true,
        tail_lines: Some(1000),
        container: Some(container_name.clone()),
        ..Default::default()
    };

    // Spawn a task to stream logs
    tauri::async_runtime::spawn(async move {
        match pods.log_stream(&pod_name, &log_params).await {
            Ok(stream) => {
                let mut lines = stream.lines();
                loop {
                    match lines.try_next().await {
                        Ok(Some(line)) => {
                            let event_name = format!("container_logs_{}", stream_id);
                            if let Err(e) = window.emit(&event_name, line) {
                                println!("Failed to emit log line: {}", e);
                                break;
                            }
                        }
                        Ok(None) => {
                            // Stream ended
                            break;
                        }
                        Err(e) => {
                            println!("Error reading log line: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to open log stream: {}", e);
            }
        }
    });

    Ok(())
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum PodEvent {
    Added(PodSummary),
    #[allow(dead_code)]
    Modified(PodSummary),
    Deleted(PodSummary),
    #[allow(dead_code)]
    Restarted(Vec<PodSummary>),
}

// Global variable or state to manage cancellation would be better, but for this demo/clone
// we will just start a new loop. The frontend should handle deduplication or we should use an ID.
// Note: This naive approach might spawn multiple watchers if called repeatedly.
// In a real app, use Tauri State with a Mutex<HashMap<String, AbortHandle>>.

#[tauri::command]
pub async fn start_pod_watch(
    window: Window,
    context_name: String,
    namespace: String,
) -> Result<(), String> {
    use kube::runtime::watcher::Config as WatchConfig;

    let client = create_client_for_context(&context_name).await?;

    let api: Api<Pod> = if namespace == "all" {
        Api::all(client)
    } else {
        Api::namespaced(client, &namespace)
    };

    let config = WatchConfig::default();

    // Spawn a task to watch
    tauri::async_runtime::spawn(async move {
        let mut stream = watcher(api, config).boxed();

        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let pod_event = match event {
                        watcher::Event::Apply(pod) => PodEvent::Added(map_pod_to_summary(pod)),
                        watcher::Event::Delete(pod) => PodEvent::Deleted(map_pod_to_summary(pod)),
                        watcher::Event::InitApply(pod) => PodEvent::Added(map_pod_to_summary(pod)),
                        _ => continue,
                    };

                    if let Err(e) = window.emit("pod_event", pod_event) {
                        // Window might be closed
                        println!("Failed to emit event: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    println!("Watch error: {}", e);
                    // Decide whether to break or continue
                }
            }
        }
    });

    Ok(())
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ContainerPort {
    name: Option<String>,
    container_port: i32,
    host_port: Option<i32>,
    protocol: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct EnvVar {
    name: String,
    value: Option<String>,
    value_from: Option<String>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct VolumeMount {
    name: String,
    mount_path: String,
    sub_path: Option<String>,
    read_only: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ProbeInfo {
    probe_type: String,   // "liveness", "readiness", "startup"
    handler_type: String, // "httpGet", "tcpSocket", "exec"
    details: String,
    initial_delay_seconds: i32,
    period_seconds: i32,
    timeout_seconds: i32,
    success_threshold: i32,
    failure_threshold: i32,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ContainerInfo {
    name: String,
    image: String,
    image_pull_policy: String,
    ready: bool,
    restart_count: i32,
    state: String,
    cpu_request: Option<String>,
    cpu_limit: Option<String>,
    memory_request: Option<String>,
    memory_limit: Option<String>,
    ports: Vec<ContainerPort>,
    env: Vec<EnvVar>,
    volume_mounts: Vec<VolumeMount>,
    probes: Vec<ProbeInfo>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct VolumeInfo {
    name: String,
    volume_type: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PodSummary {
    name: String,
    namespace: String,
    status: String,
    age: String,
    creation_timestamp: Option<String>,
    containers: usize,
    restarts: i32,
    node: String,
    qos: String,
    controlled_by: String,
    // Extended details
    labels: std::collections::BTreeMap<String, String>,
    annotations: std::collections::BTreeMap<String, String>,
    pod_ip: String,
    host_ip: String,
    service_account: String,
    priority_class: String,
    container_details: Vec<ContainerInfo>,
    volumes: Vec<VolumeInfo>,
    conditions: Vec<PodCondition>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PodCondition {
    condition_type: String,
    status: String,
    reason: Option<String>,
    message: Option<String>,
    last_transition_time: Option<String>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PodEventInfo {
    event_type: String, // "Normal", "Warning"
    reason: String,
    message: String,
    count: i32,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    source: String,
}

// NEW: Cluster-based commands using cluster IDs

#[tauri::command]
pub async fn cluster_list_namespaces(
    cluster_id: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<String>, String> {
    use k8s_openapi::api::core::v1::Namespace;
    use kube::api::ListParams;

    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let ns_api: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();

    let list = ns_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list namespaces: {}", e))?;

    let namespaces: Vec<String> = list.items.iter().map(|ns| ns.metadata.name.clone().unwrap_or_default()).collect();

    Ok(namespaces)
}

#[tauri::command]
pub async fn cluster_list_pods(
    cluster_id: String,
    namespace: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<PodSummary>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    
    let pods: Api<Pod> = if namespace == "all" {
        Api::all(client)
    } else {
        Api::namespaced(client, &namespace)
    };

    let lp = kube::api::ListParams::default();
    let list = pods
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list pods: {}", e))?;

    let summaries = list.items.iter().map(|p| map_pod_to_summary(p.clone())).collect();
    Ok(summaries)
}

#[tauri::command]
pub async fn cluster_delete_pod(
    cluster_id: String,
    namespace: String,
    pod_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<(), String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    
    pods.delete(&pod_name, &kube::api::DeleteParams::default())
        .await
        .map_err(|e| format!("Failed to delete pod: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn cluster_get_pod_events(
    cluster_id: String,
    namespace: String,
    pod_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<PodEventInfo>, String> {
    use k8s_openapi::api::core::v1::Event;
    use kube::api::ListParams;

    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let events_api: Api<Event> = Api::namespaced(client, &namespace);

    let field_selector = format!("involvedObject.name={}", pod_name);
    let lp = ListParams::default().fields(&field_selector);

    let events_list = events_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    let mut event_infos: Vec<PodEventInfo> = events_list
        .items
        .iter()
        .map(|event| {
            let event_type = event.type_.as_ref().unwrap_or(&"Unknown".to_string()).clone();
            let reason = event.reason.as_ref().unwrap_or(&"Unknown".to_string()).clone();
            let message = event.message.as_ref().unwrap_or(&"".to_string()).clone();
            let count = event.count.unwrap_or(1);
            let first_timestamp = event.first_timestamp.as_ref().map(|t| t.0.to_string());
            let last_timestamp = event.last_timestamp.as_ref().map(|t| t.0.to_string());
            let source = event
                .source
                .as_ref()
                .and_then(|s| s.component.as_ref())
                .cloned()
                .unwrap_or_default();

            PodEventInfo {
                event_type,
                reason,
                message,
                count,
                first_timestamp,
                last_timestamp,
                source,
            }
        })
        .collect();

    event_infos.sort_by(|a, b| b.last_timestamp.cmp(&a.last_timestamp));

    Ok(event_infos)
}

#[tauri::command]
pub async fn cluster_stream_container_logs(
    cluster_id: String,
    namespace: String,
    pod_name: String,
    container_name: String,
    stream_id: String,
    window: Window,
    state: State<'_, ClusterManagerState>,
    watcher_state: State<'_, WatcherState>,
) -> Result<(), String> {
    use futures::{AsyncBufReadExt, TryStreamExt};
    use kube::api::LogParams;

    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    let log_params = LogParams {
        follow: true,
        tail_lines: Some(1000),
        container: Some(container_name.clone()),
        ..Default::default()
    };

    let key = format!("logs:{}", stream_id);
    
    // Abort existing if any
    {
        let mut watchers = watcher_state.0.lock().unwrap();
        if let Some(handle) = watchers.remove(&key) {
            handle.abort();
        }
    }

    let watchers = watcher_state.inner().0.clone();
    let key_clone = key.clone();

    let handle = tauri::async_runtime::spawn(async move {
        match pods.log_stream(&pod_name, &log_params).await {
            Ok(stream) => {
                let mut lines = stream.lines();
                loop {
                    match lines.try_next().await {
                        Ok(Some(line)) => {
                            let event_name = format!("container_logs_{}", stream_id);
                            if let Err(e) = window.emit(&event_name, line) {
                                println!("Failed to emit log line: {}", e);
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            println!("Error reading log line: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to open log stream: {}", e);
            }
        }
        
        // Cleanup
        let mut watchers = watchers.lock().unwrap();
        watchers.remove(&key_clone);
    });

    // Store new handle
    {
        let mut watchers = watcher_state.0.lock().unwrap();
        watchers.insert(key, handle);
    }

    Ok(())
}

#[tauri::command]
pub async fn cluster_start_pod_watch(
    cluster_id: String,
    namespace: String,
    window: Window,
    state: State<'_, ClusterManagerState>,
    watcher_state: State<'_, WatcherState>,
) -> Result<(), String> {
    use kube::runtime::watcher::Config as WatchConfig;

    let client = create_client_for_cluster(&cluster_id, &state).await?;

    let api: Api<Pod> = if namespace == "all" {
        Api::all(client)
    } else {
        Api::namespaced(client, &namespace)
    };

    let config = WatchConfig::default();
    let key = format!("pod_watch:{}:{}", cluster_id, namespace);

    // Abort existing if any
    {
        let mut watchers = watcher_state.0.lock().unwrap();
        if let Some(handle) = watchers.remove(&key) {
            handle.abort();
        }
    }

    let watchers = watcher_state.inner().0.clone();
    let key_clone = key.clone();

    let handle = tauri::async_runtime::spawn(async move {
        let mut stream = watcher(api, config).boxed();

        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let pod_event = match event {
                        watcher::Event::Apply(pod) => PodEvent::Added(map_pod_to_summary(pod)),
                        watcher::Event::Delete(pod) => PodEvent::Deleted(map_pod_to_summary(pod)),
                        watcher::Event::InitApply(pod) => PodEvent::Added(map_pod_to_summary(pod)),
                        _ => continue,
                    };

                    if let Err(e) = window.emit("pod_event", pod_event) {
                        println!("Failed to emit event: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    println!("Watch error: {}", e);
                }
            }
        }
        
        // Cleanup
        let mut watchers = watchers.lock().unwrap();
        watchers.remove(&key_clone);
    });

    // Store new handle
    {
        let mut watchers = watcher_state.0.lock().unwrap();
        watchers.insert(key, handle);
    }

    Ok(())
}

// --- Dashboard Metrics ---

#[derive(serde::Serialize, Default, Debug)]
pub struct ResourceStats {
    pub capacity: f64,
    pub allocatable: f64,
    pub requests: f64,
    pub limits: f64,
    pub usage: f64,
}

#[derive(serde::Serialize, Default, Debug)]
pub struct ClusterMetrics {
    pub cpu: ResourceStats,
    pub memory: ResourceStats,
    pub pods: ResourceStats,
}

#[derive(serde::Serialize, Debug)]
pub struct WarningEvent {
    pub message: String,
    pub object: String,
    pub type_: String,
    pub age: String,
    pub count: i32,
}

fn parse_cpu(q: &str) -> f64 {
    if q.ends_with('m') {
        q.trim_end_matches('m').parse::<f64>().unwrap_or(0.0) / 1000.0
    } else {
        q.parse::<f64>().unwrap_or(0.0)
    }
}

fn parse_memory(q: &str) -> f64 {
    let q = q.trim();
    if let Some(val) = q.strip_suffix("Ki") { val.parse::<f64>().unwrap_or(0.0) * 1024.0 }
    else if let Some(val) = q.strip_suffix("Mi") { val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(2) }
    else if let Some(val) = q.strip_suffix("Gi") { val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(3) }
    else if let Some(val) = q.strip_suffix("Ti") { val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(4) }
    else if let Some(val) = q.strip_suffix("m") { val.parse::<f64>().unwrap_or(0.0) / 1000.0 }
    else { q.parse::<f64>().unwrap_or(0.0) }
}

#[tauri::command]
pub async fn cluster_get_metrics(
    cluster_id: String,
    state: State<'_, ClusterManagerState>,
) -> Result<ClusterMetrics, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    
    let nodes: Api<Node> = Api::all(client.clone());
    let pods: Api<Pod> = Api::all(client.clone());

    let node_list = nodes.list(&Default::default()).await.map_err(|e| e.to_string())?;
    let pod_list = pods.list(&Default::default()).await.map_err(|e| e.to_string())?;

    let mut metrics = ClusterMetrics::default();

    // Node Capacity & Allocatable
    for node in node_list.items {
        if let Some(status) = node.status {
            if let Some(cap) = status.capacity {
                if let Some(cpu) = cap.get("cpu") { metrics.cpu.capacity += parse_cpu(&cpu.0); }
                if let Some(mem) = cap.get("memory") { metrics.memory.capacity += parse_memory(&mem.0); }
                if let Some(p) = cap.get("pods") { metrics.pods.capacity += parse_cpu(&p.0); }
            }
            if let Some(alloc) = status.allocatable {
                if let Some(cpu) = alloc.get("cpu") { metrics.cpu.allocatable += parse_cpu(&cpu.0); }
                if let Some(mem) = alloc.get("memory") { metrics.memory.allocatable += parse_memory(&mem.0); }
                if let Some(p) = alloc.get("pods") { metrics.pods.allocatable += parse_cpu(&p.0); }
            }
        }
    }

    // Pod Requests & Limits
    for pod in pod_list.items {
        // Skip finished pods
        if let Some(status) = &pod.status {
            if let Some(phase) = &status.phase {
                if phase == "Succeeded" || phase == "Failed" { continue; }
            }
        }
        
        metrics.pods.usage += 1.0;

        if let Some(spec) = pod.spec {
            for container in spec.containers {
                if let Some(reqs) = container.resources.as_ref().and_then(|r| r.requests.as_ref()) {
                    if let Some(cpu) = reqs.get("cpu") { metrics.cpu.requests += parse_cpu(&cpu.0); }
                    if let Some(mem) = reqs.get("memory") { metrics.memory.requests += parse_memory(&mem.0); }
                }
                if let Some(lims) = container.resources.as_ref().and_then(|r| r.limits.as_ref()) {
                    if let Some(cpu) = lims.get("cpu") { metrics.cpu.limits += parse_cpu(&cpu.0); }
                    if let Some(mem) = lims.get("memory") { metrics.memory.limits += parse_memory(&mem.0); }
                }
            }
        }
    }

    Ok(metrics)
}

#[tauri::command]
pub async fn cluster_get_events(
    cluster_id: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<WarningEvent>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let events: Api<Event> = Api::all(client);
    
    let lp = kube::api::ListParams::default();
    let event_list = events.list(&lp).await.map_err(|e| e.to_string())?;

    let mut warnings = Vec::new();
    let now = chrono::Utc::now();

    for e in event_list.items {
        if e.type_.as_deref() == Some("Warning") {
            let age = if let Some(last_ts) = &e.last_timestamp {
                let last_ts_str = last_ts.0.to_string();
                let last_ts_parsed = chrono::DateTime::parse_from_rfc3339(&last_ts_str).unwrap().with_timezone(&chrono::Utc);
                let duration = now.signed_duration_since(last_ts_parsed);
                 if duration.num_days() > 0 {
                    format!("{}d", duration.num_days())
                } else if duration.num_hours() > 0 {
                    format!("{}h", duration.num_hours())
                } else if duration.num_minutes() > 0 {
                    format!("{}m", duration.num_minutes())
                } else {
                    format!("{}s", duration.num_seconds())
                }
            } else {
                "-".to_string()
            };

            warnings.push(WarningEvent {
                message: e.message.unwrap_or_default(),
                object: format!("{}/{}", e.involved_object.kind.unwrap_or_default(), e.involved_object.name.unwrap_or_default()),
                type_: e.type_.unwrap_or_default(),
                age,
                count: e.count.unwrap_or(1),
            });
        }
    }

    // Limit to 50 most recent warnings
    warnings.reverse(); 
    warnings.truncate(50);
    
    Ok(warnings)
}

// --- Workload Resources ---

#[derive(serde::Serialize, Clone, Debug)]
pub struct WorkloadSummary {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub age: String,
    pub labels: std::collections::BTreeMap<String, String>,
    pub status: String,
    pub images: Vec<String>,
    pub created_at: i64,
}

fn calculate_age(timestamp: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>) -> String {
    if let Some(ts) = timestamp {
        let now = chrono::Utc::now();
        // Convert k8s Time (jiff/chrono wrapper) to chrono DateTime
        // Using string parsing as reliable fallback
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts.0.to_string()) {
             let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
             if duration.num_days() > 0 {
                format!("{}d", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{}h", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{}m", duration.num_minutes())
            } else {
                format!("{}s", duration.num_seconds())
            }
        } else {
            "-".to_string()
        }
    } else {
        "-".to_string()
    }
}

fn get_created_at(timestamp: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>) -> i64 {
    if let Some(ts) = timestamp {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts.0.to_string()) {
            return dt.timestamp();
        }
    }
    0
}

fn map_deployment_to_summary(d: Deployment) -> WorkloadSummary {
    let meta = d.metadata;
    let spec = d.spec.unwrap_or_default();
    let status = d.status.unwrap_or_default();
    
    let _replicas = status.replicas.unwrap_or(0);
    let ready = status.ready_replicas.unwrap_or(0);
    let status_str = format!("{}/{}", ready, spec.replicas.unwrap_or(1));

    let images = if let Some(template) = spec.template.spec {
        template.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images,
    }
}

fn map_statefulset_to_summary(s: StatefulSet) -> WorkloadSummary {
    let meta = s.metadata;
    let spec = s.spec.unwrap_or_default();
    let status = s.status.unwrap_or_default();
    
    let ready = status.ready_replicas.unwrap_or(0);
    let replicas = spec.replicas.unwrap_or(1);
    let status_str = format!("{}/{}", ready, replicas);

    let images = if let Some(template) = spec.template.spec {
        template.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images,
    }
}

fn map_daemonset_to_summary(d: DaemonSet) -> WorkloadSummary {
    let meta = d.metadata;
    let spec = d.spec.unwrap_or_default();
    let status = d.status.unwrap_or_default();
    
    let desired = status.desired_number_scheduled;
    let ready = status.number_ready;
    let status_str = format!("{}/{}", ready, desired);

    let images = if let Some(template) = spec.template.spec {
        template.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images,
    }
}

fn map_replicaset_to_summary(r: ReplicaSet) -> WorkloadSummary {
    let meta = r.metadata;
    let spec = r.spec.unwrap_or_default();
    let status = r.status.unwrap_or_default();
    
    let ready = status.ready_replicas.unwrap_or(0);
    let replicas = spec.replicas.unwrap_or(1);
    let status_str = format!("{}/{}", ready, replicas);

    let images = if let Some(template) = spec.template {
        if let Some(tspec) = template.spec {
            tspec.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
        } else { vec![] }
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images,
    }
}

fn map_job_to_summary(j: Job) -> WorkloadSummary {
    let meta = j.metadata;
    let spec = j.spec.unwrap_or_default();
    let status = j.status.unwrap_or_default();
    
    let succeeded = status.succeeded.unwrap_or(0);
    let completions = spec.completions.unwrap_or(1);
    let status_str = format!("{}/{}", succeeded, completions);

    let images = if let Some(template) = spec.template.spec {
        template.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images,
    }
}

fn map_cronjob_to_summary(c: CronJob) -> WorkloadSummary {
    let meta = c.metadata;
    let spec = c.spec.unwrap_or_default();
    let status = c.status.unwrap_or_default();
    
    let active = status.active.map(|a| a.len()).unwrap_or(0);
    let status_str = if active > 0 { "Active" } else { "Suspended" }; // Simplified

    let images = if let Some(job_template) = spec.job_template.spec {
        if let Some(template) = job_template.template.spec {
            template.containers.into_iter().map(|c| c.image.unwrap_or_default()).collect()
        } else { vec![] }
    } else {
        vec![]
    };

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str.to_string(),
        images,
    }
}

macro_rules! impl_workload_commands {
    ($resource:ty, $list_fn:ident, $delete_fn:ident, $map_fn:ident) => {
        #[tauri::command]
        pub async fn $list_fn(
            cluster_id: String,
            namespace: Option<String>,
            state: State<'_, ClusterManagerState>,
        ) -> Result<Vec<WorkloadSummary>, String> {
            let client = create_client_for_cluster(&cluster_id, &state).await?;
            let api: Api<$resource> = if let Some(ns) = namespace {
                Api::namespaced(client, &ns)
            } else {
                Api::all(client)
            };

            let list = api.list(&Default::default()).await.map_err(|e| e.to_string())?;
            Ok(list.items.into_iter().map($map_fn).collect())
        }

        #[tauri::command]
        pub async fn $delete_fn(
            cluster_id: String,
            namespace: String,
            name: String,
            state: State<'_, ClusterManagerState>,
        ) -> Result<(), String> {
            let client = create_client_for_cluster(&cluster_id, &state).await?;
            let api: Api<$resource> = Api::namespaced(client, &namespace);
            api.delete(&name, &Default::default()).await.map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

impl_workload_commands!(Deployment, cluster_list_deployments, cluster_delete_deployment, map_deployment_to_summary);
impl_workload_commands!(StatefulSet, cluster_list_statefulsets, cluster_delete_statefulset, map_statefulset_to_summary);
impl_workload_commands!(DaemonSet, cluster_list_daemonsets, cluster_delete_daemonset, map_daemonset_to_summary);
impl_workload_commands!(ReplicaSet, cluster_list_replicasets, cluster_delete_replicaset, map_replicaset_to_summary);
impl_workload_commands!(Job, cluster_list_jobs, cluster_delete_job, map_job_to_summary);
impl_workload_commands!(CronJob, cluster_list_cronjobs, cluster_delete_cronjob, map_cronjob_to_summary);

// --- Additional Resources ---

// Macro for cluster-scoped resources (PV, StorageClass, etc.)
macro_rules! impl_cluster_resource_commands {
    ($resource:ty, $list_fn:ident, $delete_fn:ident, $map_fn:ident) => {
        #[tauri::command]
        pub async fn $list_fn(
            cluster_id: String,
            _namespace: Option<String>,
            state: State<'_, ClusterManagerState>,
        ) -> Result<Vec<WorkloadSummary>, String> {
            let client = create_client_for_cluster(&cluster_id, &state).await?;
            let api: Api<$resource> = Api::all(client);

            let list = api.list(&Default::default()).await.map_err(|e| e.to_string())?;
            Ok(list.items.into_iter().map($map_fn).collect())
        }

        #[tauri::command]
        pub async fn $delete_fn(
            cluster_id: String,
            _namespace: String,
            name: String,
            state: State<'_, ClusterManagerState>,
        ) -> Result<(), String> {
            let client = create_client_for_cluster(&cluster_id, &state).await?;
            let api: Api<$resource> = Api::all(client);
            api.delete(&name, &Default::default()).await.map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

// Config Maps
fn map_configmap_to_summary(c: ConfigMap) -> WorkloadSummary {
    let meta = c.metadata;
    let count = c.data.map(|d| d.len()).unwrap_or(0) + c.binary_data.map(|d| d.len()).unwrap_or(0);
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} items", count),
        images: vec![],
    }
}

// Secrets
fn map_secret_to_summary(s: Secret) -> WorkloadSummary {
    let meta = s.metadata;
    let count = s.data.map(|d| d.len()).unwrap_or(0) + s.string_data.map(|d| d.len()).unwrap_or(0);
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} ({} items)", s.type_.unwrap_or_else(|| "Opaque".to_string()), count),
        images: vec![],
    }
}

// Resource Quotas
fn map_resource_quota_to_summary(r: ResourceQuota) -> WorkloadSummary {
    let meta = r.metadata;
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(), 
        images: vec![],
    }
}

// Limit Ranges
fn map_limit_range_to_summary(l: LimitRange) -> WorkloadSummary {
    let meta = l.metadata;
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![],
    }
}

// HPA
fn map_hpa_to_summary(h: HorizontalPodAutoscaler) -> WorkloadSummary {
    let meta = h.metadata;
    let spec = h.spec.unwrap_or_default();
    let status = h.status.unwrap_or_default();
    
    let current = status.current_replicas;
    let desired = status.desired_replicas;
    let min = spec.min_replicas.unwrap_or(1);
    let max = spec.max_replicas;
    
    let status_str = format!("{}/{} (min: {}, max: {})", current, desired, min, max);

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: status_str,
        images: vec![],
    }
}

// PDB
fn map_pdb_to_summary(p: PodDisruptionBudget) -> WorkloadSummary {
    let meta = p.metadata;
    let status = p.status.unwrap_or_default();
    let allowed = status.disruptions_allowed;
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("Allowed: {}", allowed),
        images: vec![],
    }
}

// Services
fn map_service_to_summary(s: Service) -> WorkloadSummary {
    let meta = s.metadata;
    let spec = s.spec.unwrap_or_default();
    
    let type_ = spec.type_.unwrap_or_else(|| "ClusterIP".to_string());
    let cluster_ip = spec.cluster_ip.unwrap_or_else(|| "-".to_string());
    let ports = spec.ports.unwrap_or_default().iter().map(|p| format!("{}", p.port)).collect::<Vec<_>>().join(",");
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} ({})", type_, cluster_ip),
        images: vec![ports], // Hijacking images field for ports/info
    }
}

// Endpoints
fn map_endpoints_to_summary(e: Endpoints) -> WorkloadSummary {
    let meta = e.metadata;
    let count = e.subsets.map(|s| s.iter().map(|ss| ss.addresses.as_ref().map(|a| a.len()).unwrap_or(0)).sum::<usize>()).unwrap_or(0);

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} endpoints", count),
        images: vec![],
    }
}

// Ingresses
fn map_ingress_to_summary(i: Ingress) -> WorkloadSummary {
    let meta = i.metadata;
    let lbs = i.status.and_then(|s| s.load_balancer).and_then(|lb| lb.ingress).map(|ing| 
        ing.iter().map(|ip| ip.ip.clone().or(ip.hostname.clone()).unwrap_or_default()).collect::<Vec<_>>().join(",")
    ).unwrap_or_default();

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: lbs,
        images: vec![],
    }
}

// Network Policies
fn map_network_policy_to_summary(n: NetworkPolicy) -> WorkloadSummary {
    let meta = n.metadata;
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![],
    }
}

// PVC
fn map_pvc_to_summary(p: PersistentVolumeClaim) -> WorkloadSummary {
    let meta = p.metadata;
    let status = p.status.unwrap_or_default();
    let phase = status.phase.unwrap_or_default();
    let capacity = status.capacity.and_then(|c| c.get("storage").map(|q| q.0.clone())).unwrap_or_default();

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} ({})", phase, capacity),
        images: vec![],
    }
}

// PV (Cluster Scoped)
fn map_pv_to_summary(p: PersistentVolume) -> WorkloadSummary {
    let meta = p.metadata;
    let status = p.status.unwrap_or_default();
    let phase = status.phase.unwrap_or_default();
    let spec = p.spec.unwrap_or_default();
    let capacity = spec.capacity.and_then(|c| c.get("storage").map(|q| q.0.clone())).unwrap_or_default();

    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: "-".to_string(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: format!("{} ({})", phase, capacity),
        images: vec![],
    }
}

// Storage Classes (Cluster Scoped)
fn map_storage_class_to_summary(s: StorageClass) -> WorkloadSummary {
    let meta = s.metadata;
    let provisioner = s.provisioner;
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: "-".to_string(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![provisioner],
    }
}

// Service Accounts
fn map_service_account_to_summary(s: ServiceAccount) -> WorkloadSummary {
    let meta = s.metadata;
    
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![],
    }
}

// Roles
fn map_role_to_summary(r: Role) -> WorkloadSummary {
    let meta = r.metadata;
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: meta.namespace.clone().unwrap_or_default(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![],
    }
}

// Cluster Roles (Cluster Scoped)
fn map_cluster_role_to_summary(r: ClusterRole) -> WorkloadSummary {
    let meta = r.metadata;
    WorkloadSummary {
        id: meta.uid.clone().unwrap_or_default(),
        name: meta.name.clone().unwrap_or_default(),
        namespace: "-".to_string(),
        age: calculate_age(meta.creation_timestamp.as_ref()),
        created_at: get_created_at(meta.creation_timestamp.as_ref()),
        labels: meta.labels.unwrap_or_default(),
        status: "Active".to_string(),
        images: vec![],
    }
}

impl_workload_commands!(ConfigMap, cluster_list_config_maps, cluster_delete_config_map, map_configmap_to_summary);
impl_workload_commands!(Secret, cluster_list_secrets, cluster_delete_secret, map_secret_to_summary);
impl_workload_commands!(ResourceQuota, cluster_list_resource_quotas, cluster_delete_resource_quota, map_resource_quota_to_summary);
impl_workload_commands!(LimitRange, cluster_list_limit_ranges, cluster_delete_limit_range, map_limit_range_to_summary);
impl_workload_commands!(HorizontalPodAutoscaler, cluster_list_hpa, cluster_delete_hpa, map_hpa_to_summary);
impl_workload_commands!(PodDisruptionBudget, cluster_list_pdb, cluster_delete_pdb, map_pdb_to_summary);
impl_workload_commands!(Service, cluster_list_services, cluster_delete_service, map_service_to_summary);
impl_workload_commands!(Endpoints, cluster_list_endpoints, cluster_delete_endpoint, map_endpoints_to_summary);
impl_workload_commands!(Ingress, cluster_list_ingresses, cluster_delete_ingress, map_ingress_to_summary);
impl_workload_commands!(NetworkPolicy, cluster_list_network_policies, cluster_delete_network_policy, map_network_policy_to_summary);
impl_workload_commands!(PersistentVolumeClaim, cluster_list_pvc, cluster_delete_pvc, map_pvc_to_summary);
impl_workload_commands!(ServiceAccount, cluster_list_service_accounts, cluster_delete_service_account, map_service_account_to_summary);
impl_workload_commands!(Role, cluster_list_roles, cluster_delete_role, map_role_to_summary);

// Cluster Scoped
impl_cluster_resource_commands!(PersistentVolume, cluster_list_pv, cluster_delete_pv, map_pv_to_summary);
impl_cluster_resource_commands!(StorageClass, cluster_list_storage_classes, cluster_delete_storage_class, map_storage_class_to_summary);
impl_cluster_resource_commands!(ClusterRole, cluster_list_cluster_roles, cluster_delete_cluster_role, map_cluster_role_to_summary);
