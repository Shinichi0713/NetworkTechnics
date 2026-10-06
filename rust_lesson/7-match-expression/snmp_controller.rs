use anyhow::{anyhow, Result};
use chrono::Local;
use clap::Parser;
use snmp::{SyncSession, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

// -----------------------------------------------------------------------------
// コマンドライン引数設定
// -----------------------------------------------------------------------------
#[derive(Parser, Debug)]
#[command(author, version, about = "Rust用 SNMP デバイス状態監視ツール")]
struct Config {
    /// 監視対象デバイスの IP アドレス
    #[arg(short, long, default_value = "127.0.0.1")]
    target: String,

    /// SNMP コミュニティ名
    #[arg(short, long, default_value = "public")]
    community: String,

    /// SNMP ポート番号
    #[arg(short, long, default_value_t = 161)]
    port: u16,

    /// 監視インターバル（秒）
    #[arg(short, long, default_value_t = 5)]
    interval: u64,
}

// -----------------------------------------------------------------------------
// OID (Object Identifier) 定義
// -----------------------------------------------------------------------------
// System MIB
const OID_SYS_DESCR: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 1, 0];   // sysDescr
const OID_SYS_UPTIME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 3, 0];  // sysUpTime
const OID_SYS_NAME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 5, 0];    // sysName

// IF-MIB (インターフェース統計)
const OID_IF_NUMBER: &[u32] = &[1, 3, 6, 1, 2, 1, 2, 1, 0];  // ifNumber
const OID_IF_DESCR: &[u32] = &[1, 3, 6, 1, 2, 1, 2, 2, 1, 2]; // ifDescr (Walk用)
const OID_IF_OPER_STATUS: &[u32] = &[1, 3, 6, 1, 2, 1, 2, 2, 1, 8]; // ifOperStatus (Walk用)

// -----------------------------------------------------------------------------
// 監視データ構造体
// -----------------------------------------------------------------------------
#[derive(Debug)]
struct DeviceStatus {
    sys_name: String,
    sys_descr: String,
    uptime_ticks: u32,
    if_count: i32,
    interfaces: Vec<InterfaceInfo>,
}

#[derive(Debug)]
struct InterfaceInfo {
    index: u32,
    name: String,
    status: OperStatus,
}

#[derive(Debug, PartialEq, Eq)]
enum OperStatus {
    Up,
    Down,
    Testing,
    Unknown(i32),
}

impl From<i32> for OperStatus {
    fn from(val: i32) -> Self {
        match val {
            1 => OperStatus::Up,
            2 => OperStatus::Down,
            3 => OperStatus::Testing,
            other => OperStatus::Unknown(other),
        }
    }
}

// -----------------------------------------------------------------------------
// SNMP 取得ロジック
// -----------------------------------------------------------------------------
fn poll_device(target_ip: &str, port: u16, community: &str) -> Result<DeviceStatus> {
    let addr: SocketAddr = format!("{}:{}", target_ip, port).parse()?;
    
    // SNMP セッションの確立 (タイムアウト 3秒)
    let mut session = SyncSession::new(
        addr,
        community.as_bytes(),
        Some(Duration::from_secs(3)),
        0, // リトライ回数
    )
    .map_err(|e| anyhow!("SNMP セッション作成失敗: {:?}", e))?;

    // 1. GET リクエスト (システム基本情報の取得)
    let response = session
        .getmultiple(&[OID_SYS_NAME, OID_SYS_DESCR, OID_SYS_UPTIME, OID_IF_NUMBER])
        .map_err(|e| anyhow!("SNMP GET エラー: {:?}", e))?;

    let mut sys_name = String::from("N/A");
    let mut sys_descr = String::from("N/A");
    let mut uptime_ticks = 0u32;
    let mut if_count = 0i32;

    for (oid, val) in response {
        if oid == OID_SYS_NAME {
            if let Value::OctetString(bytes) = val {
                sys_name = String::from_utf8_lossy(bytes).to_string();
            }
        } else if oid == OID_SYS_DESCR {
            if let Value::OctetString(bytes) = val {
                sys_descr = String::from_utf8_lossy(bytes).to_string().replace('\n', " ");
            }
        } else if oid == OID_SYS_UPTIME {
            if let Value::Timeticks(ticks) = val {
                uptime_ticks = ticks;
            }
        } else if oid == OID_IF_NUMBER {
            if let Value::Integer(num) = val {
                if_count = num as i32;
            }
        }
    }

    // 2. WALK (GETNEXT) によるインターフェース名の全件取得
    let mut interfaces = Vec::new();

    if let Ok(mut walk_name) = session.walkiter(OID_IF_DESCR) {
        while let Some(Ok((oid, val))) = walk_name.next() {
            if let Some(idx) = oid.to_vec().last().copied() {
                if let Value::OctetString(bytes) = val {
                    let name = String::from_utf8_lossy(bytes).to_string();
                    interfaces.push(InterfaceInfo {
                        index: idx,
                        name,
                        status: OperStatus::Unknown(0),
                    });
                }
            }
        }
    }

    // 3. インターフェースの Operation Status を設定
    if let Ok(mut walk_status) = session.walkiter(OID_IF_OPER_STATUS) {
        while let Some(Ok((oid, val))) = walk_status.next() {
            if let Some(idx) = oid.to_vec().last().copied() {
                if let Value::Integer(st) = val {
                    if let Some(iface) = interfaces.iter_mut().find(|i| i.index == idx) {
                        iface.status = OperStatus::from(st as i32);
                    }
                }
            }
        }
    }

    Ok(DeviceStatus {
        sys_name,
        sys_descr,
        uptime_ticks,
        if_count,
        interfaces,
    })
}

