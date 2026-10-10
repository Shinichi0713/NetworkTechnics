use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use chrono::Local;
use petgraph::graph::UnGraph;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;

// -----------------------------------------------------------------------------
// GUI へブロードキャストするデータ構造
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct NodeHealth {
    pub id: usize,
    pub name: String,
    pub ip: String,
    pub rtt_ms: f32,
    pub gnn_status: String,      // GNN分析: Normal / Warning / Critical
    pub lstm_anomaly: bool,      // LSTM時系列異常フラグ
    pub prediction_error: f32,   // LSTM予測誤差 (MSE)
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardState {
    pub timestamp: String,
    pub nodes: Vec<NodeHealth>,
    pub active_alerts: Vec<String>,
}

// -----------------------------------------------------------------------------
// メインアプリケーション & リアルタイムシミュレーション
// -----------------------------------------------------------------------------

struct AppState {
    pub tx: broadcast::Sender<DashboardState>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 統合ネットワーク可視化 Web サーバー起動 ===");

    let (tx, _) = broadcast::channel::<DashboardState>(100);
    let app_state = Arc::new(AppState { tx: tx.clone() });

    // バックグラウンドタスク: 定期的にGNN/LSTM分析を実行してWebSocket経由で配信
    tokio::spawn(run_monitoring_pipeline(tx));

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Web GUI ダッシュボード URL: http://localhost:3000");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// -----------------------------------------------------------------------------
// バックグラウンド監視・分析パイプライン（模擬データ生成 & 推論）
// -----------------------------------------------------------------------------

async fn run_monitoring_pipeline(tx: broadcast::Sender<DashboardState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
    let mut step = 0;

    loop {
        interval.tick().await;
        step += 1;

        let now = Local::now().format("%H:%M:%S").to_string();

        // 疑似ノードデータの生成（ステップ経過により異常をシミュレート）
        let is_spiking = step >= 5 && step <= 10;

        let nodes = vec![
            NodeHealth {
                id: 0,
                name: "Core-Router".to_string(),
                ip: "10.0.0.1".to_string(),
                rtt_ms: if is_spiking { 120.5 } else { 12.3 },
                gnn_status: if is_spiking { "Critical SPOF".to_string() } else { "Normal".to_string() },
                lstm_anomaly: is_spiking,
                prediction_error: if is_spiking { 0.85 } else { 0.02 },
            },
            NodeHealth {
                id: 1,
                name: "Dist-Switch-A".to_string(),
                ip: "10.0.0.2".to_string(),
                rtt_ms: 15.1,
                gnn_status: "Normal".to_string(),
                lstm_anomaly: false,
                prediction_error: 0.03,
            },
            NodeHealth {
                id: 2,
                name: "Web-Server-01".to_string(),
                ip: "10.0.0.10".to_string(),
                rtt_ms: if is_spiking { 350.0 } else { 25.0 },
                gnn_status: if is_spiking { "Warning".to_string() } else { "Normal".to_string() },
                lstm_anomaly: is_spiking,
                prediction_error: if is_spiking { 1.42 } else { 0.01 },
            },
        ];

        let mut active_alerts = Vec::new();
        for node in &nodes {
            if node.lstm_anomaly {
                active_alerts.push(format!(
                    "[{}] {} ({}) で時系列予測偏差の異常を検知 (MSE: {:.2})",
                    node.name, node.ip, node.gnn_status, node.prediction_error
                ));
            }
        }

        let state_msg = DashboardState {
            timestamp: now,
            nodes,
            active_alerts,
        };

        let _ = tx.send(state_msg);
    }
}

// -----------------------------------------------------------------------------
// WebSocket ＆ Web API ハンドラ
// -----------------------------------------------------------------------------

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if let Ok(json_text) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json_text)).await.is_err() {
                break;
            }
        }
    }
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

