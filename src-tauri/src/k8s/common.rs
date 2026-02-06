use chrono;
use k8s_openapi;

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct K8sEventInfo {
    pub event_type: String,
    pub reason: String,
    pub message: String,
    pub count: i32,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub source: String,
}

pub fn calculate_age(
    timestamp: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,
) -> String {
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

pub fn get_created_at(
    timestamp: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,
) -> i64 {
    if let Some(ts) = timestamp {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts.0.to_string()) {
            return dt.timestamp();
        }
    }
    0
}