// -----------------------------------------------------------------------------
// メインルーチン (定期実行ループ)
// -----------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    let cfg = Config::parse();

    println!("=== SNMP リアルタイムデバイス監視開始 ===");
    println!("対象ノード: {}:{}", cfg.target, cfg.port);
    println!("コミュニティ: {}", cfg.community);
    println!("監視間隔: {} 秒", cfg.interval);
    println!("--------------------------------------------------\n");

    loop {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 別スレッド(ブロッキング)でSNMP通信を実行し、Tokioランタイムをブロックしない
        let target = cfg.target.clone();
        let community = cfg.community.clone();
        let port = cfg.port;

        let res = tokio::task::spawn_blocking(move || poll_device(&target, port, &community)).await;

        match res {
            Ok(Ok(status)) => {
                let uptime_sec = status.uptime_ticks / 100;
                let days = uptime_sec / 86400;
                let hours = (uptime_sec % 86400) / 3600;
                let minutes = (uptime_sec % 3600) / 60;

                println!("[{}] 状態: SUCCESS", now);
                println!("  ホスト名   : {}", status.sys_name);
                println!("  説明       : {}", status.sys_descr);
                println!("  稼働時間   : {}日 {}時間 {}分 (Ticks: {})", days, hours, minutes, status.uptime_ticks);
                println!("  IF総数     : {}", status.if_count);
                println!("  IFステータス:");
                
                for iface in status.interfaces.iter().take(10) { // 上位10個を表示
                    let st_str = match iface.status {
                        OperStatus::Up => "\x1b[32mUP\x1b[0m",
                        OperStatus::Down => "\x1b[31mDOWN\x1b[0m",
                        _ => "UNKNOWN",
                    };
                    println!("    - [{:2}] {:<15} : {}", iface.index, iface.name, st_str);
                }
            }
            Ok(Err(e)) => {
                eprintln!("[{}] 状態: ERROR - {}", now, e);
            }
            Err(e) => {
                eprintln!("[{}] 状態: TASK_ERROR - {}", now, e);
            }
        }

        println!("--------------------------------------------------");
        sleep(Duration::from_secs(cfg.interval)).await;
    }
}

use anyhow::Result;
use chrono::Local;
use petgraph::algo::{bellman_ford, dijkstra};
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::IntoNodeReferences;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

// -----------------------------------------------------------------------------
// データ構造の定義
// -----------------------------------------------------------------------------