// -----------------------------------------------------------------------------
// GUI ダッシュボード HTML / CSS / JS (Chart.js 組み込み)
// -----------------------------------------------------------------------------

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <title>ネットワーク統合観察可能基盤 (GNN + LSTM Dashboard)</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { font-family: 'Segoe UI', Arial, sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 20px; }
        h1 { color: #38bdf8; margin-bottom: 5px; }
        .subtitle { color: #94a3b8; font-size: 0.9em; margin-bottom: 20px; }
        .grid { display: grid; grid-template-columns: 2fr 1fr; gap: 20px; }
        .card { background: #1e293b; padding: 20px; border-radius: 8px; border: 1px solid #334155; }
        table { width: 100%; border-collapse: collapse; margin-top: 10px; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #334155; }
        th { background: #0f172a; color: #38bdf8; }
        .badge { padding: 4px 8px; border-radius: 4px; font-weight: bold; font-size: 0.85em; }
        .bg-normal { background: #16a34a; color: white; }
        .bg-warning { background: #ca8a04; color: white; }
        .bg-critical { background: #dc2626; color: white; }
        .alert-box { background: #450a0a; border: 1px solid #ef4444; color: #fca5a5; padding: 10px; border-radius: 4px; margin-bottom: 8px; font-size: 0.9em; }
        canvas { max-height: 250px; }
    </style>
</head>
<body>
    <h1>ネットワーク統合観測ダッシュボード</h1>
    <div class="subtitle">SNMP Metric Monitoring | GNN Topology Analysis | LSTM Anomaly Detection</div>

    <div class="grid">
        <!-- 左カラム: ノード監視テーブル ＆ RTTグラフ -->
        <div>
            <div class="card" style="margin-bottom: 20px;">
                <h3>リアルタイム・ノードヘルス一覧</h3>
                <table>
                    <thead>
                        <tr>
                            <th>ノード名</th>
                            <th>IPアドレス</th>
                            <th>応答時間 (RTT)</th>
                            <th>GNN評価 (トポロジー)</th>
                            <th>LSTM異常検知</th>
                        </tr>
                    </thead>
                    <tbody id="nodeTable"></tbody>
                </table>
            </div>

            <div class="card">
                <h3>レイテンシ推移 (Core-Router)</h3>
                <canvas id="rttChart"></canvas>
            </div>
        </div>

        <!-- 右カラム: 警報通知パネル -->
        <div>
            <div class="card">
                <h3 style="color: #f87171;">アクティブ・アラート (自動検出)</h3>
                <div id="alertContainer">
                    <div style="color: #94a3b8; font-size: 0.9em;">現在アラートはありません。</div>
                </div>
            </div>
        </div>
    </div>

    <script>
        // Chart.js 初期化
        const ctx = document.getElementById('rttChart').getContext('2d');
        const rttChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'RTT (ms)',
                    data: [],
                    borderColor: '#38bdf8',
                    backgroundColor: 'rgba(56, 189, 248, 0.1)',
                    fill: true,
                    tension: 0.2
                }]
            },
            options: {
                scales: {
                    x: { ticks: { color: '#94a3b8' }, grid: { color: '#334155' } },
                    y: { ticks: { color: '#94a3b8' }, grid: { color: '#334155' } }
                },
                plugins: { legend: { labels: { color: '#f8fafc' } } }
            }
        });

        // WebSocket リアルタイム通信
        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            
            // 1. テーブルの更新
            const tbody = document.getElementById("nodeTable");
            tbody.innerHTML = "";
            
            data.nodes.forEach(node => {
                const tr = document.createElement("tr");
                
                let gnnClass = "bg-normal";
                if (node.gnn_status.includes("Warning")) gnnClass = "bg-warning";
                if (node.gnn_status.includes("Critical")) gnnClass = "bg-critical";

                const lstmBadge = node.lstm_anomaly 
                    ? '<span class="badge bg-critical">ANOMALY DETECTED</span>'
                    : '<span class="badge bg-normal">NORMAL</span>';

                tr.innerHTML = `
                    <td><strong>${node.name}</strong></td>
                    <td><code>${node.ip}</code></td>
                    <td>${node.rtt_ms.toFixed(1)} ms</td>
                    <td><span class="badge ${gnnClass}">${node.gnn_status}</span></td>
                    <td>${lstmBadge}</td>
                `;
                tbody.appendChild(tr);
            });

            // 2. グラフの更新
            const coreNode = data.nodes.find(n => n.id === 0);
            if (coreNode) {
                rttChart.data.labels.push(data.timestamp);
                rttChart.data.datasets[0].data.push(coreNode.rtt_ms);

                if (rttChart.data.labels.length > 15) {
                    rttChart.data.labels.shift();
                    rttChart.data.datasets[0].data.shift();
                }
                rttChart.update();
            }

            // 3. アラートコンテナの更新
            const alertContainer = document.getElementById("alertContainer");
            if (data.active_alerts.length === 0) {
                alertContainer.innerHTML = '<div style="color: #94a3b8; font-size: 0.9em;">現在アラートはありません。</div>';
            } else {
                alertContainer.innerHTML = "";
                data.active_alerts.forEach(alert => {
                    const box = document.createElement("div");
                    box.className = "alert-box";
                    box.innerText = alert;
                    alertContainer.appendChild(box);
                });
            }
        };
    </script>
</body>
</html>
"#;