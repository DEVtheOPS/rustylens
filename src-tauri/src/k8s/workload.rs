use crate::cluster_manager::ClusterManagerState;
use crate::k8s::client::create_client_for_cluster;
use crate::k8s::common::{calculate_age, get_created_at, WorkloadSummary};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Endpoints, LimitRange, PersistentVolume, PersistentVolumeClaim, ResourceQuota,
    Secret, Service, ServiceAccount,
};
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use k8s_openapi::api::rbac::v1::{ClusterRole, Role};
use k8s_openapi::api::storage::v1::StorageClass;
use kube::api::Api;
use tauri::State;

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

            let list = api
                .list(&Default::default())
                .await
                .map_err(|e| e.to_string())?;
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
            api.delete(&name, &Default::default())
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

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

            let list = api
                .list(&Default::default())
                .await
                .map_err(|e| e.to_string())?;
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
            api.delete(&name, &Default::default())
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    };
}

fn map_deployment_to_summary(d: Deployment) -> WorkloadSummary {
    let meta = d.metadata;
    let spec = d.spec.unwrap_or_default();
    let status = d.status.unwrap_or_default();

    let _replicas = status.replicas.unwrap_or(0);
    let ready = status.ready_replicas.unwrap_or(0);
    let status_str = format!("{}/{}", ready, spec.replicas.unwrap_or(1));

    let images = if let Some(template) = spec.template.spec {
        template
            .containers
            .into_iter()
            .map(|c| c.image.unwrap_or_default())
            .collect()
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
        template
            .containers
            .into_iter()
            .map(|c| c.image.unwrap_or_default())
            .collect()
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
        template
            .containers
            .into_iter()
            .map(|c| c.image.unwrap_or_default())
            .collect()
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
            tspec
                .containers
                .into_iter()
                .map(|c| c.image.unwrap_or_default())
                .collect()
        } else {
            vec![]
        }
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
        template
            .containers
            .into_iter()
            .map(|c| c.image.unwrap_or_default())
            .collect()
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
            template
                .containers
                .into_iter()
                .map(|c| c.image.unwrap_or_default())
                .collect()
        } else {
            vec![]
        }
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
        status: format!(
            "{} ({} items)",
            s.type_.unwrap_or_else(|| "Opaque".to_string()),
            count
        ),
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
    let ports = spec
        .ports
        .unwrap_or_default()
        .iter()
        .map(|p| format!("{}", p.port))
        .collect::<Vec<_>>()
        .join(",");

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
    let count = e
        .subsets
        .map(|s| {
            s.iter()
                .map(|ss| ss.addresses.as_ref().map(|a| a.len()).unwrap_or(0))
                .sum::<usize>()
        })
        .unwrap_or(0);

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
    let lbs = i
        .status
        .and_then(|s| s.load_balancer)
        .and_then(|lb| lb.ingress)
        .map(|ing| {
            ing.iter()
                .map(|ip| ip.ip.clone().or(ip.hostname.clone()).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();

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
    let capacity = status
        .capacity
        .and_then(|c| c.get("storage").map(|q| q.0.clone()))
        .unwrap_or_default();

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
    let capacity = spec
        .capacity
        .and_then(|c| c.get("storage").map(|q| q.0.clone()))
        .unwrap_or_default();

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

impl_workload_commands!(
    Deployment,
    cluster_list_deployments,
    cluster_delete_deployment,
    map_deployment_to_summary
);
impl_workload_commands!(
    StatefulSet,
    cluster_list_statefulsets,
    cluster_delete_statefulset,
    map_statefulset_to_summary
);
impl_workload_commands!(
    DaemonSet,
    cluster_list_daemonsets,
    cluster_delete_daemonset,
    map_daemonset_to_summary
);
impl_workload_commands!(
    ReplicaSet,
    cluster_list_replicasets,
    cluster_delete_replicaset,
    map_replicaset_to_summary
);
impl_workload_commands!(
    Job,
    cluster_list_jobs,
    cluster_delete_job,
    map_job_to_summary
);
impl_workload_commands!(
    CronJob,
    cluster_list_cronjobs,
    cluster_delete_cronjob,
    map_cronjob_to_summary
);

impl_workload_commands!(
    ConfigMap,
    cluster_list_config_maps,
    cluster_delete_config_map,
    map_configmap_to_summary
);
impl_workload_commands!(
    Secret,
    cluster_list_secrets,
    cluster_delete_secret,
    map_secret_to_summary
);
impl_workload_commands!(
    ResourceQuota,
    cluster_list_resource_quotas,
    cluster_delete_resource_quota,
    map_resource_quota_to_summary
);
impl_workload_commands!(
    LimitRange,
    cluster_list_limit_ranges,
    cluster_delete_limit_range,
    map_limit_range_to_summary
);
impl_workload_commands!(
    HorizontalPodAutoscaler,
    cluster_list_hpa,
    cluster_delete_hpa,
    map_hpa_to_summary
);
impl_workload_commands!(
    PodDisruptionBudget,
    cluster_list_pdb,
    cluster_delete_pdb,
    map_pdb_to_summary
);
impl_workload_commands!(
    Service,
    cluster_list_services,
    cluster_delete_service,
    map_service_to_summary
);
impl_workload_commands!(
    Endpoints,
    cluster_list_endpoints,
    cluster_delete_endpoint,
    map_endpoints_to_summary
);
impl_workload_commands!(
    Ingress,
    cluster_list_ingresses,
    cluster_delete_ingress,
    map_ingress_to_summary
);
impl_workload_commands!(
    NetworkPolicy,
    cluster_list_network_policies,
    cluster_delete_network_policy,
    map_network_policy_to_summary
);
impl_workload_commands!(
    PersistentVolumeClaim,
    cluster_list_pvc,
    cluster_delete_pvc,
    map_pvc_to_summary
);
impl_workload_commands!(
    ServiceAccount,
    cluster_list_service_accounts,
    cluster_delete_service_account,
    map_service_account_to_summary
);
impl_workload_commands!(
    Role,
    cluster_list_roles,
    cluster_delete_role,
    map_role_to_summary
);

// Cluster Scoped
impl_cluster_resource_commands!(
    PersistentVolume,
    cluster_list_pv,
    cluster_delete_pv,
    map_pv_to_summary
);
impl_cluster_resource_commands!(
    StorageClass,
    cluster_list_storage_classes,
    cluster_delete_storage_class,
    map_storage_class_to_summary
);
impl_cluster_resource_commands!(
    ClusterRole,
    cluster_list_cluster_roles,
    cluster_delete_cluster_role,
    map_cluster_role_to_summary
);
