pub mod auth;
pub mod reservation;
pub mod resource;
pub mod user;
pub mod admin;

use axum::Json;
use serde_json::json;

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "schedule-app",
        "version": "0.1.0"
    }))
}