/// 監視対象の物理/仮想デバイス（グラフの頂点データ）
#[derive(Debug, Clone)]
pub struct DeviceNode {
    pub ip: String,
    pub name: String,
    pub role: DeviceRole,
    pub latency_ms: f64, // 応答遅延（ミリ秒）
    pub is_alive: bool,  // 生死状態
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceRole {
    CoreRouter,
    Switch,
    Server,
}

/// デバイス間の物理/論理リンク（グラフの辺データ）
#[derive(Debug, Clone)]
pub struct LinkEdge {
    pub bandwidth_gbps: f64,
    pub traffic_load: f64, // トラフィック負荷率 (0.0 〜 1.0)
}

impl LinkEdge {
    /// グラフ解析用：負荷と帯域からリンクの「コスト（重み）」を算出
    /// 高負荷・低帯域なほどコスト（通過抵抗）が高くなる
    pub fn calculate_cost(&pub self) -> f64 {
        if self.traffic_load >= 1.0 {
            10000.0 // 飽和リンクは極めて高いコストを設定
        } else {
            (1.0 + self.traffic_load) / self.bandwidth_gbps
        }
    }
}

/// グラフ解析によって算出されるネットワークの傾向・健康度
#[derive(Debug)]
pub struct NetworkGraphMetrics {
    pub degree_centrality: HashMap<String, usize>, // 次数中心性（接続の集約度）
    pub bottleneck_links: Vec<(String, String, f64)>, // ボトルネック傾向にあるリンク
    pub critical_nodes: Vec<String>,               // 単一障害点になり得る重要ノード
}

// -----------------------------------------------------------------------------
// グラフベースのネットワーク監視マネージャー
// -----------------------------------------------------------------------------

pub struct NetworkMonitorGraph {
    // 無向重み付きグラフ構造体 (Node: DeviceNode, Edge: LinkEdge)
    graph: UnGraph<DeviceNode, LinkEdge>,
    ip_to_node: HashMap<String, NodeIndex>,
}

impl NetworkMonitorGraph {
    pub fn new() -> Self {
        Self {
            graph: UnGraph::new_undirected(),
            ip_to_node: HashMap::new(),
        }
    }

    /// トポロジーへデバイス（ノード）を追加
    pub fn add_device(&mut self, ip: &str, name: &str, role: DeviceRole) -> NodeIndex {
        let node_data = DeviceNode {
            ip: ip.to_string(),
            name: name.to_string(),
            role,
            latency_ms: 1.0,
            is_alive: true,
        };
        let idx = self.graph.add_node(node_data);
        self.ip_to_node.insert(ip.to_string(), idx);
        idx
    }

    /// デバイス間にリンク（エッジ）を張る
    pub fn add_link(&mut self, ip_a: &str, ip_b: &str, bandwidth_gbps: f64) -> Result<()> {
        let idx_a = *self
            .ip_to_node
            .get(ip_a)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", ip_a))?;
        let idx_b = *self
            .ip_to_node
            .get(ip_b)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", ip_b))?;

        let edge_data = LinkEdge {
            bandwidth_gbps,
            traffic_load: 0.1, // 初期値
        };
        self.graph.add_edge(idx_a, idx_b, edge_data);
        Ok(())
    }

    /// メトリクス更新（SNMP等からの極小ポーリング結果を模倣）
    pub fn update_link_load(&mut self, ip_a: &str, ip_b: &str, load: f64) {
        if let (Some(&idx_a), Some(&idx_b)) =
            (self.ip_to_node.get(ip_a), self.ip_to_node.get(ip_b))
        {
            if let Some(edge_idx) = self.graph.find_edge(idx_a, idx_b) {
                if let Some(edge) = self.graph.edge_weight_mut(edge_idx) {
                    edge.traffic_load = load;
                }
            }
        }
    }

