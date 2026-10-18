use axum::{extract::{Path, State}, Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::{CreateResourceRequest, Resource};

/// リソース一覧を取得
pub async fn list(
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let resources: Vec<Resource> = sqlx::query_as::<_, Resource>(
        "SELECT * FROM resources WHERE is_active = 1 ORDER BY name"
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "count": resources.len(),
        "resources": resources
    })))
}

/// リソース詳細を取得
pub async fn get(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let resource: Resource = sqlx::query_as::<_, Resource>("SELECT * FROM resources WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("リソースが見つかりません".to_string()))?;

    Ok(Json(json!({
        "success": true,
        "resource": resource
    })))
}

/// リソースを作成（管理者用）
pub async fn create(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateResourceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let result = sqlx::query(
        "INSERT INTO resources (name, description, resource_type, capacity, is_active) VALUES (?1, ?2, ?3, ?4, 1) RETURNING id"
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.resource_type)
    .bind(req.capacity)
    .fetch_one(&pool)
    .await?;

    let id: i64 = result.try_get("id").unwrap_or_default();

    tracing::info!("リソース作成: id={}, name={}", id, req.name);

    Ok(Json(json!({
        "success": true,
        "message": "リソースを作成しました",
        "resource_id": id
    })))
}
