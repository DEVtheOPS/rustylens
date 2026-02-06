use crate::cluster_manager::ClusterManagerState;
use crate::k8s::client::create_client_for_cluster;
use crate::k8s::common::{calculate_age, K8sEventInfo};
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::{Event, Pod};
use kube::api::{Api, ListParams};
use std::collections::HashMap;
use tauri::State;

/// Detailed information about a Kubernetes StatefulSet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatefulSetDetails {
    pub name: String,
    pub namespace: String,
    pub uid: String,
    pub created_at: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub replicas_desired: i32,
    pub replicas_current: i32,
    pub replicas_ready: i32,
    pub replicas_updated: i32,
    pub replicas_available: i32,
    pub update_strategy_type: String,
    pub pod_management_policy: String,
    pub service_name: String,
    pub selector: HashMap<String, String>,
    pub conditions: Vec<StatefulSetCondition>,
    pub images: Vec<String>,
}

/// Condition of a Kubernetes StatefulSet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatefulSetCondition {
    pub condition_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub last_transition_time: Option<String>,
}

/// Get detailed information about a specific statefulset
#[tauri::command]
pub async fn cluster_get_statefulset_details(
    cluster_id: String,
    namespace: String,
    name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<StatefulSetDetails, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let statefulsets: Api<StatefulSet> = Api::namespaced(client, &namespace);

    let statefulset = statefulsets
        .get(&name)
        .await
        .map_err(|e| format!("Failed to get statefulset '{}': {}", name, e))?;

    let meta = statefulset.metadata;
    let spec = statefulset.spec.unwrap_or_default();
    let status = statefulset.status.unwrap_or_default();

    // Extract labels and annotations as HashMap
    let labels: HashMap<String, String> = meta.labels.unwrap_or_default().into_iter().collect();

    let annotations: HashMap<String, String> =
        meta.annotations.unwrap_or_default().into_iter().collect();

    // Extract selector
    let selector: HashMap<String, String> = spec
        .selector
        .match_labels
        .unwrap_or_default()
        .into_iter()
        .collect();

    // Extract conditions
    let conditions: Vec<StatefulSetCondition> = status
        .conditions
        .unwrap_or_default()
        .into_iter()
        .map(|c| StatefulSetCondition {
            condition_type: c.type_,
            status: c.status,
            reason: c.reason,
            message: c.message,
            last_transition_time: c.last_transition_time.map(|t| t.0.to_string()),
        })
        .collect();

    // Extract images from pod template
    let images: Vec<String> = spec
        .template
        .spec
        .map(|pod_spec| {
            pod_spec
                .containers
                .into_iter()
                .filter_map(|c| c.image)
                .collect()
        })
        .unwrap_or_default();

    // Extract update strategy type
    let update_strategy_type = spec
        .update_strategy
        .and_then(|s| s.type_)
        .unwrap_or_else(|| "RollingUpdate".to_string());

    // Extract pod management policy
    let pod_management_policy = spec
        .pod_management_policy
        .unwrap_or_else(|| "OrderedReady".to_string());

    // Extract service name
    let service_name = spec.service_name;

    // Extract created_at timestamp
    let created_at = meta
        .creation_timestamp
        .map(|t| t.0.to_string())
        .unwrap_or_default();

    Ok(StatefulSetDetails {
        name: meta.name.unwrap_or_default(),
        namespace: meta.namespace.unwrap_or_default(),
        uid: meta.uid.unwrap_or_default(),
        created_at,
        labels,
        annotations,
        replicas_desired: spec.replicas.unwrap_or(1),
        replicas_current: status.current_replicas.unwrap_or(0),
        replicas_ready: status.ready_replicas.unwrap_or(0),
        replicas_updated: status.updated_replicas.unwrap_or(0),
        replicas_available: status.available_replicas.unwrap_or(0),
        update_strategy_type,
        pod_management_policy,
        service_name: service_name.unwrap_or_default(),
        selector,
        conditions,
        images,
    })
}

// --- StatefulSet Pods ---

/// Information about a pod belonging to a statefulset
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatefulSetPodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub age: String,
    pub ready: String,
    pub restarts: i32,
    pub node: String,
    pub pod_ip: String,
}

/// Helper function to format age from creation timestamp
fn format_age_from_timestamp(
    creation_timestamp: &Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,
) -> String {
    calculate_age(creation_timestamp.as_ref())
}