    /// グラフ理論による分析・傾向評価の実行
    pub fn analyze_trends(&self) -> NetworkGraphMetrics {
        let mut degree_map = HashMap::new();
        let mut bottleneck_links = Vec::new();
        let mut critical_nodes = Vec::new();

        // 1. 次数中心性 (Degree Centrality) の計測
        // トポロジーのハブとなっている重要機器を同定
        for node_idx in self.graph.node_indices() {
            let node = &self.graph[node_idx];
            let neighbors_count = self.graph.neighbors(node_idx).count();
            degree_map.insert(node.name.clone(), neighbors_count);

            // 接続数が多く、かつCore/Switchである場合は障害時の影響度大（クリティカルノード）
            if neighbors_count >= 3 || node.role == DeviceRole::CoreRouter {
                critical_nodes.push(node.name.clone());
            }
        }

        // 2. 最短経路計算によるコスト・ボトルネック分析 (Dijkstra)
        // リンクの負荷状況をコスト関数化し、経路上の通過障害確率・ボトルネックを抽出
        for edge_idx in self.graph.edge_indices() {
            if let Some((idx_a, idx_b)) = self.graph.edge_endpoints(edge_idx) {
                let edge = &self.graph[edge_idx];
                let cost = edge.calculate_cost();

                // 負荷率が 75% を超えるリンクをボトルネック傾向として捕捉
                if edge.traffic_load > 0.75 {
                    let node_a = &self.graph[idx_a];
                    let node_b = &self.graph[idx_b];
                    bottleneck_links.push((
                        node_a.name.clone(),
                        node_b.name.clone(),
                        edge.traffic_load * 100.0,
                    ));
                }
            }
        }

        NetworkGraphMetrics {
            degree_centrality: degree_map,
            bottleneck_links,
            critical_nodes,
        }
    }

    /// 特定ノードからの最短通信経路・実効遅延コストのシミュレーション
    pub fn evaluate_shortest_path(&self, src_ip: &str, target_ip: &str) -> Option<f64> {
        let &src_idx = self.ip_to_node.get(src_ip)?;
        let &target_idx = self.ip_to_node.get(target_ip)?;

        // ダイクストラ法でリンクのトラフィックコストを加味した最短経路を導出
        let path_map = dijkstra(&self.graph, src_idx, Some(target_idx), |e| {
            e.weight().calculate_cost()
        });

        path_map.get(&target_idx).copied()
    }
}

// -----------------------------------------------------------------------------
// メイン監視ループ
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== グラフ理論を用いたネットワークトポロジー監視システム ===");

    let mut monitor = NetworkMonitorGraph::new();

    // トポロジーの構築（スター型 ＋ メッシュ型のハイブリッド構造）
    monitor.add_device("10.0.0.1", "Core-Router", DeviceRole::CoreRouter);
    monitor.add_device("10.0.0.2", "Dist-Switch-A", DeviceRole::Switch);
    monitor.add_device("10.0.0.3", "Dist-Switch-B", DeviceRole::Switch);
    monitor.add_device("10.0.0.10", "Web-Server-01", DeviceRole::Server);
    monitor.add_device("10.0.0.11", "Web-Server-02", DeviceRole::Server);
    monitor.add_device("10.0.0.20", "DB-Server-Primary", DeviceRole::Server);

    // 物理リンクの設定 (IP-A, IP-B, 帯域Gbps)
    monitor.add_link("10.0.0.1", "10.0.0.2", 10.0)?;
    monitor.add_link("10.0.0.1", "10.0.0.3", 10.0)?;
    monitor.add_link("10.0.0.2", "10.0.0.10", 1.0)?;
    monitor.add_link("10.0.0.2", "10.0.0.11", 1.0)?;
    monitor.add_link("10.0.0.3", "10.0.0.20", 1.0)?;
    monitor.add_link("10.0.0.2", "10.0.0.3", 10.0)?; // 冗長リンク

    println!("トポロジー構築完了。リアルタイム解析を開始します...\n");

    // モック用：時間経過に伴うトラフィック変動シミュレーション
    let mut step = 0;

