use axum::{extract::State, Extension, Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::Claims;
use crate::error::AppError;
use crate::models::User;

/// ユーザー一覧を取得（管理者用）
pub async fn list_users(
    State(pool): State<SqlitePool>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let users: Vec<User> = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, role, created_at FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&pool)
    .await?;

    // パスワードハッシュを除外して返す
    let users_safe: Vec<serde_json::Value> = users
        .into_iter()
        .map(|u| json!({
            "id": u.id,
            "email": u.email,
            "name": u.name,
            "role": u.role,
            "created_at": u.created_at
        }))
        .collect();

    Ok(Json(json!({
        "success": true,
        "count": users_safe.len(),
        "users": users_safe
    })))
}

/// 統計情報を取得（管理者用）
pub async fn stats(
    State(pool): State<SqlitePool>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;

    let reservation_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reservations")
        .fetch_one(&pool)
        .await?;

    let confirmed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reservations WHERE status = 'confirmed'")
        .fetch_one(&pool)
        .await?;

    let cancelled_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reservations WHERE status = 'cancelled'")
        .fetch_one(&pool)
        .await?;

    let resource_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources WHERE is_active = 1")
        .fetch_one(&pool)
        .await?;

    Ok(Json(json!({
        "success": true,
        "stats": {
            "total_users": user_count,
            "total_reservations": reservation_count,
            "confirmed_reservations": confirmed_count,
            "cancelled_reservations": cancelled_count,
            "active_resources": resource_count
        }
    })))
}
