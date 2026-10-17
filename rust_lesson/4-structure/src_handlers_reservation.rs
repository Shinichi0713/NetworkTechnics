use axum::{extract::{Path, Query, State}, Extension, Json};
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::Claims;
use crate::error::AppError;
use crate::models::{
    CreateReservationRequest, Reservation, ReservationQuery, ReservationResponse,
    UpdateReservationRequest,
};

/// 予約一覧を取得
pub async fn list(
    State(pool): State<SqlitePool>,
    Query(query): Query<ReservationQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut sql = String::from(
        r#"
        SELECT 
            r.id, r.user_id, r.title, r.description, r.start_time, r.end_time,
            r.status, r.resource_id, r.created_at, r.updated_at,
            u.name as user_name,
            res.name as resource_name
        FROM reservations r
        JOIN users u ON r.user_id = u.id
        LEFT JOIN resources res ON r.resource_id = res.id
        WHERE 1=1
        "#
    );

    if query.start_date.is_some() {
        sql.push_str(" AND r.start_time >= ?");
    }
    if query.end_date.is_some() {
        sql.push_str(" AND r.end_time <= ?");
    }
    if query.resource_id.is_some() {
        sql.push_str(" AND r.resource_id = ?");
    }
    // 一般ユーザーは自分の予約のみ表示
    if claims.role != "admin" {
        sql.push_str(" AND r.user_id = ?");
    } else if query.user_id.is_some() {
        sql.push_str(" AND r.user_id = ?");
    }

    sql.push_str(" ORDER BY r.start_time DESC");

    let mut query_builder = sqlx::query_as::<_, ReservationResponse>(&sql);

    if let Some(start) = query.start_date {
        query_builder = query_builder.bind(start);
    }
    if let Some(end) = query.end_date {
        query_builder = query_builder.bind(end);
    }
    if let Some(res_id) = query.resource_id {
        query_builder = query_builder.bind(res_id);
    }
    if claims.role != "admin" {
        query_builder = query_builder.bind(claims.sub);
    } else if let Some(uid) = query.user_id {
        query_builder = query_builder.bind(uid);
    }

    let reservations: Vec<ReservationResponse> = query_builder
        .fetch_all(&pool)
        .await?;

    Ok(Json(json!({
        "success": true,
        "count": reservations.len(),
        "reservations": reservations
    })))
}

/// 予約詳細を取得
pub async fn get(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let reservation: ReservationResponse = sqlx::query_as::<_, ReservationResponse>(
        r#"
        SELECT 
            r.id, r.user_id, r.title, r.description, r.start_time, r.end_time,
            r.status, r.resource_id, r.created_at, r.updated_at,
            u.name as user_name,
            res.name as resource_name
        FROM reservations r
        JOIN users u ON r.user_id = u.id
        LEFT JOIN resources res ON r.resource_id = res.id
        WHERE r.id = ?
        "#
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("予約が見つかりません".to_string()))?;

    // 権限チェック
    if claims.role != "admin" && reservation.user_id != claims.sub {
        return Err(AppError::Auth("この予約にアクセスする権限がありません".to_string()));
    }

    Ok(Json(json!({
        "success": true,
        "reservation": reservation
    })))
}

