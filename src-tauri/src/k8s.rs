// ... imports ...
use crate::config;
use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::config::Kubeconfig;
use kube::runtime::watcher;
use kube::{Api, Client, Config};
use std::path::PathBuf;
use tauri::{Emitter, Window};

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
