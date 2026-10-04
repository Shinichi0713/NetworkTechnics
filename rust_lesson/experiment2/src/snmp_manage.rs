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