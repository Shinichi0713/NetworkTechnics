use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use snmp::{SyncSession, Value};
use std::net::SocketAddr;
use std::time::Duration;

/// SNMP Network Monitor & Management Tool in Rust
#[derive(Parser)]
#[command(name = "snmp-monitor")]
#[command(about = "SNMPによりネットワークの疎通確認および機器管理を行うCLIツール", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// ターゲット機器へSNMP GETを発行し疎通・ステータスを確認
    Ping {
        /// 対象機器のIPアドレス (例: 192.168.1.1)
        #[arg(short, long)]
        target: String,

        /// コミュニティ名 (デフォルト: public)
        #[arg(short, long, default_value = "public")]
        community: String,

        /// ポート番号 (デフォルト: 161)
        #[arg(short, long, default_value_t = 161)]
        port: u16,

        /// タイムアウト時間（秒）
        #[arg(short, long, default_value_t = 2)]
        timeout: u64,
    },
    /// 指定したOIDの値を設定 (SNMP SET)
    Set {
        /// 対象機器のIPアドレス
        #[arg(short, long)]
        target: String,

        /// コミュニティ名 (書き込み権限のあるもの, デフォルト: private)
        #[arg(short, long, default_value = "private")]
        community: String,

        /// 設定対象の OID (例: 1.3.6.1.2.1.1.5.0)
        #[arg(short, long)]
        oid: String,

        /// 設定する文字列（OctetString）
        #[arg(short, long)]
        value: String,

        /// ポート番号
        #[arg(short, long, default_value_t = 161)]
        port: u16,
    },
}

// よく使われる標準MIBのOID
const OID_SYS_DESCR: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 1, 0];
const OID_SYS_UPTIME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 3, 0];
const OID_SYS_NAME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 5, 0];

fn parse_oid(oid_str: &str) -> Result<Vec<u32>> {
    oid_str
        .trim_start_matches('.')
        .split('.')
        .map(|s| s.parse::<u32>().map_err(|e| anyhow!("無効なOIDです: {}", e)))
        .collect()
}

/// SNMP GETによる疎通・システム情報取得
fn check_snmp_reachability(
    target_ip: &str,
    port: u16,
    community: &str,
    timeout_secs: u64,
) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", target_ip, port)
        .parse()
        .map_err(|e| anyhow!("IPアドレスのパースに失敗しました: {}", e))?;

    let timeout = Duration::from_secs(timeout_secs);
    let mut session = SyncSession::new(addr, community.as_bytes(), Some(timeout), 0)?;

    println!("[-] Target [{} ({})] へ接続中...", target_ip, community);

    // sysName, sysUptime, sysDescr を一括でGET要求
    let oids = &[OID_SYS_NAME, OID_SYS_UPTIME, OID_SYS_DESCR];
    let mut response = session.getnext(oids)?;

    println!("[+] 疎通確認成功! 応答を受信しました。\n");

    while let Some((name, val)) = response.next() {
        match val {
            Value::OctetString(bytes) => {
                let text = String::from_utf8_lossy(bytes);
                println!("- OID {:?}: {}", name, text);
            }
            Value::Timeticks(ticks) => {
                let seconds = ticks / 100;
                let hours = seconds / 3600;
                let mins = (seconds % 3600) / 60;
                let secs = seconds % 60;
                println!(
                    "- Uptime (OID {:?}): {} 稼働時間 ({}時間{}分{}秒)",
                    name, ticks, hours, mins, secs
                );
            }
            Value::Integer(num) => {
                println!("- OID {:?}: Integer({})", name, num);
            }
            other => {
                println!("- OID {:?}: {:?}", name, other);
            }
        }
    }

    Ok(())
}

/// SNMP SETによるパラメータ書き込み処理
fn set_snmp_value(
    target_ip: &str,
    port: u16,
    community: &str,
    oid_str: &str,
    val_str: &str,
) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", target_ip, port).parse()?;
    let timeout = Duration::from_secs(3);
    let mut session = SyncSession::new(addr, community.as_bytes(), Some(timeout), 0)?;

    let oid = parse_oid(oid_str)?;
    let value = Value::OctetString(val_str.as_bytes());

    println!("[-] OID {} に対し SET 要求を送信中...", oid_str);
    let response = session.set(&[(&oid, value)])?;

    println!("[+] SET 成功!");
    for (name, val) in response {
        println!("- 変更後の OID {:?}: {:?}", name, val);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ping {
            target,
            community,
            port,
            timeout,
        } => {
            if let Err(e) = check_snmp_reachability(&target, port, &community, timeout) {
                eprintln!("[!] 疎通確認エラー: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Set {
            target,
            community,
            oid,
            value,
            port,
        } => {
            if let Err(e) = set_snmp_value(&target, port, &community, &oid, &value) {
                eprintln!("[!] SET失敗: {}", e);
                std::process::exit(1);
            }
        }
    }
}


use anyhow::{anyhow, Result};
use chrono::Local;
use clap::Parser;
use snmp::{SyncSession, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// SNMP による端末の死活監視およびプロセス監視ツール
#[derive(Parser, Debug)]
#[command(author, version, about = "SNMP Live & Process Monitor", long_about = None)]
struct Config {
    /// 監視対象端末の IP アドレス
    #[arg(short, long, default_value = "127.0.0.1")]
    target: String,

    /// SNMP コミュニティ名
    #[arg(short, long, default_value = "public")]
    community: String,

    /// SNMP ポート番号
    #[arg(short, long, default_value_t = 161)]
    port: u16,

    /// 監視インターバル（秒）
    #[arg(short, long, default_value_t = 10)]
    interval: u64,

    /// 監視対象プロセスのフィルター名（空の場合は全プロセス検出数をカウント）
    #[arg(short, long)]
    process_name: Option<String>,
}

/// 検出されたプロセスのステータス情報
#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    status: u32,
    cpu_perf: u32,
    mem_kb: u32,
}

/// 監視対象端末のステータス
#[derive(Debug)]
struct NodeStatus {
    is_alive: bool,
    response_time_ms: u64,
    sys_uptime: Option<u32>,
    processes: Vec<ProcessInfo>,
}

// OID の定義
const OID_SYS_UPTIME: &[u32] = &[1, 3, 6, 1, 2, 1, 1, 3, 0];
const OID_HR_SW_RUN_NAME: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 2];
const OID_HR_SW_RUN_STATUS: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 4, 2, 1, 7];
const OID_HR_SW_RUN_PERF_CPU: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 5, 1, 1, 1];
const OID_HR_SW_RUN_PERF_MEM: &[u32] = &[1, 3, 6, 1, 2, 1, 25, 5, 1, 1, 2];