/// Helper function to map a Pod to StatefulSetPodInfo
pub fn map_pod_to_statefulset_pod_info(pod: &Pod) -> StatefulSetPodInfo {
    let meta = &pod.metadata;
    let spec = &pod.spec;
    let status = &pod.status;

    let name = meta.name.clone().unwrap_or_default();
    let namespace = meta.namespace.clone().unwrap_or_default();

    // Determine overall pod status
    let pod_status = status
        .as_ref()
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Format age
    let age = format_age_from_timestamp(&meta.creation_timestamp);

    // Calculate ready status (e.g., "2/2")
    let container_statuses = status.as_ref().and_then(|s| s.container_statuses.as_ref());
    let ready_count = container_statuses
        .map(|cs| cs.iter().filter(|c| c.ready).count())
        .unwrap_or(0);
    let total_count = container_statuses.map(|cs| cs.len()).unwrap_or(0);
    let ready = format!("{}/{}", ready_count, total_count);

    // Sum restarts from all containers
    let restarts: i32 = container_statuses
        .map(|cs| cs.iter().map(|c| c.restart_count).sum())
        .unwrap_or(0);

    // Get node name
    let node = spec
        .as_ref()
        .and_then(|s| s.node_name.clone())
        .unwrap_or_else(|| "-".to_string());

    // Get pod IP
    let pod_ip = status
        .as_ref()
        .and_then(|s| s.pod_ip.clone())
        .unwrap_or_else(|| "-".to_string());

    StatefulSetPodInfo {
        name,
        namespace,
        status: pod_status,
        age,
        ready,
        restarts,
        node,
        pod_ip,
    }
}

/// Get all pods matching a statefulset's selector labels
#[tauri::command]
pub async fn cluster_get_statefulset_pods(
    cluster_id: String,
    namespace: String,
    statefulset_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<StatefulSetPodInfo>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    // First, get the statefulset to retrieve its selector labels
    let statefulsets_api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
    let statefulset = statefulsets_api
        .get(&statefulset_name)
        .await
        .map_err(|e| format!("Failed to get statefulset '{}': {}", statefulset_name, e))?;

    // Extract selector labels from statefulset spec
    let selector_labels = statefulset
        .spec
        .as_ref()
        .and_then(|s| s.selector.match_labels.clone())
        .unwrap_or_default();

    if selector_labels.is_empty() {
        return Ok(vec![]);
    }

    // Build label selector string (e.g., "app=nginx,env=prod")
    let label_selector: String = selector_labels
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",");

    // List pods matching the label selector
    let pods_api: Api<Pod> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().labels(&label_selector);

    let pods = pods_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list pods: {}", e))?;

    // Map to StatefulSetPodInfo
    let pod_infos: Vec<StatefulSetPodInfo> = pods
        .items
        .iter()
        .map(map_pod_to_statefulset_pod_info)
        .collect();

    Ok(pod_infos)
}

// --- StatefulSet Events ---

/// Helper function to filter events specific to a statefulset
fn filter_statefulset_events(
    events: Vec<Event>,
    statefulset_name: &str,
    statefulset_uid: Option<&str>,
) -> Vec<K8sEventInfo> {
    let mut event_infos: Vec<K8sEventInfo> = events
        .into_iter()
        .filter(|event| {
            let involved_obj = &event.involved_object;

            // Match by name
            let name_matches = involved_obj
                .name
                .as_ref()
                .map(|n| n == statefulset_name)
                .unwrap_or(false);

            // Match by kind (StatefulSet)
            let kind_matches = involved_obj
                .kind
                .as_ref()
                .map(|k| k == "StatefulSet")
                .unwrap_or(false);

            // Match by UID if available
            let uid_matches = if let Some(uid) = statefulset_uid {
                involved_obj.uid.as_ref().map(|u| u == uid).unwrap_or(true)
            } else {
                true
            };

            name_matches && kind_matches && uid_matches
        })
        .map(|event| {
            let source = event
                .source
                .as_ref()
                .and_then(|s| s.component.clone())
                .unwrap_or_else(|| "unknown".to_string());

            K8sEventInfo {
                event_type: event.type_.unwrap_or_else(|| "Normal".to_string()),
                reason: event.reason.unwrap_or_default(),
                message: event.message.unwrap_or_default(),
                count: event.count.unwrap_or(1),
                first_timestamp: event.first_timestamp.as_ref().map(|t| t.0.to_string()),
                last_timestamp: event.last_timestamp.as_ref().map(|t| t.0.to_string()),
                source,
            }
        })
        .collect();

    // Sort by last_timestamp descending (most recent first)
    event_infos.sort_by(|a, b| b.last_timestamp.cmp(&a.last_timestamp));

    event_infos
}

/// Fetches events related to a specific statefulset
#[tauri::command]
pub async fn cluster_get_statefulset_events(
    cluster_id: String,
    namespace: String,
    statefulset_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<K8sEventInfo>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    // First, get the statefulset to retrieve its UID
    let statefulsets_api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
    let statefulset = statefulsets_api
        .get(&statefulset_name)
        .await
        .map_err(|e| format!("Failed to get statefulset '{}': {}", statefulset_name, e))?;

    let statefulset_uid = statefulset.metadata.uid.as_deref();

    // List all events in the namespace
    let events_api: Api<Event> = Api::namespaced(client, &namespace);
    let lp = ListParams::default();

    let events_list = events_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    // Filter events for this statefulset
    let event_infos =
        filter_statefulset_events(events_list.items, &statefulset_name, statefulset_uid);

    Ok(event_infos)
}
