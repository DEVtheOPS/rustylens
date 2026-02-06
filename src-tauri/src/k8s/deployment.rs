use crate::cluster_manager::ClusterManagerState;
use crate::k8s::client::create_client_for_cluster;
use crate::k8s::common::{calculate_age, K8sEventInfo};
use k8s_openapi::api::apps::v1::{Deployment, ReplicaSet};
use k8s_openapi::api::core::v1::{Event, Pod};
use kube::api::{Api, ListParams};
use std::collections::HashMap;
use tauri::State;

/// Detailed information about a Kubernetes Deployment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeploymentDetails {
    pub name: String,
    pub namespace: String,
    pub uid: String,
    pub created_at: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub replicas_desired: i32,
    pub replicas_updated: i32,
    pub replicas_total: i32,
    pub replicas_available: i32,
    pub replicas_unavailable: i32,
    pub strategy_type: String,
    pub selector: HashMap<String, String>,
    pub conditions: Vec<DeploymentCondition>,
    pub images: Vec<String>,
}

/// Condition of a Kubernetes Deployment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeploymentCondition {
    pub condition_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub last_transition_time: Option<String>,
}

/// Get detailed information about a specific deployment
#[tauri::command]
pub async fn cluster_get_deployment_details(
    cluster_id: String,
    namespace: String,
    name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<DeploymentDetails, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;
    let deployments: Api<Deployment> = Api::namespaced(client, &namespace);

    let deployment = deployments
        .get(&name)
        .await
        .map_err(|e| format!("Failed to get deployment '{}': {}", name, e))?;

    let meta = deployment.metadata;
    let spec = deployment.spec.unwrap_or_default();
    let status = deployment.status.unwrap_or_default();

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
    let conditions: Vec<DeploymentCondition> = status
        .conditions
        .unwrap_or_default()
        .into_iter()
        .map(|c| DeploymentCondition {
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

    // Extract strategy type
    let strategy_type = spec
        .strategy
        .and_then(|s| s.type_)
        .unwrap_or_else(|| "RollingUpdate".to_string());

    // Extract created_at timestamp
    let created_at = meta
        .creation_timestamp
        .map(|t| t.0.to_string())
        .unwrap_or_default();

    Ok(DeploymentDetails {
        name: meta.name.unwrap_or_default(),
        namespace: meta.namespace.unwrap_or_default(),
        uid: meta.uid.unwrap_or_default(),
        created_at,
        labels,
        annotations,
        replicas_desired: spec.replicas.unwrap_or(1),
        replicas_updated: status.updated_replicas.unwrap_or(0),
        replicas_total: status.replicas.unwrap_or(0),
        replicas_available: status.available_replicas.unwrap_or(0),
        replicas_unavailable: status.unavailable_replicas.unwrap_or(0),
        strategy_type,
        selector,
        conditions,
        images,
    })
}

// --- Deployment Pods ---

/// Information about a pod belonging to a deployment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeploymentPodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub age: String,
    pub ready: String,
    pub restarts: i32,
    pub node: String,
    pub pod_ip: String,
}

/// Helper function to map a Pod to DeploymentPodInfo
pub fn map_pod_to_deployment_pod_info(pod: &Pod) -> DeploymentPodInfo {
    let meta = &pod.metadata;
    let spec = pod.spec.as_ref();
    let status = pod.status.as_ref();

    let name = meta.name.clone().unwrap_or_default();
    let namespace = meta.namespace.clone().unwrap_or_default();

    // Get pod phase/status
    let pod_status = status
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Calculate age
    let age = calculate_age(meta.creation_timestamp.as_ref());

    // Calculate ready containers (e.g., "1/2")
    let container_statuses = status.and_then(|s| s.container_statuses.as_ref());
    let total_containers = container_statuses.map(|cs| cs.len()).unwrap_or(0);
    let ready_containers = container_statuses
        .map(|cs| cs.iter().filter(|c| c.ready).count())
        .unwrap_or(0);
    let ready = format!("{}/{}", ready_containers, total_containers);

    // Sum restarts from all containers
    let restarts: i32 = container_statuses
        .map(|cs| cs.iter().map(|c| c.restart_count).sum())
        .unwrap_or(0);

    // Get node name
    let node = spec
        .and_then(|s| s.node_name.clone())
        .unwrap_or_else(|| "-".to_string());

    // Get pod IP
    let pod_ip = status
        .and_then(|s| s.pod_ip.clone())
        .unwrap_or_else(|| "-".to_string());

    DeploymentPodInfo {
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

/// Get all pods matching a deployment's selector labels
#[tauri::command]
pub async fn cluster_get_deployment_pods(
    cluster_id: String,
    namespace: String,
    deployment_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<DeploymentPodInfo>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    // First, get the deployment to retrieve its selector labels
    let deployments_api: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let deployment = deployments_api
        .get(&deployment_name)
        .await
        .map_err(|e| format!("Failed to get deployment '{}': {}", deployment_name, e))?;

    // Extract selector labels from deployment spec
    let selector_labels = deployment
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

    // List pods with the label selector
    let pods_api: Api<Pod> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().labels(&label_selector);

    let pods_list = pods_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list pods: {}", e))?;

    // Map pods to DeploymentPodInfo
    let pod_infos: Vec<DeploymentPodInfo> = pods_list
        .items
        .iter()
        .map(map_pod_to_deployment_pod_info)
        .collect();

    Ok(pod_infos)
}

// --- Deployment ReplicaSets ---

/// Information about a ReplicaSet owned by a Deployment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ReplicaSetInfo {
    pub name: String,
    pub namespace: String,
    pub revision: String,
    pub desired: i32,
    pub current: i32,
    pub ready: i32,
    pub age: String,
    pub images: Vec<String>,
    pub created_at: String,
}

/// Extract revision number from ReplicaSet annotations
fn extract_revision(rs: &ReplicaSet) -> String {
    rs.metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("deployment.kubernetes.io/revision"))
        .cloned()
        .unwrap_or_else(|| "0".to_string())
}

/// Map a ReplicaSet to ReplicaSetInfo
fn map_replicaset_to_info(rs: &ReplicaSet) -> ReplicaSetInfo {
    let meta = &rs.metadata;
    let spec = rs.spec.as_ref();
    let status = rs.status.as_ref();

    let name = meta.name.clone().unwrap_or_default();
    let namespace = meta.namespace.clone().unwrap_or_default();
    let revision = extract_revision(rs);

    let desired = spec.and_then(|s| s.replicas).unwrap_or(0);
    let current = status.map(|s| s.replicas).unwrap_or(0);
    let ready = status.and_then(|s| s.ready_replicas).unwrap_or(0);

    let age = calculate_age(meta.creation_timestamp.as_ref());
    let created_at = meta
        .creation_timestamp
        .as_ref()
        .map(|t| t.0.to_string())
        .unwrap_or_default();

    let images = spec
        .and_then(|s| s.template.as_ref())
        .and_then(|t| t.spec.as_ref())
        .map(|pod_spec| {
            pod_spec
                .containers
                .iter()
                .filter_map(|c| c.image.clone())
                .collect()
        })
        .unwrap_or_default();

    ReplicaSetInfo {
        name,
        namespace,
        revision,
        desired,
        current,
        ready,
        age,
        images,
        created_at,
    }
}

/// Fetches ReplicaSets (revision history) for a specific deployment
#[tauri::command]
pub async fn cluster_get_deployment_replicasets(
    cluster_id: String,
    namespace: String,
    deployment_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<ReplicaSetInfo>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    // 1. Get the deployment to find its UID
    let deployments_api: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let deployment = deployments_api
        .get(&deployment_name)
        .await
        .map_err(|e| format!("Failed to get deployment '{}': {}", deployment_name, e))?;

    let deployment_uid = deployment
        .metadata
        .uid
        .ok_or_else(|| "Deployment has no UID".to_string())?;

    // 2. List all ReplicaSets in the namespace
    let replicasets_api: Api<ReplicaSet> = Api::namespaced(client, &namespace);
    let rs_list = replicasets_api
        .list(&ListParams::default())
        .await
        .map_err(|e| format!("Failed to list replicasets: {}", e))?;

    // 3. Filter by owner reference matching deployment and map to info
    let mut rs_infos: Vec<ReplicaSetInfo> = rs_list
        .items
        .iter()
        .filter(|rs| {
            rs.metadata
                .owner_references
                .as_ref()
                .map(|refs| {
                    refs.iter()
                        .any(|owner| owner.kind == "Deployment" && owner.uid == deployment_uid)
                })
                .unwrap_or(false)
        })
        .map(map_replicaset_to_info)
        .collect();

    // 4. Sort by revision (newest first)
    rs_infos.sort_by(|a, b| {
        let rev_a: i64 = a.revision.parse().unwrap_or(0);
        let rev_b: i64 = b.revision.parse().unwrap_or(0);
        rev_b.cmp(&rev_a)
    });

    Ok(rs_infos)
}

// --- Deployment Events ---

/// Helper function to filter and map events for a specific deployment
pub fn filter_deployment_events(
    events: Vec<Event>,
    deployment_name: &str,
    deployment_uid: Option<&str>,
) -> Vec<K8sEventInfo> {
    let mut event_infos: Vec<K8sEventInfo> = events
        .into_iter()
        .filter(|event| {
            let involved = &event.involved_object;
            let name_matches = involved.name.as_deref() == Some(deployment_name);
            let kind_matches = involved.kind.as_deref() == Some("Deployment");
            let uid_matches = deployment_uid
                .map(|uid| involved.uid.as_deref() == Some(uid))
                .unwrap_or(true);

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

/// Fetches events related to a specific deployment
#[tauri::command]
pub async fn cluster_get_deployment_events(
    cluster_id: String,
    namespace: String,
    deployment_name: String,
    state: State<'_, ClusterManagerState>,
) -> Result<Vec<K8sEventInfo>, String> {
    let client = create_client_for_cluster(&cluster_id, &state).await?;

    // First, get the deployment to retrieve its UID
    let deployments_api: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let deployment = deployments_api
        .get(&deployment_name)
        .await
        .map_err(|e| format!("Failed to get deployment '{}': {}", deployment_name, e))?;

    let deployment_uid = deployment.metadata.uid.as_deref();

    // List all events in the namespace
    let events_api: Api<Event> = Api::namespaced(client, &namespace);
    let lp = ListParams::default();

    let events_list = events_api
        .list(&lp)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    // Filter events for this deployment
    let event_infos = filter_deployment_events(events_list.items, &deployment_name, deployment_uid);

    Ok(event_infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- K8sEventInfo and filter_deployment_events tests ---

    // Helper to create a mock Event for testing
    fn create_mock_event(
        name: &str,
        kind: &str,
        uid: Option<&str>,
        event_type: &str,
        reason: &str,
        message: &str,
        count: i32,
        last_timestamp: Option<&str>,
    ) -> Event {
        use k8s_openapi::api::core::v1::{EventSource, ObjectReference};
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;

        Event {
            metadata: Default::default(),
            involved_object: ObjectReference {
                name: Some(name.to_string()),
                kind: Some(kind.to_string()),
                uid: uid.map(|s| s.to_string()),
                ..Default::default()
            },
            type_: Some(event_type.to_string()),
            reason: Some(reason.to_string()),
            message: Some(message.to_string()),
            count: Some(count),
            first_timestamp: last_timestamp
                .map(|ts| Time(ts.parse::<k8s_openapi::jiff::Timestamp>().unwrap())),
            last_timestamp: last_timestamp
                .map(|ts| Time(ts.parse::<k8s_openapi::jiff::Timestamp>().unwrap())),
            source: Some(EventSource {
                component: Some("deployment-controller".to_string()),
                host: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_k8s_event_info_struct_fields() {
        let event_info = K8sEventInfo {
            event_type: "Warning".to_string(),
            reason: "FailedCreate".to_string(),
            message: "Error creating pods".to_string(),
            count: 3,
            first_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            last_timestamp: Some("2024-01-01T01:00:00Z".to_string()),
            source: "deployment-controller".to_string(),
        };

        assert_eq!(event_info.event_type, "Warning");
        assert_eq!(event_info.reason, "FailedCreate");
        assert_eq!(event_info.message, "Error creating pods");
        assert_eq!(event_info.count, 3);
        assert_eq!(
            event_info.first_timestamp,
            Some("2024-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            event_info.last_timestamp,
            Some("2024-01-01T01:00:00Z".to_string())
        );
        assert_eq!(event_info.source, "deployment-controller");
    }

    #[test]
    fn test_filter_deployment_events_with_multiple_events() {
        let events = vec![
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "ScalingReplicaSet",
                "Scaled up replica set my-deployment-abc to 3",
                1,
                Some("2024-01-01T02:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Warning",
                "FailedCreate",
                "Error creating pods",
                2,
                Some("2024-01-01T01:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "ScalingReplicaSet",
                "Scaled down replica set my-deployment-xyz to 0",
                1,
                Some("2024-01-01T03:00:00Z"),
            ),
        ];

        let result = filter_deployment_events(events, "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 3);
        // Should be sorted by last_timestamp descending (newest first)
        assert_eq!(result[0].reason, "ScalingReplicaSet");
        assert!(result[0].message.contains("Scaled down"));
        assert_eq!(result[1].reason, "ScalingReplicaSet");
        assert!(result[1].message.contains("Scaled up"));
        assert_eq!(result[2].reason, "FailedCreate");
    }

    #[test]
    fn test_filter_deployment_events_with_no_events() {
        let events: Vec<Event> = vec![];

        let result = filter_deployment_events(events, "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_deployment_events_filters_by_involved_object() {
        let events = vec![
            // Event for our deployment
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "ScalingReplicaSet",
                "Scaled up",
                1,
                Some("2024-01-01T01:00:00Z"),
            ),
            // Event for a different deployment (should be filtered out)
            create_mock_event(
                "other-deployment",
                "Deployment",
                Some("uid-456"),
                "Normal",
                "ScalingReplicaSet",
                "Other scaled up",
                1,
                Some("2024-01-01T02:00:00Z"),
            ),
            // Event for a Pod (should be filtered out)
            create_mock_event(
                "my-deployment-pod-abc",
                "Pod",
                Some("uid-789"),
                "Normal",
                "Scheduled",
                "Successfully assigned",
                1,
                Some("2024-01-01T03:00:00Z"),
            ),
            // Event for a ReplicaSet (should be filtered out)
            create_mock_event(
                "my-deployment-rs-abc",
                "ReplicaSet",
                Some("uid-101"),
                "Normal",
                "SuccessfulCreate",
                "Created pod",
                1,
                Some("2024-01-01T04:00:00Z"),
            ),
        ];

        let result = filter_deployment_events(events, "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].message, "Scaled up");
    }

    #[test]
    fn test_filter_deployment_events_handles_event_types() {
        let events = vec![
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "ScalingReplicaSet",
                "Scaled up",
                1,
                Some("2024-01-01T01:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Warning",
                "FailedCreate",
                "Error creating",
                5,
                Some("2024-01-01T02:00:00Z"),
            ),
        ];

        let result = filter_deployment_events(events, "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 2);

        // Find the Warning event
        let warning_event = result.iter().find(|e| e.event_type == "Warning").unwrap();
        assert_eq!(warning_event.reason, "FailedCreate");
        assert_eq!(warning_event.count, 5);

        // Find the Normal event
        let normal_event = result.iter().find(|e| e.event_type == "Normal").unwrap();
        assert_eq!(normal_event.reason, "ScalingReplicaSet");
        assert_eq!(normal_event.count, 1);
    }

    #[test]
    fn test_filter_deployment_events_timestamp_sorting() {
        let events = vec![
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "Event1",
                "First event",
                1,
                Some("2024-01-01T01:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "Event3",
                "Third event (newest)",
                1,
                Some("2024-01-01T03:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "Event2",
                "Second event",
                1,
                Some("2024-01-01T02:00:00Z"),
            ),
        ];

        let result = filter_deployment_events(events, "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 3);
        // Verify descending order (newest first)
        assert_eq!(result[0].reason, "Event3");
        assert_eq!(result[1].reason, "Event2");
        assert_eq!(result[2].reason, "Event1");
    }

    #[test]
    fn test_filter_deployment_events_without_uid_filter() {
        // When UID is not provided, should still filter by name and kind
        let events = vec![
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-123"),
                "Normal",
                "ScalingReplicaSet",
                "Event 1",
                1,
                Some("2024-01-01T01:00:00Z"),
            ),
            create_mock_event(
                "my-deployment",
                "Deployment",
                Some("uid-456"), // Different UID
                "Normal",
                "ScalingReplicaSet",
                "Event 2",
                1,
                Some("2024-01-01T02:00:00Z"),
            ),
        ];

        // When no UID filter is provided, both events should match
        let result = filter_deployment_events(events, "my-deployment", None);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_k8s_event_info_serialization() {
        let event_info = K8sEventInfo {
            event_type: "Warning".to_string(),
            reason: "FailedCreate".to_string(),
            message: "Error creating pods".to_string(),
            count: 3,
            first_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            last_timestamp: Some("2024-01-01T01:00:00Z".to_string()),
            source: "deployment-controller".to_string(),
        };

        let json = serde_json::to_string(&event_info).expect("Serialization should work");
        assert!(json.contains("\"event_type\":\"Warning\""));
        assert!(json.contains("\"reason\":\"FailedCreate\""));
        assert!(json.contains("\"count\":3"));
    }

    #[test]
    fn test_k8s_event_info_deserialization() {
        let json = r#"{
            "event_type": "Normal",
            "reason": "ScalingReplicaSet",
            "message": "Scaled up",
            "count": 1,
            "first_timestamp": "2024-01-01T00:00:00Z",
            "last_timestamp": "2024-01-01T01:00:00Z",
            "source": "deployment-controller"
        }"#;

        let event_info: K8sEventInfo =
            serde_json::from_str(json).expect("Deserialization should work");
        assert_eq!(event_info.event_type, "Normal");
        assert_eq!(event_info.reason, "ScalingReplicaSet");
        assert_eq!(event_info.count, 1);
    }

    #[test]
    fn test_filter_deployment_events_handles_missing_fields() {
        // Test with an event that has minimal fields set
        let event = Event {
            metadata: Default::default(),
            involved_object: k8s_openapi::api::core::v1::ObjectReference {
                name: Some("my-deployment".to_string()),
                kind: Some("Deployment".to_string()),
                uid: Some("uid-123".to_string()),
                ..Default::default()
            },
            type_: None,   // Missing type
            reason: None,  // Missing reason
            message: None, // Missing message
            count: None,   // Missing count
            first_timestamp: None,
            last_timestamp: None,
            source: None, // Missing source
            ..Default::default()
        };

        let result = filter_deployment_events(vec![event], "my-deployment", Some("uid-123"));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, "Normal"); // Default value
        assert_eq!(result[0].reason, ""); // Default empty
        assert_eq!(result[0].message, ""); // Default empty
        assert_eq!(result[0].count, 1); // Default 1
        assert_eq!(result[0].source, "unknown"); // Default unknown
        assert!(result[0].first_timestamp.is_none());
        assert!(result[0].last_timestamp.is_none());
    }

    // --- DeploymentDetails struct tests ---

    #[test]
    fn test_deployment_details_serialization() {
        let details = DeploymentDetails {
            name: "nginx-deployment".to_string(),
            namespace: "default".to_string(),
            uid: "abc-123-def-456".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: HashMap::from([
                ("app".to_string(), "nginx".to_string()),
                ("env".to_string(), "production".to_string()),
            ]),
            annotations: HashMap::from([(
                "kubectl.kubernetes.io/last-applied-configuration".to_string(),
                "{}".to_string(),
            )]),
            replicas_desired: 3,
            replicas_updated: 3,
            replicas_total: 3,
            replicas_available: 3,
            replicas_unavailable: 0,
            strategy_type: "RollingUpdate".to_string(),
            selector: HashMap::from([("app".to_string(), "nginx".to_string())]),
            conditions: vec![DeploymentCondition {
                condition_type: "Available".to_string(),
                status: "True".to_string(),
                reason: Some("MinimumReplicasAvailable".to_string()),
                message: Some("Deployment has minimum availability.".to_string()),
                last_transition_time: Some("2024-01-15T10:35:00Z".to_string()),
            }],
            images: vec!["nginx:1.19".to_string()],
        };

        // Test JSON serialization
        let json = serde_json::to_string(&details).expect("Should serialize to JSON");
        assert!(json.contains("nginx-deployment"));
        assert!(json.contains("default"));
        assert!(json.contains("abc-123-def-456"));

        // Test deserialization
        let deserialized: DeploymentDetails =
            serde_json::from_str(&json).expect("Should deserialize from JSON");
        assert_eq!(deserialized.name, "nginx-deployment");
        assert_eq!(deserialized.namespace, "default");
        assert_eq!(deserialized.replicas_desired, 3);
        assert_eq!(deserialized.replicas_available, 3);
    }

    #[test]
    fn test_deployment_details_with_empty_fields() {
        let details = DeploymentDetails {
            name: "minimal".to_string(),
            namespace: "default".to_string(),
            uid: "".to_string(),
            created_at: "".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            replicas_desired: 1,
            replicas_updated: 0,
            replicas_total: 0,
            replicas_available: 0,
            replicas_unavailable: 1,
            strategy_type: "".to_string(),
            selector: HashMap::new(),
            conditions: vec![],
            images: vec![],
        };

        let json = serde_json::to_string(&details).expect("Should serialize empty fields");
        let deserialized: DeploymentDetails =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.labels.len(), 0);
        assert_eq!(deserialized.conditions.len(), 0);
        assert_eq!(deserialized.images.len(), 0);
    }

    #[test]
    fn test_deployment_condition_serialization() {
        let condition = DeploymentCondition {
            condition_type: "Progressing".to_string(),
            status: "True".to_string(),
            reason: Some("NewReplicaSetAvailable".to_string()),
            message: Some("ReplicaSet has successfully progressed.".to_string()),
            last_transition_time: Some("2024-01-15T10:35:00Z".to_string()),
        };

        let json = serde_json::to_string(&condition).expect("Should serialize condition");
        assert!(json.contains("Progressing"));
        assert!(json.contains("NewReplicaSetAvailable"));

        let deserialized: DeploymentCondition =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.condition_type, "Progressing");
        assert_eq!(deserialized.status, "True");
        assert!(deserialized.reason.is_some());
    }

    #[test]
    fn test_deployment_condition_with_none_fields() {
        let condition = DeploymentCondition {
            condition_type: "Available".to_string(),
            status: "False".to_string(),
            reason: None,
            message: None,
            last_transition_time: None,
        };

        let json = serde_json::to_string(&condition).expect("Should serialize with None fields");
        let deserialized: DeploymentCondition =
            serde_json::from_str(&json).expect("Should deserialize");

        assert!(deserialized.reason.is_none());
        assert!(deserialized.message.is_none());
        assert!(deserialized.last_transition_time.is_none());
    }

    #[test]
    fn test_deployment_details_multiple_images() {
        let details = DeploymentDetails {
            name: "multi-container".to_string(),
            namespace: "default".to_string(),
            uid: "uid-123".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            replicas_desired: 2,
            replicas_updated: 2,
            replicas_total: 2,
            replicas_available: 2,
            replicas_unavailable: 0,
            strategy_type: "RollingUpdate".to_string(),
            selector: HashMap::new(),
            conditions: vec![],
            images: vec![
                "nginx:1.19".to_string(),
                "redis:6.0".to_string(),
                "fluent/fluentd:v1.12".to_string(),
            ],
        };

        assert_eq!(details.images.len(), 3);
        assert!(details.images.contains(&"nginx:1.19".to_string()));
        assert!(details.images.contains(&"redis:6.0".to_string()));
    }

    #[test]
    fn test_deployment_details_multiple_conditions() {
        let details = DeploymentDetails {
            name: "test-deploy".to_string(),
            namespace: "default".to_string(),
            uid: "uid-456".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            replicas_desired: 3,
            replicas_updated: 3,
            replicas_total: 3,
            replicas_available: 3,
            replicas_unavailable: 0,
            strategy_type: "RollingUpdate".to_string(),
            selector: HashMap::new(),
            conditions: vec![
                DeploymentCondition {
                    condition_type: "Available".to_string(),
                    status: "True".to_string(),
                    reason: Some("MinimumReplicasAvailable".to_string()),
                    message: None,
                    last_transition_time: None,
                },
                DeploymentCondition {
                    condition_type: "Progressing".to_string(),
                    status: "True".to_string(),
                    reason: Some("NewReplicaSetAvailable".to_string()),
                    message: None,
                    last_transition_time: None,
                },
            ],
            images: vec!["nginx:latest".to_string()],
        };

        assert_eq!(details.conditions.len(), 2);
        assert!(details
            .conditions
            .iter()
            .any(|c| c.condition_type == "Available"));
        assert!(details
            .conditions
            .iter()
            .any(|c| c.condition_type == "Progressing"));
    }

    #[test]
    fn test_deployment_details_recreate_strategy() {
        let details = DeploymentDetails {
            name: "recreate-deploy".to_string(),
            namespace: "staging".to_string(),
            uid: "uid-789".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            replicas_desired: 1,
            replicas_updated: 1,
            replicas_total: 1,
            replicas_available: 1,
            replicas_unavailable: 0,
            strategy_type: "Recreate".to_string(),
            selector: HashMap::new(),
            conditions: vec![],
            images: vec![],
        };

        assert_eq!(details.strategy_type, "Recreate");
    }

    #[test]
    fn test_deployment_details_replica_mismatch() {
        // Test a deployment in the middle of a rollout
        let details = DeploymentDetails {
            name: "rolling-deploy".to_string(),
            namespace: "default".to_string(),
            uid: "uid-abc".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            replicas_desired: 5,
            replicas_updated: 3,
            replicas_total: 6, // During rollout, may have more pods
            replicas_available: 4,
            replicas_unavailable: 2,
            strategy_type: "RollingUpdate".to_string(),
            selector: HashMap::new(),
            conditions: vec![],
            images: vec![],
        };

        // Verify the replica counts are preserved correctly
        assert_eq!(details.replicas_desired, 5);
        assert_eq!(details.replicas_updated, 3);
        assert_eq!(details.replicas_total, 6);
        assert_eq!(details.replicas_available, 4);
        assert_eq!(details.replicas_unavailable, 2);
    }

    #[test]
    fn test_deployment_details_labels_and_selector_match() {
        let labels = HashMap::from([
            ("app".to_string(), "myapp".to_string()),
            ("version".to_string(), "v2".to_string()),
            ("team".to_string(), "backend".to_string()),
        ]);

        let selector = HashMap::from([("app".to_string(), "myapp".to_string())]);

        let details = DeploymentDetails {
            name: "label-test".to_string(),
            namespace: "default".to_string(),
            uid: "uid-def".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: labels.clone(),
            annotations: HashMap::new(),
            replicas_desired: 1,
            replicas_updated: 1,
            replicas_total: 1,
            replicas_available: 1,
            replicas_unavailable: 0,
            strategy_type: "RollingUpdate".to_string(),
            selector: selector.clone(),
            conditions: vec![],
            images: vec![],
        };

        // Verify labels contain all entries
        assert_eq!(details.labels.len(), 3);
        assert_eq!(details.labels.get("app"), Some(&"myapp".to_string()));

        // Verify selector is a subset of labels
        assert_eq!(details.selector.len(), 1);
        assert!(details.labels.contains_key("app"));
    }

    // --- DeploymentPodInfo struct tests ---

    #[test]
    fn test_deployment_pod_info_serialization() {
        let pod_info = DeploymentPodInfo {
            name: "nginx-deployment-abc123".to_string(),
            namespace: "default".to_string(),
            status: "Running".to_string(),
            age: "5d".to_string(),
            ready: "1/1".to_string(),
            restarts: 0,
            node: "worker-node-1".to_string(),
            pod_ip: "10.244.0.5".to_string(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&pod_info).expect("Should serialize to JSON");
        assert!(json.contains("nginx-deployment-abc123"));
        assert!(json.contains("default"));
        assert!(json.contains("Running"));
        assert!(json.contains("10.244.0.5"));

        // Test deserialization
        let deserialized: DeploymentPodInfo =
            serde_json::from_str(&json).expect("Should deserialize from JSON");
        assert_eq!(deserialized.name, "nginx-deployment-abc123");
        assert_eq!(deserialized.namespace, "default");
        assert_eq!(deserialized.status, "Running");
        assert_eq!(deserialized.restarts, 0);
    }

    #[test]
    fn test_deployment_pod_info_with_restarts() {
        let pod_info = DeploymentPodInfo {
            name: "crashloop-pod-xyz789".to_string(),
            namespace: "production".to_string(),
            status: "CrashLoopBackOff".to_string(),
            age: "2h".to_string(),
            ready: "0/1".to_string(),
            restarts: 15,
            node: "worker-node-2".to_string(),
            pod_ip: "10.244.1.10".to_string(),
        };

        assert_eq!(pod_info.restarts, 15);
        assert_eq!(pod_info.ready, "0/1");
        assert_eq!(pod_info.status, "CrashLoopBackOff");
    }

    #[test]
    fn test_deployment_pod_info_pending_status() {
        let pod_info = DeploymentPodInfo {
            name: "pending-pod-def456".to_string(),
            namespace: "staging".to_string(),
            status: "Pending".to_string(),
            age: "30s".to_string(),
            ready: "0/2".to_string(),
            restarts: 0,
            node: "-".to_string(),
            pod_ip: "-".to_string(),
        };

        assert_eq!(pod_info.status, "Pending");
        assert_eq!(pod_info.node, "-");
        assert_eq!(pod_info.pod_ip, "-");
    }

    #[test]
    fn test_deployment_pod_info_multi_container() {
        let pod_info = DeploymentPodInfo {
            name: "multi-container-pod".to_string(),
            namespace: "default".to_string(),
            status: "Running".to_string(),
            age: "1d".to_string(),
            ready: "3/3".to_string(),
            restarts: 2,
            node: "worker-node-3".to_string(),
            pod_ip: "10.244.2.15".to_string(),
        };

        assert_eq!(pod_info.ready, "3/3");
        assert_eq!(pod_info.restarts, 2);
    }

    #[test]
    fn test_deployment_pod_info_empty_fields() {
        let pod_info = DeploymentPodInfo {
            name: "".to_string(),
            namespace: "".to_string(),
            status: "Unknown".to_string(),
            age: "-".to_string(),
            ready: "0/0".to_string(),
            restarts: 0,
            node: "-".to_string(),
            pod_ip: "-".to_string(),
        };

        let json = serde_json::to_string(&pod_info).expect("Should serialize empty fields");
        let deserialized: DeploymentPodInfo =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.name, "");
        assert_eq!(deserialized.status, "Unknown");
        assert_eq!(deserialized.ready, "0/0");
    }

    // --- map_pod_to_deployment_pod_info tests ---

    // Helper function to create a mock Pod for testing
    fn create_mock_pod(
        name: &str,
        namespace: &str,
        phase: &str,
        ready_containers: usize,
        total_containers: usize,
        restarts: i32,
        node: Option<&str>,
        pod_ip: Option<&str>,
    ) -> Pod {
        use k8s_openapi::api::core::v1::{ContainerStatus, PodSpec, PodStatus};
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

        let container_statuses: Vec<ContainerStatus> = (0..total_containers)
            .map(|i| ContainerStatus {
                name: format!("container-{}", i),
                ready: i < ready_containers,
                restart_count: if i == 0 { restarts } else { 0 },
                image: "nginx:latest".to_string(),
                image_id: "sha256:abc123".to_string(),
                state: None,
                last_state: None,
                container_id: Some(format!("docker://container-{}", i)),
                started: Some(true),
                allocated_resources: None,
                resources: None,
                volume_mounts: None,
                user: None,
                allocated_resources_status: None,
            })
            .collect();

        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                creation_timestamp: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::Time(
                    k8s_openapi::jiff::Timestamp::now(),
                )),
                ..Default::default()
            },
            spec: Some(PodSpec {
                node_name: node.map(|n| n.to_string()),
                containers: vec![],
                ..Default::default()
            }),
            status: Some(PodStatus {
                phase: Some(phase.to_string()),
                pod_ip: pod_ip.map(|ip| ip.to_string()),
                container_statuses: if total_containers > 0 {
                    Some(container_statuses)
                } else {
                    None
                },
                ..Default::default()
            }),
        }
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_running() {
        let pod = create_mock_pod(
            "nginx-abc123",
            "default",
            "Running",
            1,
            1,
            0,
            Some("worker-1"),
            Some("10.244.0.5"),
        );

        let info = map_pod_to_deployment_pod_info(&pod);

        assert_eq!(info.name, "nginx-abc123");
        assert_eq!(info.namespace, "default");
        assert_eq!(info.status, "Running");
        assert_eq!(info.ready, "1/1");
        assert_eq!(info.restarts, 0);
        assert_eq!(info.node, "worker-1");
        assert_eq!(info.pod_ip, "10.244.0.5");
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_pending() {
        let pod = create_mock_pod("pending-pod", "staging", "Pending", 0, 2, 0, None, None);

        let info = map_pod_to_deployment_pod_info(&pod);

        assert_eq!(info.name, "pending-pod");
        assert_eq!(info.status, "Pending");
        assert_eq!(info.ready, "0/2");
        assert_eq!(info.node, "-");
        assert_eq!(info.pod_ip, "-");
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_with_restarts() {
        let pod = create_mock_pod(
            "crash-pod",
            "production",
            "Running",
            1,
            1,
            5,
            Some("worker-2"),
            Some("10.244.1.10"),
        );

        let info = map_pod_to_deployment_pod_info(&pod);

        assert_eq!(info.restarts, 5);
        assert_eq!(info.ready, "1/1");
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_multi_container() {
        let pod = create_mock_pod(
            "multi-container",
            "default",
            "Running",
            2,
            3,
            3,
            Some("worker-3"),
            Some("10.244.2.20"),
        );

        let info = map_pod_to_deployment_pod_info(&pod);

        assert_eq!(info.ready, "2/3");
        assert_eq!(info.restarts, 3);
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_no_containers() {
        let pod = create_mock_pod("empty-pod", "default", "Pending", 0, 0, 0, None, None);

        let info = map_pod_to_deployment_pod_info(&pod);

        assert_eq!(info.ready, "0/0");
        assert_eq!(info.restarts, 0);
    }

    #[test]
    fn test_map_pod_to_deployment_pod_info_age_format() {
        // Test that age is formatted (exact value depends on current time, but format should be valid)
        let pod = create_mock_pod(
            "test-pod",
            "default",
            "Running",
            1,
            1,
            0,
            Some("worker-1"),
            Some("10.244.0.1"),
        );

        let info = map_pod_to_deployment_pod_info(&pod);

        // Age should be in format like "1d", "2h", "30m", "45s", or "-"
        assert!(
            info.age.ends_with('d')
                || info.age.ends_with('h')
                || info.age.ends_with('m')
                || info.age.ends_with('s')
                || info.age == "-"
        );
    }

    #[test]
    fn test_deployment_pod_info_vec_serialization() {
        // Test that a vector of DeploymentPodInfo serializes correctly
        let pods = vec![
            DeploymentPodInfo {
                name: "pod-1".to_string(),
                namespace: "default".to_string(),
                status: "Running".to_string(),
                age: "1d".to_string(),
                ready: "1/1".to_string(),
                restarts: 0,
                node: "node-1".to_string(),
                pod_ip: "10.0.0.1".to_string(),
            },
            DeploymentPodInfo {
                name: "pod-2".to_string(),
                namespace: "default".to_string(),
                status: "Running".to_string(),
                age: "2d".to_string(),
                ready: "2/2".to_string(),
                restarts: 1,
                node: "node-2".to_string(),
                pod_ip: "10.0.0.2".to_string(),
            },
        ];

        let json = serde_json::to_string(&pods).expect("Should serialize vec");
        let deserialized: Vec<DeploymentPodInfo> =
            serde_json::from_str(&json).expect("Should deserialize vec");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].name, "pod-1");
        assert_eq!(deserialized[1].name, "pod-2");
    }

    #[test]
    fn test_deployment_pod_info_all_fields_populated() {
        // Test that all fields are properly captured
        let pod_info = DeploymentPodInfo {
            name: "full-test-pod-abc123xyz".to_string(),
            namespace: "kube-system".to_string(),
            status: "Running".to_string(),
            age: "30d".to_string(),
            ready: "5/5".to_string(),
            restarts: 100,
            node: "master-node-01.cluster.local".to_string(),
            pod_ip: "192.168.1.100".to_string(),
        };

        // Verify all fields
        assert!(!pod_info.name.is_empty());
        assert!(!pod_info.namespace.is_empty());
        assert!(!pod_info.status.is_empty());
        assert!(!pod_info.age.is_empty());
        assert!(!pod_info.ready.is_empty());
        assert!(pod_info.restarts >= 0);
        assert!(!pod_info.node.is_empty());
        assert!(!pod_info.pod_ip.is_empty());
    }
}
