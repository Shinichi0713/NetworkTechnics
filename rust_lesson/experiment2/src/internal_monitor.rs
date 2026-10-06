use anyhow::Result;
use chrono::Local;
use clap::Parser;
use snmp::{SyncSession, Value};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

// -----------------------------------------------------------------------------
// 設定 & アラート閾値（Threshold）定義
// -----------------------------------------------------------------------------
#[derive(Parser, Debug)]
#[command(author, version, about = "SNMP アプリケーション不調・稼働状況定期監視ツール")]
struct Config {
    /// 監視対象 IP
    #[arg(short, long, default_value = "127.0.0.1")]
    target: String,

    /// コミュニティ名
    #[arg(short, long, default_value = "public")]
    community: String,

    /// 監視インターバル（秒）
    #[arg(short, long, default_value_t = 10)]
    interval: u64,

    /// 監視対象アプリケーションのプロセス名 (例: nginx, mysqld, app_server)
    #[arg(short, long, default_value = "nginx")]
    app_name: String,

    /// 最小期待プロセス数 (これ未満ならアプリケーション停止・不調と判定)
    #[arg(long, default_value_t = 1)]
    min_processes: usize,

    /// アプリケーション全体のメモリ上限閾値 (KB) (例: 500MB = 512000 KB)
    #[arg(long, default_value_t = 512000)]
    max_mem_kb: u32,

    /// 応答遅延警告の閾値 (ms)
    #[arg(long, default_value_t = 500)]
    max_rtt_ms: u64,

    /// データベースファイルパス
    #[arg(short, long, default_value = "app_monitor.db")]
    db_path: String,
}

// アプリケーションの健全性ステータス
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AppHealthStatus {
    Healthy,       // 正常稼働
    Degraded,      // 端末は動いているがアプリに不調あり (リソース高騰/一部プロセスダウン)
    AppDown,       // 端末は動いているがアプリが完全に停止
    NodeUnreachable, // 端末自体が応答なし
}

impl std::fmt::Display for AppHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppHealthStatus::Healthy => write!(f, "HEALTHY"),
            AppHealthStatus::Degraded => write!(f, "DEGRADED (不調検知)"),
            AppHealthStatus::AppDown => write!(f, "APP_DOWN (アプリ停止)"),
            AppHealthStatus::NodeUnreachable => write!(f, "NODE_UNREACHABLE"),
        }
    }
}

// -----------------------------------------------------------------------------
// データモデル
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct ProcessMetric {
    pid: u32,
    name: String,
    status: u32,
    cpu_perf: u32,
    mem_kb: u32,
}

#[derive(Debug)]
struct MonitorResult {
    is_node_alive: bool,
    rtt_ms: u64,
    sys_uptime: Option<u32>,
    app_processes: Vec<ProcessMetric>,
    total_app_mem_kb: u32,
    health_status: AppHealthStatus,
    issues: Vec<String>,
}

// OID 定義
const OID_SYS_UPTIME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 3, 0];
const OID_HR_SW_RUN_NAME: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 2];
const OID_HR_SW_RUN_STATUS: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 7];
const OID_HR_SW_RUN_PERF_CPU: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 5, 1, 1, 1];
const OID_HR_SW_RUN_PERF_MEM: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 5, 1, 1, 2];

// -----------------------------------------------------------------------------
// DB 処理
// -----------------------------------------------------------------------------
struct AppDatabase {
    pool: SqlitePool,
}