    loop {
        step += 1;
        let now = Local::now().format("%H:%M:%S").to_string();

        // トラフィック負荷の変化をシミュレート
        if step == 2 {
            // Web-Server-01 へのリンクが高負荷化
            monitor.update_link_load("10.0.0.2", "10.0.0.10", 0.85);
        } else if step == 3 {
            // Core-Router と Switch-A 間が輻輳状態に移行
            monitor.update_link_load("10.0.0.1", "10.0.0.2", 0.95);
        }

        // グラフ構造の解析実行
        let metrics = monitor.analyze_trends();

        println!("[{}] --- ネットワークトポロジー解析スコア ---", now);

        // 1. 重要ノード（影響範囲の広いノード）の評価
        println!("  [重要構造ノード (SPOFリスク)]");
        for node_name in &metrics.critical_nodes {
            let deg = metrics.degree_centrality.get(node_name).unwrap_or(&0);
            println!("   - ノード: {:<18} | 隣接接続数: {}", node_name, deg);
        }

        // 2. ボトルネック傾向の検知
        if !metrics.bottleneck_links.is_empty() {
            println!("  \x1b[31m[警報: トラフィックボトルネック検知]\x1b[0m");
            for (from, to, load) in &metrics.bottleneck_links {
                println!(
                    "   - リンク: {} <--> {} | 負荷率: {:.1}%",
                    from, to, load
                );
            }
        } else {
            println!("  [リンク状態] 正常 (全リンク負荷率 75% 未満)");
        }

        // 3. グラフ理論ベースの動的最短コスト計算 (Web-Server-01 -> DB-Server)
        if let Some(cost) = monitor.evaluate_shortest_path("10.0.0.10", "10.0.0.20") {
            println!(
                "  [経路解析] Web-Server-01 -> DB-Server コスト値: {:.4}",
                cost
            );
        }

        println!("------------------------------------------------------------\n");

        if step >= 3 {
            println!("監視ループのデモが完了しました。");
            break;
        }

        sleep(Duration::from_secs(3)).await;
    }

    Ok(())
}

use anyhow::{anyhow, Result};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{NodeIndex, UnGraph};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::Command;

// -----------------------------------------------------------------------------
// データ構造の定義
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum DeviceRole {
    CoreRouter,
    Switch,
    Server,
}

#[derive(Debug, Clone)]
pub struct DeviceNode {
    pub ip: String,
    pub name: String,
    pub role: DeviceRole,
}

#[derive(Debug, Clone)]
pub struct LinkEdge {
    pub bandwidth_gbps: f64,
    pub traffic_load: f64, // 負荷率 (0.0 ～ 1.0)
}

pub struct VisualizableNetworkGraph {
    pub graph: UnGraph<DeviceNode, LinkEdge>,
    pub ip_to_node: HashMap<String, NodeIndex>,
}

impl VisualizableNetworkGraph {
    pub fn new() -> Self {
        Self {
            graph: UnGraph::new_undirected(),
            ip_to_node: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, ip: &str, name: &str, role: DeviceRole) -> NodeIndex {
        let idx = self.graph.add_node(DeviceNode {
            ip: ip.to_string(),
            name: name.to_string(),
            role,
        });
        self.ip_to_node.insert(ip.to_string(), idx);
        idx
    }

    pub fn add_link(&mut self, ip_a: &str, ip_b: &str, bandwidth_gbps: f64, load: f64) -> Result<()> {
        let idx_a = *self.ip_to_node.get(ip_a).ok_or_else(|| anyhow!("Node not found: {}", ip_a))?;
        let idx_b = *self.ip_to_node.get(ip_b).ok_or_else(|| anyhow!("Node not found: {}", ip_b))?;

        self.graph.add_edge(idx_a, idx_b, LinkEdge { bandwidth_gbps, traffic_load: load });
        Ok(())
    }

    // -------------------------------------------------------------------------
    // DOT 言語によるカスタマイズ可視化エクスポート
    // -------------------------------------------------------------------------

    /// Graphviz形式のDOT文字列を生成（属性や装飾をカスタマイズ）
    pub fn export_dot(&self) -> String {
        // petgraph の標準 Dot 出力にカスタム描画設定を適用
        let raw_dot = format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[Config::NodeNoLabel, Config::EdgeNoLabel],
                &|_g, edge_ref| {
                    let weight = edge_ref.weight();
                    let load_pct = weight.traffic_load * 100.0;
                    
                    // トラフィック負荷率に応じて線の色と太さを強調
                    if weight.traffic_load >= 0.85 {
                        format!("label=\" {:.0}% ({:.0}G)\" color=\"red\" penwidth=3.0 fontcolor=\"red\"", load_pct, weight.bandwidth_gbps)
                    } else if weight.traffic_load >= 0.60 {
                        format!("label=\" {:.0}% ({:.0}G)\" color=\"orange\" penwidth=2.0 fontcolor=\"orange\"", load_pct, weight.bandwidth_gbps)
                    } else {
                        format!("label=\" {:.0}% ({:.0}G)\" color=\"#4A5568\" penwidth=1.0", load_pct, weight.bandwidth_gbps)
                    }
                },
                &|_g, node_ref| {
                    let node = node_ref.1;
                    // デバイス種別ごとに形状とカラーリングを設定
                    match node.role {
                        DeviceRole::CoreRouter => format!(
                            "label=\"{} \\n({})\" shape=diamond style=filled fillcolor=\"#FCA5A5\" color=\"#DC2626\"",
                            node.name, node.ip
                        ),
                        DeviceRole::Switch => format!(
                            "label=\"{} \\n({})\" shape=box style=filled fillcolor=\"#93C5FD\" color=\"#2563EB\"",
                            node.name, node.ip
                        ),
                        DeviceRole::Server => format!(
                            "label=\"{} \\n({})\" shape=ellipse style=filled fillcolor=\"#86EFAC\" color=\"#16A34A\"",
                            node.name, node.ip
                        ),
                    }
                }
            )
        );

