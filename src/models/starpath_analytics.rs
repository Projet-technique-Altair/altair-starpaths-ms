use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct StarpathAnalyticsQuery {
    pub window: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathAnalytics {
    pub window: String,
    pub starpath_id: Uuid,
    pub linked_labs_count: i32,
    pub learners_started: i64,
    pub learners_completed: i64,
    pub completion_rate: f64,
    pub progress_distribution: Vec<StarpathProgressPoint>,
    pub strongest_dropoff: Option<StarpathDropoff>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathProgressPoint {
    pub position: i32,
    pub reached_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathDropoff {
    pub from_position: i32,
    pub to_position: i32,
    pub previous_count: i64,
    pub next_count: i64,
    pub drop_percent: f64,
}

#[derive(Debug, Clone)]
pub struct StarpathProgressSnapshot {
    pub current_position: i32,
    pub started_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionSummary {
    pub lab_id: Uuid,
    pub status: String,
    pub completed_at: Option<NaiveDateTime>,
}