impl AppDatabase {
    async fn new(db_path: &str) -> Result<Self> {
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_health_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_ip TEXT NOT NULL,
                app_name TEXT NOT NULL,
                node_alive BOOLEAN NOT NULL,
                health_status TEXT NOT NULL,
                rtt_ms INTEGER NOT NULL,
                process_count INTEGER NOT NULL,
                total_mem_kb INTEGER NOT NULL,
                issues TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    async fn log_health(&self, target_ip: &str, app_name: &str, res: &MonitorResult, now: &str) -> Result<()> {
        let issues_str = res.issues.join("; ");
        sqlx::query(
            r#"
            INSERT INTO app_health_logs 
            (target_ip, app_name, node_alive, health_status, rtt_ms, process_count, total_mem_kb, issues, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(target_ip)
        .bind(app_name)
        .bind(res.is_node_alive)
        .bind(res.health_status.to_string())
        .bind(res.rtt_ms as i64)
        .bind(res.app_processes.len() as i64)
        .bind(res.total_app_mem_kb as i64)
        .bind(issues_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// 監視 & 判定ロジック
// -----------------------------------------------------------------------------
fn collect_and_evaluate(cfg: &Config) -> MonitorResult {
    let start = Instant::now();
    let addr: SocketAddr = match format!("{}:161", cfg.target).parse() {
        Ok(a) => a,
        Err(_) => return build_unreachable_result(0),
    };

    let mut session = match SyncSession::new(addr, cfg.community.as_bytes(), Some(Duration::from_secs(3)), 0) {
        Ok(s) => s,
        Err(_) => return build_unreachable_result(0),
    };

    // 1. ノード応答確認 (sysUptime)
    let res = session.get(OID_SYS_UPTIME);
    let rtt = start.elapsed().as_millis() as u64;

    let sys_uptime = match res {
        Ok(mut r) => match r.next() {
            Some((_, Value::Timeticks(t))) => Some(t),
            _ => None,
        },
        Err(_) => return build_unreachable_result(rtt),
    };

    // 2. 全プロセスの集約
    let mut proc_map: HashMap<u32, ProcessMetric> = HashMap::new();

    if let Ok(mut walk) = session.walkiter(OID_HR_SW_RUN_NAME) {
        while let Some(Ok((oid, value))) = walk.next() {
            if let Some(pid) = oid.to_vec().last().copied() {
                if let Value::OctetString(bytes) = value {
                    let name = String::from_utf8_lossy(bytes).to_string();
                    if name.contains(&cfg.app_name) {
                        proc_map.insert(
                            pid,
                            ProcessMetric {
                                pid,
                                name,
                                status: 0,
                                cpu_perf: 0,
                                mem_kb: 0,
                            },
                        );
                    }
                }
            }
        }
    }

    // ステータス・メモリ・CPU情報のマッピング
    if !proc_map.is_empty() {
        if let Ok(mut walk) = session.walkiter(OID_HR_SW_RUN_STATUS) {
            while let Some(Ok((oid, Value::Integer(st)))) = walk.next() {
                if let Some(pid) = oid.to_vec().last() {
                    if let Some(p) = proc_map.get_mut(pid) { p.status = st as u32; }
                }
            }
        }
        if let Ok(mut walk) = session.walkiter(OID_HR_SW_RUN_PERF_MEM) {
            while let Some(Ok((oid, Value::Integer(mem)))) = walk.next() {
                if let Some(pid) = oid.to_vec().last() {
                    if let Some(p) = proc_map.get_mut(pid) { p.mem_kb = mem as u32; }
                }
            }
        }
        if let Ok(mut walk) = session.walkiter(OID_HR_SW_RUN_PERF_CPU) {
            while let Some(Ok((oid, Value::Integer(cpu)))) = walk.next() {
                if let Some(pid) = oid.to_vec().last() {
                    if let Some(p) = proc_map.get_mut(pid) { p.cpu_perf = cpu as u32; }
                }
            }
        }
    }

    let app_processes: Vec<ProcessMetric> = proc_map.into_values().collect();
    let total_app_mem_kb: u32 = app_processes.iter().map(|p| p.mem_kb).sum();

    // 3. アプリケーション不調ルールエンジン（評価ロジック）
    let mut issues = Vec::new();
    let mut status = AppHealthStatus::Healthy;

    // 判定A: 端末応答遅延
    if rtt > cfg.max_rtt_ms {
        issues.push(format!("ネットワークレスポンス遅延 (RTT: {}ms > 閾値: {}ms)", rtt, cfg.max_rtt_ms));
        status = AppHealthStatus::Degraded;
    }

    // 判定B: アプリケーションプロセスの存在チェック
    if app_processes.len() < cfg.min_processes {
        if app_processes.is_empty() {
            issues.push(format!("アプリ '{}' のプロセスが全く存在しません (停止状態)", cfg.app_name));
            status = AppHealthStatus::AppDown;
        } else {
            issues.push(format!("プロセス数が不足しています (検出: {} < 期待値: {})", app_processes.len(), cfg.min_processes));
            status = AppHealthStatus::Degraded;
        }
    }

    // 判定C: 異常状態(NotRunnable等)のプロセスチェック
    let abnormal_count = app_processes.iter().filter(|p| p.status != 1 && p.status != 2).count();
    if abnormal_count > 0 {
        issues.push(format!("異常状態のプロセスを検出しました (個数: {})", abnormal_count));
        if status != AppHealthStatus::AppDown {
            status = AppHealthStatus::Degraded;
        }
    }

    // 判定D: メモリ過大消費（リークや負荷急増の不調）
    if total_app_mem_kb > cfg.max_mem_kb {
        issues.push(format!("アプリメモリ消費量が閾値を超過 (使用量: {} KB > 限界: {} KB)", total_app_mem_kb, cfg.max_mem_kb));
        if status != AppHealthStatus::AppDown {
            status = AppHealthStatus::Degraded;
        }
    }

    MonitorResult {
        is_node_alive: true,
        rtt_ms: rtt,
        sys_uptime,
        app_processes,
        total_app_mem_kb,
        health_status: status,
        issues,
    }
}

fn build_unreachable_result(rtt: u64) -> MonitorResult {
    MonitorResult {
        is_node_alive: false,
        rtt_ms: rtt,
        sys_uptime: None,
        app_processes: vec![],
        total_app_mem_kb: 0,
        health_status: AppHealthStatus::NodeUnreachable,
        issues: vec!["端末(OS/SNMP)からの応答がありません".to_string()],
    }
}

// -----------------------------------------------------------------------------
// メインルーチン
// -----------------------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::parse();
    let db = AppDatabase::new(&cfg.db_path).await?;

    println!("=== アプリケーション不調・稼働状況モニタリング開始 ===");
    println!("対象端末: {} | 対象アプリ: '{}'", cfg.target, cfg.app_name);
    println!("閾値設定: 最小プロセス数={}, メモリ上限={}KB, 許容RTT={}ms", cfg.min_processes, cfg.max_mem_kb, cfg.max_rtt_ms);
    println!("--------------------------------------------------");

    let mut timer = tokio::time::interval(Duration::from_secs(cfg.interval));
    let mut prev_status: Option<AppHealthStatus> = None;

    loop {
        timer.tick().await;
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let res = collect_and_evaluate(&cfg);

        // 状態変化（健全 -> 不調 / 不調 -> 健全）の通知ハンドリング
        if let Some(prev) = prev_status {
            if prev != res.health_status {
                println!("\n>>> [ステータス変更検知] {} -> {} <<<", prev, res.health_status);
            }
        }
        prev_status = Some(res.health_status);

        // コンソール出力
        match res.health_status {
            AppHealthStatus::Healthy => {
                println!(
                    "[{}] [OK] 状態: HEALTHY | 端末: 正常(RTT {}ms) | アプリ '{}': 正常稼働 (個数: {}, メモリ: {} KB)",
                    now, res.rtt_ms, cfg.app_name, res.app_processes.len(), res.total_app_mem_kb
                );
            }
            AppHealthStatus::Degraded => {
                println!(
                    "[{}] [ALERT-不調] 状態: DEGRADED | 端末: 動作中(RTT {}ms) | 原因: {:?}",
                    now, res.rtt_ms, res.issues
                );
            }
            AppHealthStatus::AppDown => {
                println!(
                    "[{}] [CRITICAL-アプリ停止] 状態: APP_DOWN | 端末: 動作中(RTT {}ms) | アプリ '{}' が停止しています",
                    now, res.rtt_ms, cfg.app_name
                );
            }
            AppHealthStatus::NodeUnreachable => {
                println!("[{}] [FATAL] 状態: UNREACHABLE | 端末自体がダウンしています", now);
            }
        }

        // DBに状態および判定理由（issues）を保存
        if let Err(e) = db.log_health(&cfg.target, &cfg.app_name, &res, &now).await {
            eprintln!("[!] DB記録エラー: {}", e);
        }
    }
}