        // レイアウトを美しく見せる全般設定を挿入
        let global_styles = r#"
    graph [overlap=false, splines=true, nodesep=0.8, ranksep=1.0];
    node [fontname="Helvetica", fontsize=10];
    edge [fontname="Helvetica", fontsize=9];
"#;

        raw_dot.replacen('{', &format!("{{{}", global_styles), 1)
    }

    /// DOTファイルを保存し、`dot` コマンドで画像化（PNG / SVG）
    pub fn render_to_file(&self, dot_filename: &str, output_image_filename: &str) -> Result<()> {
        let dot_data = self.export_dot();

        // 1. .dot ファイルの書き出し
        let mut file = File::create(dot_filename)?;
        file.write_all(dot_data.as_bytes())?;
        println!("DOTファイルを保存しました: {}", dot_filename);

        // 2. Graphviz (dot コマンド) を呼び出して PNG/SVG にレンダリング
        let output = Command::new("dot")
            .arg("-Tpng")
            .arg(dot_filename)
            .arg("-o")
            .arg(output_image_filename)
            .output();

        match output {
            Ok(res) if res.status.success() => {
                println!("画像レンダリング成功: {}", output_image_filename);
            }
            Ok(res) => {
                let err_msg = String::from_utf8_lossy(&res.stderr);
                eprintln!("Graphviz エラー: {}", err_msg);
            }
            Err(_) => {
                println!("\n[注意] 'dot' コマンドが見つかりません。");
                println!("生成された '{}' は Graphviz または Online Graphviz Visualizer 等で画像化可能です。", dot_filename);
            }
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// メインルーチン
// -----------------------------------------------------------------------------

fn main() -> Result<()> {
    let mut network = VisualizableNetworkGraph::new();

    // デバイス追加
    network.add_device("10.0.0.1", "Core-Router", DeviceRole::CoreRouter);
    network.add_device("10.0.0.2", "Dist-Switch-A", DeviceRole::Switch);
    network.add_device("10.0.0.3", "Dist-Switch-B", DeviceRole::Switch);
    network.add_device("10.0.0.10", "Web-Server-01", DeviceRole::Server);
    network.add_device("10.0.0.11", "Web-Server-02", DeviceRole::Server);
    network.add_device("10.0.0.20", "DB-Server-Primary", DeviceRole::Server);

    // リンクおよび最新の負荷率を設定
    network.add_link("10.0.0.1", "10.0.0.2", 10.0, 0.90)?; // 赤色表示（高負荷）
    network.add_link("10.0.0.1", "10.0.0.3", 10.0, 0.20)?;
    network.add_link("10.0.0.2", "10.0.0.10", 1.0, 0.70)?; // オレンジ色表示
    network.add_link("10.0.0.2", "10.0.0.11", 1.0, 0.15)?;
    network.add_link("10.0.0.3", "10.0.0.20", 1.0, 0.40)?;
    network.add_link("10.0.0.2", "10.0.0.3", 10.0, 0.05)?;

    // レンダリングの実行
    network.render_to_file("network_topology.dot", "network_topology.png")?;

    Ok(())
}