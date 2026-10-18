use axum::{extract::State, Extension, Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::Claims;
use crate::error::AppError;
use crate::models::UserResponse;

/// 現在のユーザー情報を取得
pub async fn me(
    State(pool): State<SqlitePool>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user: UserResponse = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(claims.sub)
    .fetch_one(&pool)
    .await
    .map(|u| u.into())
    .map_err(|_| AppError::NotFound("ユーザーが見つかりません".to_string()))?;

    Ok(Json(json!({
        "success": true,
        "user": user
    })))
}
