use reqwest::Error;
use serde::{Deserialize, Serialize};

// ArcGISのレスポンス構造を定義
#[derive(Deserialize, Debug)]
struct ArcGISResponse {
    features: Vec<Feature>,
}

#[derive(Deserialize, Debug)]
struct Feature {
    attributes: serde_json::Value,
    geometry: Option<serde_json::Value>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // 1. エンドポイントの設定（例：米国の州境界サービス）
    let service_url = "https://services.arcgis.com/P3ePLMYs2RVChqki/arcgis/rest/services/USA_States_Generalized/FeatureServer/0/query";

    // 2. クエリパラメータの構築
    let params = [
        ("where", "1=1"),           // 全件（または特定の条件）
        ("outFields", "STATE_NAME,POP2010"), // 取得したいカラム
        ("f", "json"),               // レスポンス形式をJSONに指定
        ("resultRecordCount", "5"),  // 取得件数制限
    ];

    // 3. リクエストの送信
    let client = reqwest::Client::new();
    let response = client
        .get(service_url)
        .query(&params)
        .send()
        .await?;

    // 4. 結果の解析と表示
    if response.status().is_success() {
        let data: ArcGISResponse = response.json().await?;
        
        println!("--- ArcGIS Data fetched via Rust ---");
        for feature in data.features {
            let name = &feature.attributes["STATE_NAME"];
            let pop = &feature.attributes["POP2010"];
            println!("州名: {:<15} | 2010年人口: {}", name, pop);
        }
    } else {
        println!("Error: {}", response.status());
    }

    Ok(())
}