struct SnmpMonitor {
    target: String,
    port: u16,
    community: String,
    timeout: Duration,
}

impl SnmpMonitor {
    fn new(target: String, port: u16, community: String, timeout_secs: u64) -> Self {
        Self {
            target,
            port,
            community,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// 死活監視およびプロセス監視の実行
    fn poll(&self) -> NodeStatus {
        let start_time = Instant::now();
        let addr: SocketAddr = match format!("{}:{}", self.target, self.port).parse() {
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

        let mut session = match SyncSession::new(addr, self.community.as_bytes(), Some(self.timeout), 0) {
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

        // 1. 死活チェック (sysUptime の取得)
        let response = session.get(OID_SYS_UPTIME);
        let elapsed = start_time.elapsed().as_millis() as u64;

        let sys_uptime = match response {
            Ok(mut res) => {
                if let Some((_, Value::Timeticks(ticks))) = res.next() {
                    Some(ticks)
                } else {
                    None
                }
            }
            Err(_) => {
                // sysUptime が取得できなければ Down と判定
                return NodeStatus {
                    is_alive: false,
                    response_time_ms: elapsed,
                    sys_uptime: None,
                    processes: vec![],
                };
            }
        };

        // 2. プロセス一覧情報の収集 (Walk 処理)
        let processes = self.collect_processes(&mut session).unwrap_or_default();

        NodeStatus {
            is_alive: true,
            response_time_ms: elapsed,
            sys_uptime,
            processes,
        }
    }

    /// OIDツリーをウォークしてプロセス一覧を取得
    fn collect_processes(&self, session: &mut SyncSession) -> Result<Vec<ProcessInfo>> {
        let mut process_map: HashMap<u32, ProcessInfo> = HashMap::new();

        // 2a. プロセス名 (hrSWRunName) のウォーク
        let mut walk = session.walkiter(OID_HR_SW_RUN_NAME)?;
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
                            cpu_perf: 0,
                            mem_kb: 0,
                        },
                    );
                }
            }
        }

        // 2b. プロセスステータス (hrSWRunStatus) のウォーク
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

        // 2c. プロセス メモリ使用量 (hrSWRunPerfMem) のウォーク
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

        Ok(process_map.into_values().collect())
    }
}

#[tokio::main]
async fn main() {
    let config = Config::parse();
    println!("=== SNMP 監視サービスを開始します ===");
    println!("ターゲット: {}:{}", config.target, config.port);
    println!("監視周期: {} 秒", config.interval);
    if let Some(ref target_proc) = config.process_name {
        println!("監視対象プロセス名: {}", target_proc);
    }
    println!("--------------------------------------------------");

    let monitor = SnmpMonitor::new(
        config.target.clone(),
        config.port,
        config.community.clone(),
        3, // タイムアウト 3 秒
    );

    let mut interval_timer = tokio::time::interval(Duration::from_secs(config.interval));

    loop {
        interval_timer.tick().await;

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let status = monitor.poll();

        if !status.is_alive {
            println!(
                "[{}] [CRITICAL] 機器 {}: 応答がありません (DOWN)",
                now, config.target
            );
            continue;
        }

        let uptime_secs = status.sys_uptime.unwrap_or(0) / 100;
        println!(
            "[{}] [OK] 死活: UP | 応答時間: {}ms | Uptime: {}s | 総プロセス数: {}",
            now,
            status.response_time_ms,
            uptime_secs,
            status.processes.len()
        );

        // 指定プロセスまたは全体の動作状況を出力
        if let Some(ref target_proc) = config.process_name {
            let matched_procs: Vec<&ProcessInfo> = status
                .processes
                .iter()
                .filter(|p| p.name.contains(target_proc))
                .collect();

            if matched_procs.is_empty() {
                println!(
                    "  └─ [ALERT] プロセス '{}' が検出されませんでした (停止中)",
                    target_proc
                );
            } else {
                for proc in matched_procs {
                    let status_str = match proc.status {
                        1 => "Running",
                        2 => "Runnable",
                        3 => "NotRunnable",
                        4 => "Invalid",
                        _ => "Unknown",
                    };
                    println!(
                        "  └─ [PROCESS] PID: {:<6} | Name: {:<20} | Status: {:<10} | Mem: {} KB",
                        proc.pid, proc.name, status_str, proc.mem_kb
                    );
                }
            }
        }
    }
}