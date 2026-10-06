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