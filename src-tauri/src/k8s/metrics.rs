use crate::cluster_manager::ClusterManagerState;
use crate::k8s::client::create_client_for_cluster;
use k8s_openapi::api::core::v1::{Event, Node, Pod};
use kube::api::Api;
use tauri::State;

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
    if let Some(val) = q.strip_suffix("Ki") {
        val.parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if let Some(val) = q.strip_suffix("Mi") {
        val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(2)
    } else if let Some(val) = q.strip_suffix("Gi") {
        val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(3)
    } else if let Some(val) = q.strip_suffix("Ti") {
        val.parse::<f64>().unwrap_or(0.0) * 1024.0f64.powi(4)
    } else if let Some(val) = q.strip_suffix("m") {
        val.parse::<f64>().unwrap_or(0.0) / 1000.0
    } else {
        q.parse::<f64>().unwrap_or(0.0)
    }
}

#[tauri::command]
pub async fn cluster_get_metrics(
    cluster_id: String,
    state: State<'_, ClusterManagerState>,
) -> Result<ClusterMetrics, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    let nodes: Api<Node> = Api::all(client.clone());
    let pods: Api<Pod> = Api::all(client.clone());

    let node_list = nodes
        .list(&Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let pod_list = pods
        .list(&Default::default())
        .await
        .map_err(|e| e.to_string())?;

    let mut metrics = ClusterMetrics::default();

    // Node Capacity & Allocatable
    for node in node_list.items {
        if let Some(status) = node.status {
            if let Some(cap) = status.capacity {
                if let Some(cpu) = cap.get("cpu") {
                    metrics.cpu.capacity += parse_cpu(&cpu.0);
                }
                if let Some(mem) = cap.get("memory") {
                    metrics.memory.capacity += parse_memory(&mem.0);
                }
                if let Some(p) = cap.get("pods") {
                    metrics.pods.capacity += parse_cpu(&p.0);
                }
            }
            if let Some(alloc) = status.allocatable {
                if let Some(cpu) = alloc.get("cpu") {
                    metrics.cpu.allocatable += parse_cpu(&cpu.0);
                }
                if let Some(mem) = alloc.get("memory") {
                    metrics.memory.allocatable += parse_memory(&mem.0);
                }
                if let Some(p) = alloc.get("pods") {
                    metrics.pods.allocatable += parse_cpu(&p.0);
                }
            }
        }
    }

    // Pod Requests & Limits
    for pod in pod_list.items {
        // Skip finished pods
        if let Some(status) = &pod.status {
            if let Some(phase) = &status.phase {
                if phase == "Succeeded" || phase == "Failed" {
                    continue;
                }
            }
        }

        metrics.pods.usage += 1.0;

        if let Some(spec) = pod.spec {
            for container in spec.containers {
                if let Some(reqs) = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.requests.as_ref())
                {
                    if let Some(cpu) = reqs.get("cpu") {
                        metrics.cpu.requests += parse_cpu(&cpu.0);
                    }
                    if let Some(mem) = reqs.get("memory") {
                        metrics.memory.requests += parse_memory(&mem.0);
                    }
                }
                if let Some(lims) = container.resources.as_ref().and_then(|r| r.limits.as_ref()) {
                    if let Some(cpu) = lims.get("cpu") {
                        metrics.cpu.limits += parse_cpu(&cpu.0);
                    }
                    if let Some(mem) = lims.get("memory") {
                        metrics.memory.limits += parse_memory(&mem.0);
                    }
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
                let last_ts_parsed = chrono::DateTime::parse_from_rfc3339(&last_ts_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc);
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
                object: format!(
                    "{}/{}",
                    e.involved_object.kind.unwrap_or_default(),
                    e.involved_object.name.unwrap_or_default()
                ),
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helper function tests ---

    #[test]
    fn test_parse_cpu_millicores() {
        assert_eq!(parse_cpu("100m"), 0.1);
        assert_eq!(parse_cpu("500m"), 0.5);
        assert_eq!(parse_cpu("1000m"), 1.0);
    }

    #[test]
    fn test_parse_cpu_cores() {
        assert_eq!(parse_cpu("1"), 1.0);
        assert_eq!(parse_cpu("2"), 2.0);
        assert_eq!(parse_cpu("0.5"), 0.5);
    }

    #[test]
    fn test_parse_cpu_invalid() {
        assert_eq!(parse_cpu("invalid"), 0.0);
        assert_eq!(parse_cpu(""), 0.0);
    }

    #[test]
    fn test_parse_memory_ki() {
        let result = parse_memory("1024Ki");
        assert_eq!(result, 1024.0 * 1024.0);
    }

    #[test]
    fn test_parse_memory_mi() {
        let result = parse_memory("256Mi");
        assert_eq!(result, 256.0 * 1024.0 * 1024.0);
    }

    #[test]
    fn test_parse_memory_gi() {
        let result = parse_memory("2Gi");
        assert_eq!(result, 2.0 * 1024.0 * 1024.0 * 1024.0);
    }

    #[test]
    fn test_parse_memory_ti() {
        let result = parse_memory("1Ti");
        assert_eq!(result, 1024.0_f64.powi(4));
    }

    #[test]
    fn test_parse_memory_bytes() {
        let result = parse_memory("1000000");
        assert_eq!(result, 1000000.0);
    }

    #[test]
    fn test_parse_memory_invalid() {
        assert_eq!(parse_memory("invalid"), 0.0);
        assert_eq!(parse_memory(""), 0.0);
    }

    #[test]
    fn test_parse_memory_millibytes() {
        // Edge case: memory in millibytes (rare but valid)
        let result = parse_memory("1000m");
        assert_eq!(result, 1.0);
    }
}
