use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use snmp::{SyncSession, Value};
use sqlx::sqlite::SqlitePool;
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

// -----------------------------------------------------------------------------
// CLI 定義
// -----------------------------------------------------------------------------
#[derive(Parser)]
#[command(name = "snmp-db-monitor")]
#[command(about = "SNMP 監視およびデータ蓄積・参照ツール", long_about = None)]
struct Cli {
    /// データベースファイルパス
    #[arg(short, long, default_value = "monitor.db")]
    db_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 監視ループを実行し、取得データをDBに蓄積
    Monitor {
        /// 監視対象 IP
        #[arg(short, long, default_value = "127.0.0.1")]
        target: String,

        /// コミュニティ名
        #[arg(short, long, default_value = "public")]
        community: String,

        /// ポート番号
        #[arg(short, long, default_value_t = 161)]
        port: u16,

        /// 監視周期（秒）
        #[arg(short, long, default_value_t = 10)]
        interval: u64,
    },
    /// DBから監視履歴を参照・検索
    Query {
        #[command(subcommand)]
        sub: QuerySubcommands,
    },
}

#[derive(Subcommand)]
enum QuerySubcommands {
    /// 死活監視ログの取得
    Nodes {
        /// 特定IPで絞り込み
        #[arg(short, long)]
        target: Option<String>,

        /// 直近の表示件数
        #[arg(short, long, default_value_t = 10)]
        limit: i64,
    },
    /// プロセス動作履歴の取得
    Processes {
        /// プロセス名で部分一致検索 (例: nginx)
        #[arg(short, long)]
        name: Option<String>,

        /// 直近の表示件数
        #[arg(short, long, default_value_t = 20)]
        limit: i64,
    },
}

// -----------------------------------------------------------------------------
// データ構造体 & OID
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    status: u32,
    mem_kb: u32,
}

struct NodeStatus {
    is_alive: bool,
    response_time_ms: i64,
    sys_uptime: Option<i64>,
    processes: Vec<ProcessInfo>,
}

#[derive(FromRow, Debug)]
struct NodeLogRecord {
    id: i64,
    target_ip: String,
    is_alive: bool,
    response_time_ms: i64,
    sys_uptime: Option<i64>,
    created_at: String,
}

#[derive(FromRow, Debug)]
struct ProcessLogRecord {
    id: i64,
    node_log_id: i64,
    pid: i64,
    process_name: String,
    status: i64,
    mem_kb: i64,
    created_at: String,
}

const OID_SYS_UPTIME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 3, 0];
const OID_HR_SW_RUN_NAME: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 2];
const OID_HR_SW_RUN_STATUS: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 7];
const OID_HR_SW_RUN_PERF_MEM: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 5, 1, 1, 2];

// -----------------------------------------------------------------------------
// DB 操作モジュール
// -----------------------------------------------------------------------------
struct Database {
    pool: SqlitePool,
}