/// 予約を作成
pub async fn create(
    State(pool): State<SqlitePool>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateReservationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // バリデーション
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // 時間の妥当性チェック
    if req.end_time <= req.start_time {
        return Err(AppError::Validation(
            "終了時刻は開始時刻より後である必要があります".to_string(),
        ));
    }

    // 過去の日時チェック
    if req.start_time < Utc::now() {
        return Err(AppError::Validation(
            "過去の日時には予約できません".to_string(),
        ));
    }

    // リソースの存在確認
    if let Some(res_id) = req.resource_id {
        let resource: Option<(i64,)> = sqlx::query_as("SELECT id FROM resources WHERE id = ? AND is_active = 1")
            .bind(res_id)
            .fetch_optional(&pool)
            .await?;

        if resource.is_none() {
            return Err(AppError::NotFound("指定されたリソースが見つかりません".to_string()));
        }
    }

    // 予約重複チェック
    let conflict: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT id FROM reservations 
        WHERE resource_id = ?1 
        AND status != 'cancelled'
        AND ((start_time <= ?2 AND end_time > ?2) 
             OR (start_time < ?3 AND end_time >= ?3)
             OR (start_time >= ?2 AND end_time <= ?3))
        "#
    )
    .bind(req.resource_id)
    .bind(req.start_time)
    .bind(req.end_time)
    .fetch_optional(&pool)
    .await?;

    if conflict.is_some() {
        return Err(AppError::Conflict(
            "指定された時間帯は既に予約されています".to_string(),
        ));
    }

    let now = Utc::now();
    let result = sqlx::query(
        r#"
        INSERT INTO reservations 
        (user_id, title, description, start_time, end_time, status, resource_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 'confirmed', ?6, ?7, ?7)
        RETURNING id
        "#
    )
    .bind(claims.sub)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(req.resource_id)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    let id: i64 = result.try_get("id").unwrap_or_default();

    tracing::info!("予約作成: id={}, user_id={}, title={}", id, claims.sub, req.title);

    Ok(Json(json!({
        "success": true,
        "message": "予約が完了しました",
        "reservation_id": id
    })))
}

/// 予約を更新
pub async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<UpdateReservationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 既存予約の確認
    let existing: Reservation = sqlx::query_as::<_, Reservation>("SELECT * FROM reservations WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("予約が見つかりません".to_string()))?;

    // 権限チェック
    if claims.role != "admin" && existing.user_id != claims.sub {
        return Err(AppError::Auth("この予約を更新する権限がありません".to_string()));
    }

    // キャセル済みは更新不可
    if existing.status == "cancelled" {
        return Err(AppError::Validation("キャンセル済みの予約は更新できません".to_string()));
    }

    let start_time = req.start_time.unwrap_or(existing.start_time);
    let end_time = req.end_time.unwrap_or(existing.end_time);

    // 時間の妥当性チェック
    if end_time <= start_time {
        return Err(AppError::Validation(
            "終了時刻は開始時刻より後である必要があります".to_string(),
        ));
    }

    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE reservations 
        SET title = COALESCE(?1, title),
            description = ?2,
            start_time = ?3,
            end_time = ?4,
            status = COALESCE(?5, status),
            resource_id = ?6,
            updated_at = ?7
        WHERE id = ?8
        "#
    )
    .bind(req.title)
    .bind(req.description)
    .bind(start_time)
    .bind(end_time)
    .bind(req.status)
    .bind(req.resource_id)
    .bind(now)
    .bind(id)
    .execute(&pool)
    .await?;

    tracing::info!("予約更新: id={}, user_id={}", id, claims.sub);

    Ok(Json(json!({
        "success": true,
        "message": "予約を更新しました"
    })))
}

/// 予約を削除（キャンセル）
pub async fn delete(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let existing: Reservation = sqlx::query_as::<_, Reservation>("SELECT * FROM reservations WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("予約が見つかりません".to_string()))?;

    // 権限チェック
    if claims.role != "admin" && existing.user_id != claims.sub {
        return Err(AppError::Auth("この予約をキャンセルする権限がありません".to_string()));
    }

    // 論理削除（ステータスをcancelledに変更）
    sqlx::query("UPDATE reservations SET status = 'cancelled', updated_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(id)
        .execute(&pool)
        .await?;

    tracing::info!("予約キャンセル: id={}, user_id={}", id, claims.sub);

    Ok(Json(json!({
        "success": true,
        "message": "予約をキャンセルしました"
    })))
}

/// 自分の予約一覧
pub async fn my_reservations(
    State(pool): State<SqlitePool>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let reservations: Vec<ReservationResponse> = sqlx::query_as::<_, ReservationResponse>(
        r#"
        SELECT 
            r.id, r.user_id, r.title, r.description, r.start_time, r.end_time,
            r.status, r.resource_id, r.created_at, r.updated_at,
            u.name as user_name,
            res.name as resource_name
        FROM reservations r
        JOIN users u ON r.user_id = u.id
        LEFT JOIN resources res ON r.resource_id = res.id
        WHERE r.user_id = ? AND r.status != 'cancelled'
        ORDER BY r.start_time ASC
        "#
    )
    .bind(claims.sub)
    .fetch_all(&pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "count": reservations.len(),
        "reservations": reservations
    })))
}
