use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ユーザーから受け取るデータ構造
#[derive(Deserialize)]
struct UserInput {
    name: String,
    age: i32,
    lat: f64,
    lon: f64,
}

// ArcGISに送るためのフィーチャ構造
#[derive(Serialize)]
struct ArcGISFeature {
    attributes: serde_json::Value,
    geometry: Geometry,
}

#[derive(Serialize)]
struct Geometry {
    x: f64,
    y: f64,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/register", post(handle_register));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("サーバー起動中: http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_register(Json(input): Json<UserInput>) -> Json<serde_json::Value> {
    // 1. ArcGISのフォーマットに変換
    let feature = ArcGISFeature {
        attributes: serde_json::json!({
            "NAME": input.name,
            "AGE": input.age,
        }),
        geometry: Geometry { x: input.lon, y: input.lat },
    };

    // 2. ArcGIS REST API へのリクエスト送信
    let result = send_to_arcgis(vec![feature]).await;

    match result {
        Ok(resp) => Json(serde_json::json!({"status": "success", "arcgis_response": resp})),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string()})),
    }
}

async fn send_to_arcgis(features: Vec<ArcGISFeature>) -> Result<serde_json::Value, reqwest::Error> {
    let client = reqwest::Client::new();
    
    // 対象のFeature ServiceのURL (実際のURLに置き換えてください)
    let url = "https://services.arcgis.com/YOUR_ORG/arcgis/rest/services/YOUR_LAYER/FeatureServer/0/addFeatures";

    let params = [
        ("features", serde_json::to_string(&features).unwrap()),
        ("f", "json".to_string()),
        // トークンが必要な場合はここに追加
        // ("token", "YOUR_ACCESS_TOKEN".to_string()),
    ];

    let response = client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response)
}