impl Database {
    async fn new(db_path: &str) -> Result<Self> {
        let connection_str = format!("sqlite:{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&connection_str).await?;

        // テーブル初期化
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS node_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_ip TEXT NOT NULL,
                is_alive BOOLEAN NOT NULL,
                response_time_ms INTEGER NOT NULL,
                sys_uptime INTEGER,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS process_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_log_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                process_name TEXT NOT NULL,
                status INTEGER NOT NULL,
                mem_kb INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(node_log_id) REFERENCES node_logs(id)
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// 取得した監視情報をトランザクションで書き込み
    async fn insert_status(
        &self,
        target_ip: &str,
        status: &NodeStatus,
        now: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // 1. node_logs に挿入し、生成された ID を取得
        let node_log_id = sqlx::query(
            r#"
            INSERT INTO node_logs (target_ip, is_alive, response_time_ms, sys_uptime, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(target_ip)
        .bind(status.is_alive)
        .bind(status.response_time_ms)
        .bind(status.sys_uptime)
        .bind(now)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        // 2. process_logs に全プロセス情報を一括挿入
        for p in &status.processes {
            sqlx::query(
                r#"
                INSERT INTO process_logs (node_log_id, pid, process_name, status, mem_kb, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(node_log_id)
            .bind(p.pid as i64)
            .bind(&p.name)
            .bind(p.status as i64)
            .bind(p.mem_kb as i64)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// 死活履歴の照会
    async fn fetch_node_logs(&self, target: Option<String>, limit: i64) -> Result<Vec<NodeLogRecord>> {
        let query = if let Some(t) = target {
            sqlx::query_as::<_, NodeLogRecord>(
                "SELECT * FROM node_logs WHERE target_ip = ? ORDER BY id DESC LIMIT ?",
            )
            .bind(t)
            .bind(limit)
        } else {
            sqlx::query_as::<_, NodeLogRecord>(
                "SELECT * FROM node_logs ORDER BY id DESC LIMIT ?",
            )
            .bind(limit)
        };

        Ok(query.fetch_all(&self.pool).await?)
    }

    /// プロセス履歴の照会
    async fn fetch_process_logs(&self, name: Option<String>, limit: i64) -> Result<Vec<ProcessLogRecord>> {
        let query = if let Some(n) = name {
            sqlx::query_as::<_, ProcessLogRecord>(
                "SELECT * FROM process_logs WHERE process_name LIKE ? ORDER BY id DESC LIMIT ?",
            )
            .bind(format!("%{}%", n))
            .bind(limit)
        } else {
            sqlx::query_as::<_, ProcessLogRecord>(
                "SELECT * FROM process_logs ORDER BY id DESC LIMIT ?",
            )
            .bind(limit)
        };

        Ok(query.fetch_all(&self.pool).await?)
    }
}

// -----------------------------------------------------------------------------
// SNMP ポーリング処理
// -----------------------------------------------------------------------------
fn poll_snmp(target: &str, port: u16, community: &str, timeout_secs: u64) -> NodeStatus {
    let start_time = Instant::now();
    let addr: SocketAddr = match format!("{}:{}", target, port).parse() {
        Ok(a) => a,
        Err(_) => {
            return NodeStatus {
                is_alive: false,
                response_time_ms: 0,
                sys_uptime: None,
                processes: vec![],
            }
        }
    };

    let mut session = match SyncSession::new(addr, community.as_bytes(), Some(Duration::from_secs(timeout_secs)), 0) {
        Ok(s) => s,
        Err(_) => {
            return NodeStatus {
                is_alive: false,
                response_time_ms: 0,
                sys_uptime: None,
                processes: vec![],
            }
        }
    };

    // 1. sysUptime
    let response = session.get(OID_SYS_UPTIME);
    let elapsed = start_time.elapsed().as_millis() as i64;

    let sys_uptime = match response {
        Ok(mut res) => {
            if let Some((_, Value::Timeticks(ticks))) = res.next() {
                Some(ticks as i64)
            } else {
                None
            }
        }
        Err(_) => {
            return NodeStatus {
                is_alive: false,
                response_time_ms: elapsed,
                sys_uptime: None,
                processes: vec![],
            }
        }
    };

    // 2. プロセス情報収集 (Walk)
    let mut process_map: HashMap<u32, ProcessInfo> = HashMap::new();

    if let Ok(mut walk) = session.walkiter(OID_HR_SW_RUN_NAME) {
        while let Some(Ok((oid, value))) = walk.next() {
            if let Some(pid) = oid.to_vec().last().copied() {
                if let Value::OctetString(bytes) = value {
                    let name = String::from_utf8_lossy(bytes).to_string();
                    process_map.insert(
                        pid,
                        ProcessInfo {
                            pid,
                            name,
                            status: 0,
                            mem_kb: 0,
                        },
                    );
                }
            }
        }
    }

    if let Ok(mut walk_status) = session.walkiter(OID_HR_SW_RUN_STATUS) {
        while let Some(Ok((oid, value))) = walk_status.next() {
            if let Some(pid) = oid.to_vec().last().copied() {
                if let Some(info) = process_map.get_mut(&pid) {
                    if let Value::Integer(status) = value {
                        info.status = status as u32;
                    }
                }
            }
        }
    }

    if let Ok(mut walk_mem) = session.walkiter(OID_HR_SW_RUN_PERF_MEM) {
        while let Some(Ok((oid, value))) = walk_mem.next() {
            if let Some(pid) = oid.to_vec().last().copied() {
                if let Some(info) = process_map.get_mut(&pid) {
                    if let Value::Integer(mem) = value {
                        info.mem_kb = mem as u32;
                    }
                }
            }
        }
    }

    NodeStatus {
        is_alive: true,
        response_time_ms: elapsed,
        sys_uptime,
        processes: process_map.into_values().collect(),
    }
}

// -----------------------------------------------------------------------------
// メイン関数
// -----------------------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::new(&cli.db_path).await?;

    match cli.command {
        Commands::Monitor {
            target,
            community,
            port,
            interval,
        } => {
            println!("=== SNMP DB 監視サービス開始 ===");
            println!("ターゲット: {}:{} | 記録先DB: {}", target, port, cli.db_path);

            let mut timer = tokio::time::interval(Duration::from_secs(interval));

            loop {
                timer.tick().await;
                let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                let status = poll_snmp(&target, port, &community, 3);
                let alive_str = if status.is_alive { "UP" } else { "DOWN" };

                println!(
                    "[{}] Target: {} | Alive: {} | RTT: {}ms | ProcCount: {}",
                    now,
                    target,
                    alive_str,
                    status.response_time_ms,
                    status.processes.len()
                );

                // DBに状態および全プロセス情報を記録
                if let Err(e) = db.insert_status(&target, &status, &now).await {
                    eprintln!("[!] DB書き込みエラー: {}", e);
                }
            }
        }
        Commands::Query { sub } => match sub {
            QuerySubcommands::Nodes { target, limit } => {
                let records = db.fetch_node_logs(target, limit).await?;
                println!("--- ノード死活履歴 (直近 {} 件) ---", limit);
                println!("{:<5} | {:<15} | {:<6} | {:<8} | {:<20}", "ID", "IP", "Status", "RTT", "Timestamp");
                println!("{:-<65}", "");
                for r in records {
                    let status_str = if r.is_alive { "UP" } else { "DOWN" };
                    println!(
                        "{:<5} | {:<15} | {:<6} | {:<5}ms  | {:<20}",
                        r.id, r.target_ip, status_str, r.response_time_ms, r.created_at
                    );
                }
            }
            QuerySubcommands::Processes { name, limit } => {
                let records = db.fetch_process_logs(name, limit).await?;
                println!("--- プロセス動態ログ (直近 {} 件) ---", limit);
                println!(
                    "{:<5} | {:<6} | {:<22} | {:<10} | {:<10} | {:<20}",
                    "ID", "PID", "Process Name", "Status", "Mem(KB)", "Timestamp"
                );
                println!("{:-<85}", "");
                for r in records {
                    let status_str = match r.status {
                        1 => "Running",
                        2 => "Runnable",
                        3 => "NotRunnable",
                        4 => "Invalid",
                        _ => "Unknown",
                    };
                    println!(
                        "{:<5} | {:<6} | {:<22} | {:<10} | {:<10} | {:<20}",
                        r.id, r.pid, r.process_name, status_str, r.mem_kb, r.created_at
                    );
                }
            }
        },
    }

    Ok(())
}