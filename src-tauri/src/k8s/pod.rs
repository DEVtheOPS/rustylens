use crate::cluster_manager::ClusterManagerState;
use crate::k8s::client::{create_client_for_cluster, create_client_for_context};
use crate::k8s::watcher::WatcherState;
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{DeleteParams, ListParams, LogParams};
use kube::runtime::watcher;
use kube::Api;
use tauri::{Emitter, State, Window};

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

    let summaries = list
        .items
        .iter()
        .map(|p| map_pod_to_summary(p.clone()))
        .collect();
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
            let event_type = event
                .type_
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
                .clone();
            let reason = event
                .reason
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
                .clone();
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
