use axum::{extract::State, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::generate_token;
use crate::error::AppError;
use crate::models::{CreateUserRequest, LoginRequest, LoginResponse, User, UserResponse};

/// ユーザー登録
pub async fn register(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // バリデーション
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // メールアドレス重複チェック
    let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&req.email)
        .fetch_optional(&pool)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "このメールアドレスは既に登録されています".to_string(),
        ));
    }

    // パスワードハッシュ化
    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // ユーザー作成
    let user_id = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, role)
        VALUES (?1, ?2, ?3, ?4)
        RETURNING id
        "#,
    )
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.name)
    .bind("user")
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e))?;

    let id: i64 = user_id.try_get("id").unwrap_or_default();

    tracing::info!("新規ユーザー登録: id={}, email={}", id, req.email);

    Ok(Json(json!({
        "success": true,
        "message": "ユーザー登録が完了しました",
        "user_id": id
    })))
}

/// ログイン
pub async fn login(
    State(pool): State<SqlitePool>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // ユーザー検索
    let user: Option<User> = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&req.email)
        .fetch_optional(&pool)
        .await?;

    let user = user.ok_or_else(|| AppError::Auth("メールアドレスまたはパスワードが正しくありません".to_string()))?;

    // パスワード検証
    let valid = verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !valid {
        return Err(AppError::Auth(
            "メールアドレスまたはパスワードが正しくありません".to_string(),
        ));
    }

    // JWTトークン生成
    let token = generate_token(user.id, &user.email, &user.role)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("ユーザーログイン: id={}, email={}", user.id, user.email);

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}
